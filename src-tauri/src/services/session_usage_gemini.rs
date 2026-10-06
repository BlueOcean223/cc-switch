//! Gemini CLI 会话日志使用追踪
//!
//! 从 `~/.gemini/tmp/<项目目录>/chats/` 下的会话文件提取每次 API 调用的 token 用量：
//! - `session-*.jsonl`：2026-04-09 起的追加写格式（gemini-cli PR #23749），读法见
//!   [`crate::session_manager::providers::gemini::jsonl::scan_usage`]
//! - `session-*.json`：之前的整份 JSON
//! - `<主会话 id>/*.jsonl`（2026-03-26 ~ 04-09 为 `.json`）：子代理会话
//!
//! ## 数据流
//! ```text
//! ~/.gemini/tmp/*/chats/{session-*.json(l), <主会话 id>/*.json(l)}
//!   → 按消息 id 收集带 tokens 的 gemini 消息 → 费用计算 → proxy_request_logs 表
//! ```
//!
//! ## 去重
//! - 每份 tokens 只挂在一条 gemini 消息上，这条消息之后的整条重写（补工具调用、补用量、迁移、
//!   checkpoint）都带着同一份，所以按 `(sessionId, 消息 id)` 去重就不会重复计数：
//!   `request_id = gemini_session:{sessionId}:{消息 id}` 加 UPSERT。这个格式不能改，已入库的行
//!   靠它去重。消息 id 只在会话内唯一（初始上下文消息的 id 每个会话都一样），所以必须带 sessionId。
//! - 旧 `.json` 与恢复时迁移出的同名 `.jsonl`、旧 hash 目录与 slug 目录里复制的副本，sessionId
//!   与消息 id 都相同，靠 UPSERT 自然去重。
//! - 子代理的调用只记在子代理文件里，父会话的 tokens 不含它们，两者相加不重复。request_id 用
//!   子代理自己的 sessionId，`session_id` 列记父会话 id（子目录名），与 Codex 子代理一致。
//! - 已知会重复的边界：`gemini --session-file` 导入会生成新 sessionId 并原样复制原消息（id 与
//!   tokens 不变），与原会话重复计费。用得少，暂不处理；以后要处理可以识别回放后第一条
//!   `type: "info"`、id 为 `import-<ms>` 的消息，跳过时间早于它的 gemini 消息。
//!
//! ## 与官方 /stats 的口径差异
//! 被 `$rewindTo` 回退、被 `$patch` 移除或被压缩掉的调用也计入（官方恢复会话后只累计回放后剩下
//! 的消息），因为这些调用实际已经发生、已经计费。
//!
//! ## 增量
//! 文件 mtime 变化就整份重读，UPSERT 补全写到一半的回复；写到一半的末行跳过，下一轮再读。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::gemini_config::get_gemini_dir;
use crate::services::session_usage::{
    metadata_modified_nanos, update_sync_state, SessionSyncResult,
};
use crate::services::sql_helpers::INPUT_TOKEN_SEMANTICS_FRESH;
use crate::services::usage_stats::find_model_pricing;
use crate::session_manager::providers::gemini::jsonl;
use crate::token_usage::calculator::{CostBreakdown, CostCalculator, ServiceTier};
use crate::token_usage::parser::TokenUsage;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Gemini CLI 会话里一条回复的 `tokens`。
#[derive(Debug, Clone, Default)]
struct GeminiTokens {
    /// promptTokenCount：通常含 `cached`
    input: u32,
    output: u32,
    cached: u32,
    thoughts: u32,
    /// toolUsePromptTokenCount，按输入计费
    tool: u32,
    total: Option<u64>,
}

impl GeminiTokens {
    /// 换成互不重叠的几桶。`input` 含缓存时 `total = input + output + thoughts + tool`
    /// （缓存不另计）；只有这样才从 input 里减掉 `cached`，和 ccusage / tokscale 一致。
    fn token_usage(&self) -> TokenUsage {
        let inclusive_total = u64::from(self.input)
            + u64::from(self.output)
            + u64::from(self.thoughts)
            + u64::from(self.tool);
        let input_includes_cached = self.total.is_none_or(|total| total == inclusive_total);
        let fresh_input = if input_includes_cached {
            self.input.saturating_sub(self.cached)
        } else {
            self.input
        };
        TokenUsage {
            input_tokens: fresh_input.saturating_add(self.tool),
            output_tokens: self.output.saturating_add(self.thoughts),
            cache_read_tokens: self.cached,
            cache_creation_tokens: 0,
            cache_creation_1h_tokens: 0,
        }
    }
}

/// 同步 Gemini 使用数据（从会话日志）
pub fn sync_gemini_usage(db: &Database) -> Result<SessionSyncResult, AppError> {
    sync_gemini_usage_in(db, &get_gemini_dir())
}

fn sync_gemini_usage_in(db: &Database, gemini_dir: &Path) -> Result<SessionSyncResult, AppError> {
    let files = collect_gemini_session_files(gemini_dir);

    let mut result = SessionSyncResult {
        imported: 0,
        skipped: 0,
        files_scanned: files.len() as u32,
        deferred_files: 0,
        errors: vec![],
    };

    if files.is_empty() {
        return Ok(result);
    }

    let cursors = crate::services::session_usage::load_sync_cursors(db)?;

    for file in &files {
        let last_modified = cursors
            .get(file.path.to_string_lossy().as_ref())
            .map_or(0, |c| c.last_modified);
        match sync_single_gemini_file(db, file, last_modified) {
            Ok((imported, skipped)) => {
                result.imported += imported;
                result.skipped += skipped;
            }
            Err(e) => {
                let msg = format!("Gemini 会话文件解析失败 {}: {e}", file.path.display());
                log::warn!("[GEMINI-SYNC] {msg}");
                result.errors.push(msg);
            }
        }
    }

    if result.imported > 0 {
        log::info!(
            "[GEMINI-SYNC] 同步完成: 导入 {} 条, 跳过 {} 条, 扫描 {} 个文件",
            result.imported,
            result.skipped,
            result.files_scanned
        );
    }

    Ok(result)
}

/// 要导入的一个会话文件。
#[derive(Debug, Clone, PartialEq, Eq)]
struct GeminiSessionFile {
    path: PathBuf,
    /// 子代理文件所在的子目录名，即主会话的 sessionId（经过官方的 `[^a-zA-Z0-9_-]→_` 清洗）
    parent_session_id: Option<String>,
}

/// 收集所有 Gemini 会话文件：`tmp/*/chats/session-*.json(l)` 是主会话（以及 2026-03-26 之前
/// 混在其中的 `kind: "subagent"` 旧子代理），`tmp/*/chats/<主会话 id>/*.json(l)` 是子代理
/// （固定一层，不递归）。`*.jsonl.tmp-*`、`*.jsonl.unreadable-*` 按后缀排除。
fn collect_gemini_session_files(gemini_dir: &Path) -> Vec<GeminiSessionFile> {
    fn is_session_file(path: &Path) -> bool {
        path.extension()
            .is_some_and(|ext| ext == "json" || ext == "jsonl")
            && path.is_file()
    }

    let mut files = Vec::new();
    let Ok(project_dirs) = fs::read_dir(gemini_dir.join("tmp")) else {
        return files;
    };
    for project in project_dirs.flatten() {
        let Ok(entries) = fs::read_dir(project.path().join("chats")) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                let Ok(agents) = fs::read_dir(&path) else {
                    continue;
                };
                files.extend(
                    agents
                        .flatten()
                        .map(|agent| agent.path())
                        .filter(|agent| is_session_file(agent))
                        .map(|agent| GeminiSessionFile {
                            path: agent,
                            parent_session_id: Some(name.clone()),
                        }),
                );
            } else if name.starts_with("session-") && is_session_file(&path) {
                files.push(GeminiSessionFile {
                    path,
                    parent_session_id: None,
                });
            }
        }
    }
    files
}

/// 同步单个 Gemini 会话文件，返回 (imported, skipped)。
///
/// `last_modified` 来自调用方批量预取的游标（见 [`crate::services::session_usage::load_sync_cursors`]）。
fn sync_single_gemini_file(
    db: &Database,
    file: &GeminiSessionFile,
    last_modified: i64,
) -> Result<(u32, u32), AppError> {
    let file_path_str = file.path.to_string_lossy().to_string();

    // 获取文件元数据
    let metadata = fs::metadata(&file.path)
        .map_err(|e| AppError::Config(format!("无法读取文件元数据: {e}")))?;
    let file_modified = metadata_modified_nanos(&metadata);

    // 文件未变化则跳过
    if file_modified <= last_modified {
        return Ok((0, 0));
    }

    // 收集所有带 tokens 的 gemini 消息（含被回退、移除的，口径见模块文档）
    let scan = jsonl::scan_usage(&file.path).map_err(AppError::Config)?;
    let session_id = scan.meta.session_id.as_deref();
    // 子代理的用量记到父会话名下；2026-03-26 之前 chats 根下的旧子代理没有父 id，用自身 id
    let owner_session_id = file.parent_session_id.as_deref().or(session_id);

    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    let mut gemini_msg_count: i64 = 0;

    for msg in &scan.messages {
        let tokens = parse_gemini_tokens(&msg.tokens);
        if tokens.token_usage().is_empty() {
            continue; // 跳过全零的空 token 消息
        }

        gemini_msg_count += 1;

        // 合成消息没有 model 时已沿用同文件前一条消息的模型，仍没有才记为 unknown
        let model = msg.model.as_deref().unwrap_or("unknown");

        // 生成唯一 request_id（格式不能改：已入库的行靠它去重）
        let session_id_str = session_id.unwrap_or("unknown");
        let request_id = format!("gemini_session:{session_id_str}:{}", msg.id);

        match insert_gemini_session_entry(
            db,
            &request_id,
            &tokens,
            model,
            owner_session_id,
            msg.timestamp.as_deref(),
        ) {
            Ok(true) => imported += 1,
            Ok(false) => skipped += 1,
            Err(e) => {
                log::warn!("[GEMINI-SYNC] 插入失败 ({}): {e}", request_id);
                skipped += 1;
            }
        }
    }

    // 更新同步状态
    update_sync_state(db, &file_path_str, file_modified, gemini_msg_count)?;

    Ok((imported, skipped))
}

/// 从 tokens JSON 对象中提取 token 数据
fn parse_gemini_tokens(tokens: &serde_json::Value) -> GeminiTokens {
    GeminiTokens {
        input: tokens.get("input").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        output: tokens.get("output").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        cached: tokens.get("cached").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        thoughts: tokens.get("thoughts").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        tool: tokens.get("tool").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        total: tokens.get("total").and_then(|v| v.as_u64()),
    }
}

/// 插入单条 Gemini 会话记录到 proxy_request_logs
fn insert_gemini_session_entry(
    db: &Database,
    request_id: &str,
    tokens: &GeminiTokens,
    model: &str,
    session_id: Option<&str>,
    timestamp: Option<&str>,
) -> Result<bool, AppError> {
    let conn = lock_conn!(db.conn);

    let created_at = timestamp
        .and_then(|ts| {
            chrono::DateTime::parse_from_rfc3339(ts)
                .ok()
                .map(|dt| dt.timestamp())
        })
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });

    // 保留期以前的日期已经汇总，再导入会在下次汇总时重复计入
    if crate::services::usage_rebuild::is_below_import_floor(created_at) {
        return Ok(false);
    }

    // 已入库的行不跳过：整份文件重读时要用下面的 UPSERT 补全写到一半的回复
    let usage = tokens.token_usage();

    let [input_cost, output_cost, cache_read_cost, cache_creation_cost, total_cost] =
        match find_model_pricing(&conn, model) {
            Some(p) => CostCalculator::calculate(&usage, &p, ServiceTier::Standard, created_at)
                .to_strings(),
            None => CostBreakdown::zero_strings(),
        };

    // 使用 UPSERT：新记录插入，已存在记录更新 token 和费用（Gemini 全量重读可能携带更新值）
    conn.execute(
        "INSERT INTO proxy_request_logs (
            request_id, provider_id, app_type, model, request_model,
            input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
            input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd, total_cost_usd,
            latency_ms, first_token_ms, status_code, error_message, session_id,
            provider_type, is_streaming, cost_multiplier, created_at, data_source,
            input_token_semantics
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)
        ON CONFLICT(request_id) DO UPDATE SET
            model = excluded.model,
            input_tokens = excluded.input_tokens,
            output_tokens = excluded.output_tokens,
            cache_read_tokens = excluded.cache_read_tokens,
            input_cost_usd = excluded.input_cost_usd,
            output_cost_usd = excluded.output_cost_usd,
            cache_read_cost_usd = excluded.cache_read_cost_usd,
            cache_creation_cost_usd = excluded.cache_creation_cost_usd,
            total_cost_usd = excluded.total_cost_usd,
            input_token_semantics = excluded.input_token_semantics
        WHERE input_tokens != excluded.input_tokens
           OR output_tokens != excluded.output_tokens
           OR cache_read_tokens != excluded.cache_read_tokens
           OR model != excluded.model",
        rusqlite::params![
            request_id,
            "_gemini_session",   // provider_id
            "gemini",            // app_type
            model,
            model,               // request_model = model
            usage.input_tokens,
            usage.output_tokens,
            usage.cache_read_tokens,
            0i64,                // cache_creation_tokens
            input_cost,
            output_cost,
            cache_read_cost,
            cache_creation_cost,
            total_cost,
            0i64,                // latency_ms
            Option::<i64>::None, // first_token_ms
            200i64,              // status_code
            Option::<String>::None, // error_message
            session_id.map(|s| s.to_string()),
            Some("gemini_session"), // provider_type
            1i64,                // is_streaming
            "1.0",               // cost_multiplier
            created_at,
            "gemini_session",    // data_source
            INPUT_TOKEN_SEMANTICS_FRESH,
        ],
    )
    .map_err(|e| AppError::Database(format!("插入 Gemini 会话日志失败: {e}")))?;

    // changes() > 0 表示新插入或已更新，== 0 表示值完全相同（无实际变更）
    let changed = conn.changes() > 0;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::providers::gemini::jsonl::tests::{
        MAIN_SESSION, PROJECT_HASH, SUBAGENT_SESSION,
    };
    use serde_json::json;

    const MAIN_ID: &str = "5f0c1a2b-3c4d-4e5f-8a9b-0c1d2e3f4a5b";
    const AGENT_ID: &str = "a1b2c3d4-0000-4000-8000-000000000001";
    const MAIN_FILE: &str = "session-2026-10-07T08-30-5f0c1a2b.jsonl";

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn lines(values: &[serde_json::Value]) -> String {
        values.iter().map(|v| format!("{v}\n")).collect()
    }

    /// (request_id, session_id, model, input_tokens)，按 request_id 排序
    fn rows(db: &Database) -> Result<Vec<(String, String, String, i64)>, AppError> {
        let conn = lock_conn!(db.conn);
        let mut stmt = conn.prepare(
            "SELECT request_id, session_id, model, input_tokens FROM proxy_request_logs
             WHERE data_source = 'gemini_session' ORDER BY request_id",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// 规格 8.1 / 8.2：主会话 6 次调用（含被 `$rewindTo` 回退的 m-g4）、子代理 2 次；
    /// 旧 hash 目录里复制的副本不重复计数
    #[test]
    fn imports_every_billed_call_once_including_rewound_and_subagent_calls() -> Result<(), AppError>
    {
        let temp = tempfile::tempdir().unwrap();
        let chats = temp.path().join("tmp/my-app/chats");
        write(&chats.join(MAIN_FILE), MAIN_SESSION);
        write(
            &chats.join(MAIN_ID).join(format!("{AGENT_ID}.jsonl")),
            SUBAGENT_SESSION,
        );
        write(
            &temp
                .path()
                .join("tmp")
                .join(PROJECT_HASH)
                .join("chats")
                .join(MAIN_FILE),
            MAIN_SESSION,
        );
        // 被改名保留、原子重写的临时文件不收
        write(&chats.join(format!("{MAIN_FILE}.tmp-42")), MAIN_SESSION);

        let mut files = collect_gemini_session_files(temp.path());
        files.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(files.len(), 3);
        assert_eq!(
            files
                .iter()
                .filter_map(|f| f.parent_session_id.as_deref())
                .collect::<Vec<_>>(),
            [MAIN_ID]
        );

        let db = Database::memory()?;
        let result = sync_gemini_usage_in(&db, temp.path())?;
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.files_scanned, 3);
        assert_eq!(result.imported, 8);

        let main = |id: &str| {
            (
                format!("gemini_session:{MAIN_ID}:{id}"),
                MAIN_ID.to_string(),
                "gemini-3-pro-preview".to_string(),
            )
        };
        // 子代理：request_id 用子代理自己的 id，session_id 列记父会话
        let agent = |id: &str| {
            (
                format!("gemini_session:{AGENT_ID}:{id}"),
                MAIN_ID.to_string(),
                "gemini-3-flash-preview".to_string(),
            )
        };
        let expected = vec![
            main("m-g1"),
            main("m-g2"),
            main("m-g3"),
            main("m-g4"),
            main("m-g5"),
            main("m-g6"),
            agent("s-g1"),
            agent("s-g2"),
        ];
        let stored: Vec<_> = rows(&db)?
            .into_iter()
            .map(|(request, session, model, _)| (request, session, model))
            .collect();
        assert_eq!(stored, expected);
        // 输入减掉缓存命中：m-g1 12034 - 8192
        assert_eq!(rows(&db)?[0].3, 12034 - 8192);

        // 文件没变：不重读
        let again = sync_gemini_usage_in(&db, temp.path())?;
        assert_eq!((again.imported, again.skipped), (0, 0));
        Ok(())
    }

    /// 旧 `.json` 与恢复时迁移出的 `.jsonl` 共存：同一次调用只记一行
    #[test]
    fn legacy_json_and_migrated_jsonl_count_once() -> Result<(), AppError> {
        let temp = tempfile::tempdir().unwrap();
        let chats = temp.path().join("tmp/my-app/chats");
        let session_id = "7e8f9a0b-1111-4222-8333-444455556666";
        let user = json!({ "id": "o-u1", "timestamp": "2026-03-20T09:15:10.000Z", "type": "user", "content": [{ "text": "hello" }] });
        let reply = json!({ "id": "o-g1", "timestamp": "2026-03-20T09:15:12.000Z", "type": "gemini", "content": "Hi!", "thoughts": [],
                            "tokens": { "input": 9000, "output": 5, "cached": 0, "thoughts": 30, "tool": 0, "total": 9035 }, "model": "gemini-2.5-pro" });
        let meta = json!({ "sessionId": session_id, "projectHash": PROJECT_HASH,
                           "startTime": "2026-03-20T09:15:02.000Z", "lastUpdated": "2026-03-20T09:20:41.000Z", "kind": "main" });
        let mut legacy = meta.clone();
        legacy["messages"] = json!([user, reply]);
        write(
            &chats.join("session-2026-03-20T09-15-7e8f9a0b.json"),
            &serde_json::to_string_pretty(&legacy).unwrap(),
        );
        write(
            &chats.join("session-2026-03-20T09-15-7e8f9a0b.jsonl"),
            &lines(&[
                meta,
                user,
                reply,
                json!({ "$set": { "sessionId": session_id } }),
            ]),
        );

        let db = Database::memory()?;
        let result = sync_gemini_usage_in(&db, temp.path())?;
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            rows(&db)?,
            [(
                format!("gemini_session:{session_id}:o-g1"),
                session_id.to_string(),
                "gemini-2.5-pro".to_string(),
                9000
            )]
        );
        Ok(())
    }

    /// `$set.messages` checkpoint 里的 tokens 也收；缺 model 的合成消息沿用前一条的模型；
    /// `$patch` 移除的调用仍计入；chats 根下的旧子代理记在自己名下
    #[test]
    fn checkpoints_model_fallback_and_legacy_subagents() -> Result<(), AppError> {
        let temp = tempfile::tempdir().unwrap();
        let chats = temp.path().join("tmp/p/chats");
        let tokens = |input: u64| json!({ "input": input, "output": 1, "cached": 0, "thoughts": 0, "tool": 0, "total": input + 1 });
        let gemini = |id: &str, input: u64, model: Option<&str>| {
            let mut message = json!({ "id": id, "timestamp": "2026-10-07T08:00:00Z", "type": "gemini",
                                      "content": "", "tokens": tokens(input) });
            if let Some(model) = model {
                message["model"] = json!(model);
            }
            message
        };
        write(
            &chats.join("session-2026-10-07T08-00-s1.jsonl"),
            &lines(&[
                json!({ "sessionId": "s1", "projectHash": "h1", "kind": "main" }),
                // checkpoint 才第一次出现的消息
                json!({ "$set": { "messages": [gemini("g0", 100, Some("gemini-2.5-flash"))] } }),
                gemini("g1", 200, Some("gemini-2.5-pro")),
                gemini("g2", 300, None),
                json!({ "id": "g3", "type": "gemini", "content": "", "tokens": null, "model": "gemini-2.5-pro" }),
                json!({ "$patch": { "removeIds": ["g1", "g2"] } }),
                json!({ "$rewindTo": "nope" }),
                "{\"id\":\"g4\",\"type\":\"gemini\",\"tokens\":{\"input\":9".into(),
            ]),
        );
        write(
            &chats.join("session-2026-03-01T00-00-a0a0a0a0.json"),
            &json!({ "sessionId": "a0a0a0a0", "projectHash": "h1", "kind": "subagent",
                     "messages": [gemini("a-g1", 50, Some("gemini-2.5-flash"))] })
            .to_string(),
        );

        let db = Database::memory()?;
        let result = sync_gemini_usage_in(&db, temp.path())?;
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let row = |sid: &str, id: &str, model: &str, input: i64| {
            (
                format!("gemini_session:{sid}:{id}"),
                sid.to_string(),
                model.to_string(),
                input,
            )
        };
        assert_eq!(
            rows(&db)?,
            [
                row("a0a0a0a0", "a-g1", "gemini-2.5-flash", 50),
                row("s1", "g0", "gemini-2.5-flash", 100),
                row("s1", "g1", "gemini-2.5-pro", 200),
                row("s1", "g2", "gemini-2.5-pro", 300),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_collect_gemini_session_files_nonexistent() {
        let files = collect_gemini_session_files(Path::new("/nonexistent/path"));
        assert!(files.is_empty());
    }

    #[test]
    fn test_insert_gemini_session_stores_exclusive_buckets() -> Result<(), AppError> {
        let db = Database::memory()?;
        let tokens = GeminiTokens {
            input: 10,
            output: 2,
            cached: 1,
            thoughts: 5,
            ..Default::default()
        };
        let inserted = insert_gemini_session_entry(
            &db,
            "gemini-session-1",
            &tokens,
            "gemini-2.5-pro",
            Some("session-1"),
            Some("1970-01-01T00:16:45Z"),
        )?;
        assert!(inserted);

        let conn = lock_conn!(db.conn);
        let stored: (i64, i64, i64) = conn.query_row(
            "SELECT input_tokens, output_tokens, cache_read_tokens
             FROM proxy_request_logs WHERE request_id = 'gemini-session-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        // 输入减掉缓存命中，输出含 thoughts
        assert_eq!(stored, (9, 7, 1));
        Ok(())
    }

    #[test]
    fn test_parse_gemini_tokens() {
        let json: serde_json::Value = serde_json::json!({
            "input": 8522,
            "output": 29,
            "cached": 3138,
            "thoughts": 405,
            "tool": 0,
            "total": 8956
        });
        let tokens = parse_gemini_tokens(&json);
        assert_eq!(tokens.input, 8522);
        assert_eq!(tokens.output, 29);
        assert_eq!(tokens.cached, 3138);
        assert_eq!(tokens.thoughts, 405);
        // output + thoughts = 29 + 405 = 434（用于计费）
        assert_eq!(tokens.output + tokens.thoughts, 434);
    }

    #[test]
    fn test_parse_gemini_tokens_missing_fields() {
        // 缺少某些字段时应返回 0
        let json: serde_json::Value = serde_json::json!({
            "input": 100,
            "output": 50
        });
        let tokens = parse_gemini_tokens(&json);
        assert_eq!(tokens.input, 100);
        assert_eq!(tokens.output, 50);
        assert_eq!(tokens.cached, 0);
        assert_eq!(tokens.thoughts, 0);
    }

    #[test]
    fn test_parse_gemini_tokens_all_zero() {
        let json: serde_json::Value = serde_json::json!({
            "input": 0,
            "output": 0,
            "cached": 0,
            "thoughts": 0,
            "tool": 0,
            "total": 0
        });
        let tokens = parse_gemini_tokens(&json);
        assert_eq!(tokens.input, 0);
        assert_eq!(tokens.output, 0);
        // 全零（包括 cached=0）会被 sync 逻辑跳过
        assert!(
            tokens.input == 0 && tokens.output == 0 && tokens.thoughts == 0 && tokens.cached == 0
        );
    }

    #[test]
    fn test_parse_gemini_tokens_cache_only_not_skipped() {
        // 纯缓存命中消息（input/output/thoughts=0 但 cached>0）不应被跳过
        let json: serde_json::Value = serde_json::json!({
            "input": 0,
            "output": 0,
            "cached": 5000,
            "thoughts": 0
        });
        let tokens = parse_gemini_tokens(&json);
        assert_eq!(tokens.cached, 5000);
        // 跳过条件：所有四个字段都为 0 才跳过
        let should_skip =
            tokens.input == 0 && tokens.output == 0 && tokens.thoughts == 0 && tokens.cached == 0;
        assert!(!should_skip, "纯缓存命中记录不应被跳过");
    }
}
