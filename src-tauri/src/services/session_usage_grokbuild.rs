//! Grok Build (Grok CLI) 会话用量追踪
//!
//! 从 `~/.grok/{sessions,archived_sessions}/<enc-cwd>/<session-id>/updates.jsonl`
//! 的 `turn_completed` 事件中提取用量，写入 proxy_request_logs。
//!
//! ## 数据流
//! ```text
//! updates.jsonl（逐轮 turn_completed） → 费用计算 → proxy_request_logs
//! ```
//!
//! ## 事件口径（2026-07-23 单进程双 prompt 实测 + CLI 二进制逆向双重确证）
//! - `sessionUpdate == "turn_completed"` 事件的 usage 是【该 user prompt 一轮
//!   的独立总量】：轮内跨 inference loop 累加（`modelCalls`/`numTurns` = 本轮
//!   loop 数），下一轮从零起算。【不是】进程或会话累计——进程累计走 CLI 内
//!   另一条独立通道（`GetSessionUsage`，"since start or last resume"），不落
//!   updates.jsonl。🔴 勿改回相邻事件差分：那是把每轮总量误当累计快照，会把
//!   第二轮记成两轮之差造成巨量漏记（曾犯，实测单进程双 prompt 证伪）。
//! - 逐事件按面值入账即为正确的逐轮记录；两轮数值完全相同 = 两笔真实用量，
//!   照常都入账。
//! - `reasoningTokens` ⊂ `outputTokens`（totalTokens = input + output，且
//!   costUsdTicks 反推 output 未加计 reasoning），不参与计费。
//! - `costUsdTicks`（1 tick = 1e-10 USD）是 CLI 自报的本轮精确成本，6 个实测
//!   样本与本地定价 grok-4.5-build 2/6/0.30 分毫不差。**有自报且完整时
//!   total_cost 以自报为准**（回填只补 total<=0 的行、不修正错价，入账后无
//!   修复路径，所以定价漂移窗口不能押在本地价上）；本地定价负责分项成本与
//!   漂移告警。费用不完整（`costIsPartial` / `usageIsIncomplete`）时 grok-build
//!   写出前已删掉 costUsdTicks（`scrub_untrustworthy_costs`），按本地定价复算。
//! - `cacheCreationTokens` 和 `cachedReadTokens` 都含在 `inputTokens` 里，缓存写
//!   按缓存写价格单独计。
//! - 子代理会话不导入：它的用量已并入父会话当轮的 turn_completed。fork 会话
//!   复制了源会话的全部事件，源会话已有的 prompt_id 跳过（见 [`GrokSessionOrigin`]）。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::session_usage::{
    metadata_modified_nanos, update_sync_state, SessionSyncResult,
};
use crate::services::sql_helpers::INPUT_TOKEN_SEMANTICS_FRESH;
use crate::services::usage_stats::find_model_pricing;
use crate::token_usage::calculator::{CostCalculator, ModelPricing, ServiceTier};
use crate::token_usage::parser::TokenUsage;
use rusqlite::OptionalExtension;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// 单个模型的本轮用量（从 `modelUsage` 或顶层 usage 提取，均为逐轮口径）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct GrokCounters {
    /// 含缓存读和缓存写的全部输入
    input: u64,
    output: u64,
    cached: u64,
    /// `cacheCreationTokens`，含在 `input` 里（grok-build `PromptUsageModel`）
    cache_write: u64,
    api_ms: u64,
    model_calls: u64,
    /// CLI 自报本轮成本，1 tick = 1e-10 USD；0 = 上游未提供。费用不完整
    /// （`costIsPartial` / `usageIsIncomplete`）时 grok-build 写出前就清掉了它
    /// （`scrub_untrustworthy_costs`），所以有值就是完整的
    cost_ticks: u64,
}

impl GrokCounters {
    fn is_zero(&self) -> bool {
        self.input == 0 && self.output == 0 && self.cached == 0
    }

    fn reported_cost_usd(&self) -> Option<Decimal> {
        (self.cost_ticks > 0)
            .then(|| Decimal::from(self.cost_ticks) / Decimal::from(10_000_000_000u64))
    }
}

/// 一条 `turn_completed` 用量事件
#[derive(Debug)]
struct GrokUsageEvent {
    created_at: i64,
    prompt_id: String,
    per_model: Vec<(String, GrokCounters)>,
}

/// summary.json 里决定用量归属的字段（grok-build `Summary`）
#[derive(Debug, Default, Deserialize)]
struct GrokSessionOrigin {
    /// `subagent*` / `fork` / `worktree` 等；普通会话缺省
    #[serde(default)]
    session_kind: Option<String>,
    /// fork 出来的会话指向源会话
    #[serde(default)]
    parent_session_id: Option<String>,
}

impl GrokSessionOrigin {
    fn read(session_dir: &Path) -> Self {
        fs::read_to_string(session_dir.join("summary.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// 子代理的用量在结束时并入父会话当轮（grok-build `record_subagent_usage` →
    /// `SubagentUsageApply::AttributedToPrompt`），父会话的 turn_completed 已经含这部分。
    /// 父会话那一轮已结束时只记进父会话的会话总账（`SessionOnly`），不进任何
    /// turn_completed，这部分会少记；本机 25 份 usage.json 里会话总账都等于逐轮之和，
    /// 未观测到。
    fn is_subagent(&self) -> bool {
        self.session_kind
            .as_deref()
            .is_some_and(|kind| kind.starts_with("subagent"))
    }

    /// fork 会逐行复制源会话的 updates.jsonl（只改会话 ID，prompt_id 不变，见 grok-build
    /// `session/storage/jsonl/copy.rs`），复制来的轮次源会话已经记过
    fn parent(&self) -> Option<&str> {
        self.parent_session_id
            .as_deref()
            .filter(|id| !id.is_empty())
    }
}

/// 同步 Grok Build 使用数据（从 updates.jsonl 会话日志）
pub fn sync_grokbuild_usage(db: &Database) -> Result<SessionSyncResult, AppError> {
    for root in crate::session_manager::providers::grokbuild::session_roots() {
        crate::services::session_usage::ensure_readable_if_present(&root)?;
    }
    let files = collect_grok_updates_files();
    let files_by_session: HashMap<&str, &Path> = files
        .iter()
        .filter_map(|path| Some((session_id_of(path)?, path.as_path())))
        .collect();

    let mut result = SessionSyncResult {
        files_scanned: files.len() as u32,
        ..Default::default()
    };

    let cursors = crate::services::session_usage::load_sync_cursors(db)?;

    for file_path in &files {
        match sync_single_grok_file(db, file_path, &cursors, &files_by_session) {
            Ok(file_result) => result.merge(file_result),
            Err(e) => {
                let msg = format!("Grok Build 会话文件解析失败 {}: {e}", file_path.display());
                log::warn!("[GROK-SYNC] {msg}");
                result.errors.push(msg);
            }
        }
    }

    if result.imported > 0 {
        log::info!(
            "[GROK-SYNC] 同步完成: 导入 {} 条, 跳过 {} 条, 扫描 {} 个文件, 延后 {} 个文件",
            result.imported,
            result.skipped,
            result.files_scanned,
            result.deferred_files
        );
    }

    Ok(result)
}

/// 收集所有 Grok 会话的 updates.jsonl（含归档会话，与会话浏览器同根）
fn collect_grok_updates_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in crate::session_manager::providers::grokbuild::session_roots() {
        collect_files_named(&root, "updates.jsonl", &mut files, 0);
    }
    files
}

/// 递归收集 session 日志时的最大目录深度，防止 symlink 循环导致栈溢出。
const MAX_COLLECT_DEPTH: usize = 16;

/// 递归收集目录下指定文件名的文件（容忍布局深度变化，对齐会话浏览器的做法）
fn collect_files_named(root: &Path, name: &str, files: &mut Vec<PathBuf>, depth: usize) {
    if depth > MAX_COLLECT_DEPTH {
        log::warn!(
            "Grok session directory traversal exceeded max depth {} at {}",
            MAX_COLLECT_DEPTH,
            root.display()
        );
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // `entry.metadata()` 不跟随符号链接（不同于 `path.is_dir()`），这里据此
        // **无条件跳过一切 symlink**：目录 symlink 不递归（避免循环），文件
        // symlink 也不收集——同名文件若经 symlink 指向 sessions 根之外，会把用户
        // 意料之外的内容当作会话日志读入。代价：把 sessions 目录整体做成 symlink
        // 的用户会同步不到数据，所以跳过必须留日志，便于排查"用量数据静默缺失"。
        let metadata = entry.metadata();
        if metadata.as_ref().map(|m| m.is_symlink()).unwrap_or(false) {
            log::info!("[GROK-SYNC] 跳过符号链接（不跟随）: {}", path.display());
            continue;
        }
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        if is_dir {
            collect_files_named(&path, name, files, depth + 1);
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            files.push(path);
        }
    }
}

/// 会话 ID = 会话目录名（与 summary.json 的 info.id 一致）
fn session_id_of(updates_path: &Path) -> Option<&str> {
    updates_path.parent()?.file_name()?.to_str()
}

fn read_usage_events(path: &Path) -> Result<Vec<GrokUsageEvent>, AppError> {
    // 逐行读：长会话的 updates.jsonl 可以有上百 MB（大部分是流式消息块）
    let file = fs::File::open(path).map_err(|e| AppError::Config(format!("无法打开文件: {e}")))?;
    parse_grok_usage_events(BufReader::new(file))
        .map_err(|e| AppError::Config(format!("无法读取文件: {e}")))
}

const GROK_TAIL_DOMAIN: &[u8] = b"grok-updates-tail-v1";

/// 一次增量读取的结果。
#[derive(Debug)]
struct GrokFileRead {
    events: Vec<GrokUsageEvent>,
    /// `events[0]` 在整个文件的用量事件里的序号（`idx{N}` 回退键用）
    first_index: usize,
    /// 以换行结尾的事件数；没有换行的尾行也解析，但不算已提交
    committed_events: usize,
    /// 最后一个完整行之后的字节位置
    committed_offset: i64,
    tail_fingerprint: i64,
}

/// 从游标处读 updates.jsonl 追加的部分。
///
/// 每个 turn_completed 事件是一轮的独立合计，不依赖前面的事件，所以只读游标之后
/// 的字节就够了。游标超出文件大小（rewind 截断）或游标前的尾部字节变了（改写）时
/// 从头读：入库按 prompt_id UPSERT，重读不会重复计数。
fn read_usage_events_since(
    path: &Path,
    cursor: Option<&crate::services::session_usage::SyncCursor>,
) -> Result<GrokFileRead, AppError> {
    use crate::services::session_usage::{read_tail_before, tail_fingerprint};
    use std::io::{Seek, SeekFrom};

    let mut file =
        fs::File::open(path).map_err(|e| AppError::Config(format!("无法打开文件: {e}")))?;
    let size = file
        .metadata()
        .map_err(|e| AppError::Config(format!("无法读取文件元数据: {e}")))?
        .len() as i64;
    let resume = cursor.and_then(|cursor| {
        let offset = cursor.last_byte_offset?;
        let expected = cursor.last_tail_fingerprint?;
        Some((offset, expected, cursor.last_line_offset))
    });
    let (start, first_index) = match resume {
        Some((offset, expected, events)) if (0..=size).contains(&offset) => {
            let tail = read_tail_before(&mut file, offset)?;
            if tail_fingerprint(GROK_TAIL_DOMAIN, &tail) == expected {
                (offset, usize::try_from(events).unwrap_or(0))
            } else {
                (0, 0)
            }
        }
        _ => (0, 0),
    };
    file.seek(SeekFrom::Start(start as u64))
        .map_err(|e| AppError::Config(format!("无法定位文件偏移: {e}")))?;

    let mut reader = BufReader::new(file);
    let mut events = Vec::new();
    let mut committed_events = 0;
    let mut committed_offset = start;
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader
            .read_until(b'\n', &mut line)
            .map_err(|e| AppError::Config(format!("无法读取文件: {e}")))?;
        if read == 0 {
            break;
        }
        let complete = line.ends_with(b"\n");
        if let Some(event) = parse_usage_line(&line) {
            events.push(event);
        }
        if !complete {
            break;
        }
        committed_offset += read as i64;
        committed_events = events.len();
    }

    let mut file = reader.into_inner();
    let tail = read_tail_before(&mut file, committed_offset)?;
    Ok(GrokFileRead {
        events,
        first_index,
        committed_events,
        committed_offset,
        tail_fingerprint: tail_fingerprint(GROK_TAIL_DOMAIN, &tail),
    })
}

/// fork 源会话的 prompt_id 集合，按 (路径, mtime, 大小) 缓存：源会话日志可能很大，
/// fork 会话每次变化都要用到。
fn parent_prompt_ids(path: &Path) -> Option<std::sync::Arc<HashSet<String>>> {
    use std::sync::{Arc, Mutex, OnceLock};
    type Cache = HashMap<PathBuf, ((i64, u64), Arc<HashSet<String>>)>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

    let metadata = fs::metadata(path).ok()?;
    let version = (metadata_modified_nanos(&metadata), metadata.len());
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some((cached_version, prompts)) = cache.lock().ok()?.get(path) {
        if *cached_version == version {
            return Some(prompts.clone());
        }
    }
    let prompts: Arc<HashSet<String>> = Arc::new(
        read_usage_events(path)
            .ok()?
            .into_iter()
            .map(|event| event.prompt_id)
            .collect(),
    );
    cache
        .lock()
        .ok()?
        .insert(path.to_path_buf(), (version, prompts.clone()));
    Some(prompts)
}

/// 同步单个 updates.jsonl 文件。游标来自调用方批量预取。
fn sync_single_grok_file(
    db: &Database,
    file_path: &Path,
    cursors: &HashMap<String, crate::services::session_usage::SyncCursor>,
    files_by_session: &HashMap<&str, &Path>,
) -> Result<SessionSyncResult, AppError> {
    let file_path_str = file_path.to_string_lossy().to_string();

    let metadata = fs::metadata(file_path)
        .map_err(|e| AppError::Config(format!("无法读取文件元数据: {e}")))?;
    let file_modified = metadata_modified_nanos(&metadata);

    let last_modified = cursors.get(&file_path_str).map_or(0, |c| c.last_modified);
    if file_modified <= last_modified {
        return Ok(SessionSyncResult::default());
    }

    let origin = file_path
        .parent()
        .map(GrokSessionOrigin::read)
        .unwrap_or_default();
    if origin.is_subagent() {
        update_sync_state(db, &file_path_str, file_modified, 0)?;
        return Ok(SessionSyncResult::default());
    }

    // 只读游标之后追加的部分；改写或截断时从头读，UPSERT 幂等使重读无害
    let read = read_usage_events_since(file_path, cursors.get(&file_path_str))?;

    // request_id 唯一性押在会话 ID（UUIDv7）全局唯一上：同 ID 的归档/活跃副本经
    // UPSERT 幂等收敛（有意），不同 <enc-cwd> 下撞 ID 视为不可能。
    let session_id = session_id_of(file_path).unwrap_or("unknown").to_string();

    // fork：源会话日志还在时取它的全部 prompt_id；源会话已删时逐条查库里有没有源会话的同一行
    let parent = origin.parent();
    let parent_prompts = parent
        .and_then(|id| files_by_session.get(id))
        .and_then(|path| parent_prompt_ids(path));

    let mut result = SessionSyncResult::default();

    // 一个文件一个事务：逐行自动提交每行都要一次 fsync。单条失败跳过；提交失败
    // 整个文件回滚，游标不推进，下一轮重读
    let conn = lock_conn!(db.conn);
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::Database(format!("开启 Grok 会话写入事务失败: {e}")))?;

    for (offset, event) in read.events.iter().enumerate() {
        let idx = read.first_index + offset;
        for (model, turn) in &event.per_model {
            if turn.is_zero() {
                continue;
            }
            // 幂等键锚定上游稳定 ID（prompt_id 是每轮唯一的 UUID），不含会话
            // ID 和文件内序号：fork 复制的轮次 prompt_id 不变，和源会话是同一个
            // 键，不用知道祖先链（A → B → C、B 已删除时也认得出）；updates.jsonl
            // 前缀被改写（如 rewind 截断）导致事件序号前移时，幸存轮次仍命中原行
            // 不会双算；被移除轮次的行保留——rewind 不退还已消耗的 token，留存即
            // 正确记账。若上游对同一 prompt_id 写多条 turn_completed（未观测到），
            // UPSERT 取后者，方向是少记不双算。prompt_id 缺失时回退
            // "{session}:idx{N}"（UUID 形态的 prompt_id 不可能与之撞名）。
            let request_id = if event.prompt_id.is_empty() {
                format!("grok_session:{session_id}:idx{idx}:{model}")
            } else {
                format!("grok_session:{}:{model}", event.prompt_id)
            };
            if !event.prompt_id.is_empty() {
                // 复制来的轮次：源会话日志还在时看它的 prompt_id；否则看这个键是不是
                // 已经记在别的会话名下（已汇总的由导入账本挡住）
                let inherited = match &parent_prompts {
                    Some(prompts) => prompts.contains(&event.prompt_id),
                    None => {
                        grok_row_session(&tx, &request_id)?.is_some_and(|owner| owner != session_id)
                    }
                };
                if inherited {
                    result.skipped += 1;
                    continue;
                }
            }
            match insert_grok_session_entry(
                &tx,
                &request_id,
                turn,
                model,
                &session_id,
                event.created_at,
            ) {
                Ok(true) => result.imported += 1,
                Ok(false) => result.skipped += 1,
                Err(e) => {
                    log::warn!("[GROK-SYNC] 插入失败 ({request_id}): {e}");
                    result.skipped += 1;
                }
            }
        }
    }

    crate::services::session_usage::update_byte_cursor_on_conn(
        &tx,
        &file_path_str,
        file_modified,
        (read.first_index + read.committed_events) as i64,
        read.committed_offset,
        read.tail_fingerprint,
    )?;
    tx.commit()
        .map_err(|e| AppError::Database(format!("提交 Grok 会话写入事务失败: {e}")))?;

    Ok(result)
}

/// 用量事件所在行的 `method`，按字节先筛一遍行
const USAGE_METHOD_MARKER: &[u8] = b"_x.ai/session/update";

/// 从 updates.jsonl 解析出全部逐轮用量事件（保持文件顺序）。解析不了的行跳过。
fn parse_grok_usage_events(reader: impl BufRead) -> std::io::Result<Vec<GrokUsageEvent>> {
    let mut events = Vec::new();
    for line in reader.split(b'\n') {
        if let Some(event) = parse_usage_line(&line?) {
            events.push(event);
        }
    }
    Ok(events)
}

/// 解析一行；不是用量事件或解析不了返回 `None`。
fn parse_usage_line(line: &[u8]) -> Option<GrokUsageEvent> {
    // 先按行头过滤：绝大多数行是流式消息块，不用整行解析
    if !line
        .windows(USAGE_METHOD_MARKER.len())
        .any(|window| window == USAGE_METHOD_MARKER)
    {
        return None;
    }
    let record = serde_json::from_slice::<serde_json::Value>(line).ok()?;
    if record.get("method").and_then(|v| v.as_str()) != Some("_x.ai/session/update") {
        return None;
    }
    let update = record.get("params").and_then(|p| p.get("update"));
    // 只认 turn_completed（实测全体带 usage 的事件均为此类；判别字段是
    // sessionUpdate，serde internally-tagged）。字段缺失时向后兼容放行，
    // 但显式标为其它类型的事件即使带 usage 也不导入——中途快照若与轮末
    // 事件并存，双导会双算。
    let kind = update
        .and_then(|u| u.get("sessionUpdate"))
        .and_then(|v| v.as_str());
    if kind.is_some() && kind != Some("turn_completed") {
        return None;
    }
    let usage = update
        .and_then(|u| u.get("usage"))
        .filter(|u| u.is_object())?;
    // 没有时间戳的事件不知道该记到哪一天，跳过。
    let created_at = parse_event_timestamp(record.get("timestamp"))?;

    let prompt_id = update
        .and_then(|u| u.get("prompt_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut per_model: Vec<(String, GrokCounters)> = usage
        .get("modelUsage")
        .and_then(|m| m.as_object())
        .map(|map| {
            map.iter()
                .map(|(model, counters)| (model.clone(), parse_grok_counters(counters)))
                .collect()
        })
        .unwrap_or_default();
    if per_model.is_empty() {
        // 缺 modelUsage 时退回顶层逐轮值；模型名未知，交由查价层兜底。
        per_model.push(("unknown".to_string(), parse_grok_counters(usage)));
    }
    // modelUsage 是 JSON object，遍历序不保证稳定；排序保证插入顺序
    // 与日志在多次重扫间确定。
    per_model.sort_by(|a, b| a.0.cmp(&b.0));

    Some(GrokUsageEvent {
        created_at,
        prompt_id,
        per_model,
    })
}

fn parse_grok_counters(value: &serde_json::Value) -> GrokCounters {
    let get = |key: &str| value.get(key).and_then(|v| v.as_u64()).unwrap_or(0);
    GrokCounters {
        input: get("inputTokens"),
        output: get("outputTokens"),
        cached: get("cachedReadTokens"),
        cache_write: get("cacheCreationTokens"),
        api_ms: get("apiDurationMs"),
        model_calls: get("modelCalls"),
        cost_ticks: get("costUsdTicks"),
    }
}

/// updates.jsonl 顶层 `timestamp` 实测为数字 epoch 秒（勿与 summary.json 的
/// RFC3339 字符串混淆）；字符串形态仅作防御性兜底。
fn parse_event_timestamp(value: Option<&serde_json::Value>) -> Option<i64> {
    let value = value?;
    if let Some(n) = value.as_i64() {
        // 防未来毫秒形态：超过 1e11 视作毫秒
        return Some(if n > 100_000_000_000 { n / 1000 } else { n });
    }
    value
        .as_str()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.timestamp())
}

/// 这个键已入库时记在哪个会话名下
fn grok_row_session(
    conn: &rusqlite::Connection,
    request_id: &str,
) -> Result<Option<String>, AppError> {
    conn.query_row(
        "SELECT COALESCE(session_id, '') FROM proxy_request_logs WHERE request_id = ?1",
        [request_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| AppError::Database(format!("查询 Grok 用量记录失败: {e}")))
}

/// 插入单条 Grok 会话记录到 proxy_request_logs
fn insert_grok_session_entry(
    conn: &rusqlite::Connection,
    request_id: &str,
    turn: &GrokCounters,
    model: &str,
    session_id: &str,
    created_at: i64,
) -> Result<bool, AppError> {
    // 保留期以前的日期已经汇总；导入过的整份文件重读时再写回会在下次汇总时重复计入
    if !crate::services::usage_rebuild::import_gate(conn, "grok_session", request_id, created_at) {
        return Ok(false);
    }
    // 回合聚合的用量和旧路由逐请求记的行对不上指纹，按当时是否在走路由判断
    if crate::services::usage_proxy_dedup::skip_if_grokbuild_routed(conn, request_id, created_at)? {
        return Ok(false);
    }

    // inputTokens 含缓存读和缓存写；入库换成未命中缓存的输入，缓存写单独计价
    let clamp = |v: u64| v.min(u32::MAX as u64) as u32;
    let cached = turn.cached.min(turn.input);
    let cache_write = turn.cache_write.min(turn.input - cached);
    let usage = TokenUsage {
        input_tokens: clamp(turn.input - cached - cache_write),
        output_tokens: clamp(turn.output),
        cache_read_tokens: clamp(cached),
        cache_creation_tokens: clamp(cache_write),
        cache_creation_1h_tokens: 0,
    };

    // 一行是一轮的合计，超长上下文档位无从判断
    let pricing = find_model_pricing(conn, model).map(ModelPricing::without_long_context);
    let reported = turn.reported_cost_usd();
    // 合计取 CLI 自报的费用时标记 native_cost，按定价重算时不覆盖它
    let native_cost = reported.is_some();
    // 插入成功（changed）后才发，避免重扫时重复刷日志
    let mut deferred_warn: Option<String> = None;

    // total_cost 取值优先级：
    // 1. 有自报 → 以自报为准（上游 ground truth，定价漂移窗口内也准确；
    //    本地定价负责分项与漂移告警，漂移时分项与 total 允许暂不自洽）；
    // 2. 无自报（含费用不完整被清掉的）→ 本地复算；彻底无价才整单记 0。
    let (input_cost, output_cost, cache_read_cost, cache_creation_cost, total_cost) = match pricing
    {
        Some(p) => {
            let cost = CostCalculator::calculate(&usage, &p, ServiceTier::Standard, created_at);
            let total = match reported {
                Some(reported) => {
                    // 偏差超 1%（微额下限 1e-6）即本地定价漂移——xAI 调价时
                    // 最早的可观测信号，提醒更新 seed/repair。
                    let tolerance = (reported * Decimal::new(1, 2)).max(Decimal::new(1, 6));
                    if (cost.total_cost - reported).abs() > tolerance {
                        deferred_warn = Some(format!(
                            "本地定价与 CLI 自报成本偏差超阈值，total 已以自报为准，请更新本地定价: model={model} local={} reported={reported} request_id={request_id}",
                            cost.total_cost
                        ));
                    }
                    reported
                }
                _ => cost.total_cost,
            };
            (
                cost.input_cost.to_string(),
                cost.output_cost.to_string(),
                cost.cache_read_cost.to_string(),
                cost.cache_creation_cost.to_string(),
                total.to_string(),
            )
        }
        None => {
            // 未 seed 的新别名：token 照常入账；有自报成本时直接采用（分项
            // 记 0），彻底无价才整单记 0。xAI 内部别名会周期性变动
            // （grok-4.5-build 即先例），两种情况都要留下可排查的痕迹。
            let total = match reported {
                Some(reported) => {
                    if model != "unknown" {
                        deferred_warn = Some(format!(
                            "模型定价未找到，采用 CLI 自报成本入账: model={model} total={reported} request_id={request_id}"
                        ));
                    }
                    reported.to_string()
                }
                None => {
                    if model != "unknown" {
                        deferred_warn = Some(format!(
                            "模型定价未找到且无自报成本，成本记 0: model={model} request_id={request_id}"
                        ));
                    }
                    "0".to_string()
                }
            };
            (
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                total,
            )
        }
    };

    // UPSERT：重扫幂等；解析口径修正后重扫时更新既有行（token/成本/
    // latency；created_at 保持首插值不动，避免行在 rollup 边界间漂移）。
    // WHERE 的 data_source 守卫是纵深防御：request_id 前缀命名空间已隔离，
    // 万一撞上非本导入器的行也绝不改写它。
    conn.execute(
        "INSERT INTO proxy_request_logs (
            request_id, provider_id, app_type, model, request_model,
            input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
            input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd, total_cost_usd,
            latency_ms, first_token_ms, status_code, error_message, session_id,
            provider_type, is_streaming, cost_multiplier, created_at, data_source,
            input_token_semantics, native_cost
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)
        ON CONFLICT(request_id) DO UPDATE SET
            model = excluded.model,
            input_tokens = excluded.input_tokens,
            output_tokens = excluded.output_tokens,
            cache_read_tokens = excluded.cache_read_tokens,
            cache_creation_tokens = excluded.cache_creation_tokens,
            input_cost_usd = excluded.input_cost_usd,
            output_cost_usd = excluded.output_cost_usd,
            cache_read_cost_usd = excluded.cache_read_cost_usd,
            cache_creation_cost_usd = excluded.cache_creation_cost_usd,
            total_cost_usd = excluded.total_cost_usd,
            latency_ms = excluded.latency_ms,
            input_token_semantics = excluded.input_token_semantics,
            native_cost = excluded.native_cost
        WHERE data_source = 'grok_session'
          AND (input_tokens != excluded.input_tokens
           OR output_tokens != excluded.output_tokens
           OR cache_read_tokens != excluded.cache_read_tokens
           OR cache_creation_tokens != excluded.cache_creation_tokens
           OR latency_ms != excluded.latency_ms
           OR model != excluded.model
           OR total_cost_usd != excluded.total_cost_usd
           OR input_token_semantics != excluded.input_token_semantics)",
        rusqlite::params![
            request_id,
            "_grok_session",     // provider_id
            "grokbuild",         // app_type
            model,
            model,               // request_model = model
            usage.input_tokens,
            usage.output_tokens,
            usage.cache_read_tokens,
            usage.cache_creation_tokens,
            input_cost,
            output_cost,
            cache_read_cost,
            cache_creation_cost,
            total_cost,
            turn.api_ms.min(i64::MAX as u64) as i64, // latency_ms（本轮 API 时长）
            Option::<i64>::None, // first_token_ms
            200i64,              // status_code
            Option::<String>::None, // error_message
            session_id,
            Some("grok_session"), // provider_type
            1i64,                // is_streaming
            "1.0",               // cost_multiplier
            created_at,
            "grok_session",      // data_source
            INPUT_TOKEN_SEMANTICS_FRESH,
            i64::from(native_cost),
        ],
    )
    .map_err(|e| AppError::Database(format!("插入 Grok Build 会话日志失败: {e}")))?;

    // changes() > 0 表示新插入或已更新，== 0 表示值完全相同（无实际变更）
    let changed = conn.changes() > 0;
    if changed {
        if let Some(msg) = deferred_warn {
            log::warn!("[GROK-SYNC] {msg}");
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::session_usage::get_sync_state;
    use std::io::Write;
    use std::time::SystemTime;
    use tempfile::tempdir;

    /// 固定的过去时刻（2023-11-14T22:13:20Z）
    const OLD_EPOCH: i64 = 1_700_000_000;

    fn epoch_to_rfc3339(epoch: i64) -> String {
        chrono::DateTime::from_timestamp(epoch, 0)
            .expect("valid epoch")
            .to_rfc3339()
    }

    /// 顶层 timestamp 用真实的数字 epoch 秒格式（RFC3339 兜底见 parses 测试）
    fn usage_event_line(epoch: i64, prompt_id: &str, model_usage: &str) -> String {
        format!(
            r#"{{"timestamp":{epoch},"method":"_x.ai/session/update","params":{{"update":{{"sessionUpdate":"turn_completed","prompt_id":"{prompt_id}","stop_reason":"end_turn","usage":{{"modelUsage":{{{model_usage}}}}}}}}}}}"#
        )
    }

    fn model_counters(model: &str, input: u64, output: u64, cached: u64, calls: u64) -> String {
        model_counters_with_ticks(model, input, output, cached, calls, 0)
    }

    fn model_counters_with_ticks(
        model: &str,
        input: u64,
        output: u64,
        cached: u64,
        calls: u64,
        ticks: u64,
    ) -> String {
        format!(
            r#""{model}":{{"inputTokens":{input},"outputTokens":{output},"cachedReadTokens":{cached},"reasoningTokens":0,"modelCalls":{calls},"apiDurationMs":1000,"costUsdTicks":{ticks}}}"#
        )
    }

    fn sync_file(db: &Database, path: &Path) -> Result<SessionSyncResult, AppError> {
        sync_single_grok_file(
            db,
            path,
            &crate::services::session_usage::load_sync_cursors(db).unwrap(),
            &HashMap::new(),
        )
    }

    fn write_session_file(dir: &Path, session_id: &str, lines: &[String]) -> PathBuf {
        let session_dir = dir.join("sessions").join("enc-project").join(session_id);
        std::fs::create_dir_all(&session_dir).expect("create session dir");
        let path = session_dir.join("updates.jsonl");
        let mut file = std::fs::File::create(&path).expect("create updates.jsonl");
        for line in lines {
            writeln!(file, "{line}").expect("write line");
        }
        path
    }

    /// (request_id, input, output, cache_read, input_token_semantics)
    type GrokSessionRow = (String, u32, u32, u32, i64);

    fn query_rows(db: &Database) -> Result<Vec<GrokSessionRow>, AppError> {
        let conn = lock_conn!(db.conn);
        let mut stmt = conn
            .prepare(
                "SELECT request_id, input_tokens, output_tokens, cache_read_tokens, input_token_semantics
                 FROM proxy_request_logs WHERE data_source = 'grok_session' ORDER BY request_id",
            )
            .expect("prepare");
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .expect("query")
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    fn query_costs(db: &Database) -> Result<Vec<(String, String)>, AppError> {
        let conn = lock_conn!(db.conn);
        let mut stmt = conn
            .prepare(
                "SELECT request_id, total_cost_usd FROM proxy_request_logs
                 WHERE data_source = 'grok_session' ORDER BY created_at, request_id",
            )
            .expect("prepare");
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query")
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    fn append_and_bump(path: &Path, text: &str) {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("open for append");
        file.write_all(text.as_bytes()).expect("append");
        let later = SystemTime::now() + std::time::Duration::from_secs(5);
        file.set_times(std::fs::FileTimes::new().set_modified(later))
            .expect("bump mtime");
    }

    fn cursor_of(db: &Database, path: &Path) -> crate::services::session_usage::SyncCursor {
        crate::services::session_usage::load_sync_cursors(db)
            .unwrap()
            .get(path.to_string_lossy().as_ref())
            .copied()
            .expect("cursor")
    }

    #[test]
    fn appended_turns_are_read_from_the_byte_cursor() -> Result<(), AppError> {
        let db = Database::memory()?;
        let dir = tempdir().unwrap();
        let line = |prompt: &str| {
            usage_event_line(
                OLD_EPOCH,
                prompt,
                &model_counters("grok-4.5-build", 100, 10, 0, 1),
            )
        };
        let path = write_session_file(dir.path(), "s1", &[line("p1"), line("p2")]);
        assert_eq!(sync_file(&db, &path)?.imported, 2);

        // 尾行还没写完换行：照样解析，游标停在它前面
        append_and_bump(&path, &format!("{}\n{}", line("p3"), line("p4")));
        let read = read_usage_events_since(&path, Some(&cursor_of(&db, &path)))?;
        assert_eq!((read.first_index, read.events.len()), (2, 2));
        assert_eq!(read.committed_events, 1);
        assert_eq!(sync_file(&db, &path)?.imported, 2);
        assert_eq!(cursor_of(&db, &path).last_line_offset, 3);

        append_and_bump(&path, "\n");
        let read = read_usage_events_since(&path, Some(&cursor_of(&db, &path)))?;
        assert_eq!((read.first_index, read.events.len()), (3, 1));
        sync_file(&db, &path)?;
        assert_eq!(query_rows(&db)?.len(), 4);
        Ok(())
    }

    #[test]
    fn rewritten_file_is_read_again_without_double_counting() -> Result<(), AppError> {
        let db = Database::memory()?;
        let dir = tempdir().unwrap();
        let line = |prompt: &str| {
            usage_event_line(
                OLD_EPOCH,
                prompt,
                &model_counters("grok-4.5-build", 100, 10, 0, 1),
            )
        };
        let path = write_session_file(dir.path(), "s1", &[line("p1"), line("p2"), line("p3")]);
        assert_eq!(sync_file(&db, &path)?.imported, 3);

        // rewind 截掉最后一轮，再写新的一轮：文件内容在游标前就变了
        let rewound = write_session_file(dir.path(), "s1", &[line("p1"), line("p2"), line("p4")]);
        append_and_bump(&rewound, "");
        let read = read_usage_events_since(&rewound, Some(&cursor_of(&db, &rewound)))?;
        assert_eq!((read.first_index, read.events.len()), (0, 3));
        sync_file(&db, &rewound)?;

        let ids: Vec<String> = query_rows(&db)?.into_iter().map(|row| row.0).collect();
        assert_eq!(
            ids,
            ["p1", "p2", "p3", "p4"]
                .map(|p| format!("grok_session:{p}:grok-4.5-build"))
                .to_vec()
        );
        Ok(())
    }

    #[test]
    fn parses_turn_completed_and_ignores_noise_and_other_kinds() {
        let content = concat!(
            "{\"timestamp\":\"2026-07-20T13:26:10Z\",\"method\":\"session/update\",\"params\":{\"update\":{\"sessionUpdate\":\"agent_message_chunk\",\"content\":{}}}}\n",
            "not json at all\n",
            // 显式标为非 turn_completed 却带 usage：防中途快照双算，不得导入
            "{\"timestamp\":\"2026-07-20T13:26:20Z\",\"method\":\"_x.ai/session/update\",\"params\":{\"update\":{\"sessionUpdate\":\"usage_snapshot\",\"prompt_id\":\"px\",\"usage\":{\"inputTokens\":9999,\"outputTokens\":9,\"cachedReadTokens\":0}}}}\n",
            "{\"timestamp\":\"2026-07-20T13:26:24Z\",\"method\":\"_x.ai/session/update\",\"params\":{\"update\":{\"sessionUpdate\":\"turn_completed\",\"prompt_id\":\"p1\",\"usage\":{\"inputTokens\":16632,\"outputTokens\":104,\"cachedReadTokens\":0,\"modelUsage\":{\"grok-4.5-build\":{\"inputTokens\":16632,\"outputTokens\":104,\"cachedReadTokens\":0,\"apiDurationMs\":5342,\"costUsdTicks\":338880000}}}}}}\n",
        );
        let events = parse_grok_usage_events(content.as_bytes()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].prompt_id, "p1");
        assert_eq!(events[0].per_model.len(), 1);
        assert_eq!(events[0].per_model[0].0, "grok-4.5-build");
        assert_eq!(
            events[0].per_model[0].1,
            GrokCounters {
                input: 16632,
                output: 104,
                cached: 0,
                cache_write: 0,
                api_ms: 5342,
                model_calls: 0,
                cost_ticks: 338_880_000,
            }
        );
    }

    #[test]
    fn missing_model_usage_falls_back_to_top_level_counters() {
        // 同时覆盖：sessionUpdate 字段缺失时向后兼容放行
        let line = format!(
            r#"{{"timestamp":"{}","method":"_x.ai/session/update","params":{{"update":{{"prompt_id":"p1","usage":{{"inputTokens":100,"outputTokens":10,"cachedReadTokens":5}}}}}}}}"#,
            epoch_to_rfc3339(OLD_EPOCH)
        );
        let events = parse_grok_usage_events(line.as_bytes()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].per_model[0].0, "unknown");
        assert_eq!(events[0].per_model[0].1.input, 100);
    }

    #[test]
    fn two_turns_import_at_face_value_matching_reported_ticks() -> Result<(), AppError> {
        use std::str::FromStr;
        // 2026-07-23 单进程双 prompt 实测原值：turn_completed 是逐轮独立总量。
        // 若误用相邻差分，第二轮会被记成 53/28/6144 的假增量（曾犯）。
        // 每轮 ticks 同时钉死逐轮口径与 2/6/0.30 定价：
        //   轮1 (17294-11136)×2 + 11136×0.30 + 28×6 = 15824.8 µUSD = 158248000 ticks
        //   轮2 (17347-17280)×2 + 17280×0.30 + 56×6 =  5654.0 µUSD =  56540000 ticks
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let lines = vec![
            usage_event_line(
                OLD_EPOCH,
                "p1",
                &model_counters_with_ticks("grok-4.5-build", 17294, 28, 11136, 1, 158_248_000),
            ),
            usage_event_line(
                OLD_EPOCH + 60,
                "p2",
                &model_counters_with_ticks("grok-4.5-build", 17347, 56, 17280, 1, 56_540_000),
            ),
        ];
        let path = write_session_file(temp.path(), "sess-two-turns", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 2);
        assert_eq!(result.deferred_files, 0);

        let rows = query_rows(&db)?;
        assert_eq!(rows.len(), 2);
        // input_tokens 入库的是未命中缓存的输入（inputTokens − cachedReadTokens）
        assert_eq!((rows[0].1, rows[0].2, rows[0].3), (6158, 28, 11136));
        assert_eq!((rows[1].1, rows[1].2, rows[1].3), (67, 56, 17280));
        assert!(rows.iter().all(|r| r.4 == INPUT_TOKEN_SEMANTICS_FRESH));

        // 本地定价复算须与 CLI 自报 ticks 分毫不差（漂移告警在此阈值内静默）
        let costs = query_costs(&db)?;
        let expected1 = Decimal::from(158_248_000u64) / Decimal::from(10_000_000_000u64);
        let expected2 = Decimal::from(56_540_000u64) / Decimal::from(10_000_000_000u64);
        assert_eq!(Decimal::from_str(&costs[0].1).expect("decimal"), expected1);
        assert_eq!(Decimal::from_str(&costs[1].1).expect("decimal"), expected2);
        Ok(())
    }

    #[test]
    fn second_turn_with_smaller_counters_imports_at_face_value() -> Result<(), AppError> {
        // 2026-07-23 跨进程实测原值（进程 A 单轮 27386/74/15360，--resume 的
        // 进程 B 单轮 13793/21/13696）。逐轮口径下"第二轮更小"是常态，
        // 与是否跨进程无关，一律按面值入账。
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let lines = vec![
            usage_event_line(
                OLD_EPOCH,
                "p1",
                &model_counters("grok-4.5-build", 27386, 74, 15360, 2),
            ),
            usage_event_line(
                OLD_EPOCH + 15,
                "p2",
                &model_counters("grok-4.5-build", 13793, 21, 13696, 1),
            ),
        ];
        let path = write_session_file(temp.path(), "sess-resume", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 2);

        let rows = query_rows(&db)?;
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].1, rows[0].2, rows[0].3), (12026, 74, 15360));
        assert_eq!((rows[1].1, rows[1].2, rows[1].3), (97, 21, 13696));
        Ok(())
    }

    #[test]
    fn identical_turns_both_import() -> Result<(), AppError> {
        // 回归（逐轮口径）：两轮数值完全相同 = 两笔真实用量，都必须入账。
        // 差分口径会把第二轮当零增量整轮跳过——那正是被证伪的旧行为。
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let lines = vec![
            usage_event_line(
                OLD_EPOCH,
                "p1",
                &model_counters("grok-4.5-build", 100, 10, 0, 1),
            ),
            usage_event_line(
                OLD_EPOCH + 60,
                "p2",
                &model_counters("grok-4.5-build", 100, 10, 0, 1),
            ),
        ];
        let path = write_session_file(temp.path(), "sess-identical", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 2, "相同数值的两轮都是真实用量");
        assert_eq!(query_rows(&db)?.len(), 2);
        Ok(())
    }

    #[test]
    fn multi_model_event_produces_row_per_model() -> Result<(), AppError> {
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let both = format!(
            "{},{}",
            model_counters("grok-4.5-build", 100, 10, 0, 1),
            model_counters("grok-4.3", 30, 3, 0, 1)
        );
        let lines = vec![usage_event_line(OLD_EPOCH, "p1", &both)];
        let path = write_session_file(temp.path(), "sess-multi", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 2);
        let rows = query_rows(&db)?;
        assert!(rows[0].0.ends_with(":grok-4.3"));
        assert!(rows[1].0.ends_with(":grok-4.5-build"));
        Ok(())
    }

    #[test]
    fn recent_events_are_imported_and_sync_state_is_recorded() -> Result<(), AppError> {
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("now")
            .as_secs() as i64;
        let lines = vec![usage_event_line(
            now,
            "p1",
            &model_counters("grok-4.5-build", 250, 30, 0, 1),
        )];
        let path = write_session_file(temp.path(), "sess-recent", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 1);
        assert_eq!(result.deferred_files, 0);

        let (last_modified, _) = get_sync_state(&db, &path.to_string_lossy())?;
        assert_ne!(last_modified, 0, "导入后记录同步状态");
        Ok(())
    }

    #[test]
    fn rescan_is_idempotent() -> Result<(), AppError> {
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let lines = vec![
            usage_event_line(
                OLD_EPOCH,
                "p1",
                &model_counters("grok-4.5-build", 100, 10, 0, 1),
            ),
            usage_event_line(
                OLD_EPOCH + 60,
                "p2",
                &model_counters("grok-4.5-build", 250, 30, 50, 1),
            ),
        ];
        let path = write_session_file(temp.path(), "sess-idem", &lines);

        let first = sync_file(&db, &path)?;
        assert_eq!(first.imported, 2);

        // mtime 未变 → 短路
        let second = sync_file(&db, &path)?;
        assert_eq!(second.imported + second.skipped, 0);

        // 强制重读（清同步状态）→ UPSERT 全部无变化
        {
            let conn = lock_conn!(db.conn);
            conn.execute("DELETE FROM session_log_sync", [])?;
        }
        let third = sync_file(&db, &path)?;
        assert_eq!(third.imported, 0);
        assert_eq!(third.skipped, 2);
        assert_eq!(query_rows(&db)?.len(), 2);
        Ok(())
    }

    #[test]
    fn rewind_truncation_does_not_double_count() -> Result<(), AppError> {
        // 回归（对比评审发现）：幂等键若含文件内序号，updates.jsonl 前缀被
        // 改写（rewind 截断）后幸存事件序号前移会生成新 request_id 造成双算。
        // prompt_id 锚定键下：幸存轮命中原行；被移除轮的行保留（rewind 不
        // 退还已消耗 token，留存即正确）。
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let full = vec![
            usage_event_line(
                OLD_EPOCH,
                "p1",
                &model_counters("grok-4.5-build", 100, 10, 0, 1),
            ),
            usage_event_line(
                OLD_EPOCH + 60,
                "p2",
                &model_counters("grok-4.5-build", 200, 20, 0, 1),
            ),
            usage_event_line(
                OLD_EPOCH + 120,
                "p3",
                &model_counters("grok-4.5-build", 300, 30, 0, 1),
            ),
        ];
        let path = write_session_file(temp.path(), "sess-rewind", &full);
        assert_eq!(sync_file(&db, &path)?.imported, 3);

        // 模拟 rewind 截掉 p2：p3 从 idx2 前移到 idx1
        let truncated = vec![full[0].clone(), full[2].clone()];
        write_session_file(temp.path(), "sess-rewind", &truncated);
        {
            let conn = lock_conn!(db.conn);
            conn.execute("DELETE FROM session_log_sync", [])?;
        }

        let rescan = sync_file(&db, &path)?;
        assert_eq!(rescan.imported, 0, "幸存轮不得因序号前移重新入账");

        let rows = query_rows(&db)?;
        assert_eq!(rows.len(), 3, "被截掉轮次的行保留（token 已实际消耗）");
        let p3: Vec<_> = rows.iter().filter(|r| r.0.contains(":p3:")).collect();
        assert_eq!(p3.len(), 1);
        assert_eq!(p3[0].1, 300);
        Ok(())
    }

    #[test]
    fn empty_prompt_id_falls_back_to_index_key() -> Result<(), AppError> {
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let lines = vec![usage_event_line(
            OLD_EPOCH,
            "",
            &model_counters("grok-4.5-build", 100, 10, 0, 1),
        )];
        let path = write_session_file(temp.path(), "sess-noprompt", &lines);

        assert_eq!(sync_file(&db, &path)?.imported, 1);
        let rows = query_rows(&db)?;
        assert!(rows[0].0.contains(":idx0:"), "空 prompt_id 回退序号键");
        Ok(())
    }

    #[test]
    fn cost_matches_cli_reported_ticks_for_seeded_grok45_build() -> Result<(), AppError> {
        use std::str::FromStr;
        // 真实样本：inputTokens=16632, outputTokens=104, cache=0,
        // costUsdTicks=338880000（1 tick = 1e-10 USD）。seed 的 grok-4.5-build
        // 定价（2/6）应精确复现 CLI 自报成本。fixture 故意不带 ticks，
        // 验证的是本地定价独立复算。
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let lines = vec![usage_event_line(
            OLD_EPOCH,
            "p1",
            &model_counters("grok-4.5-build", 16632, 104, 0, 1),
        )];
        let path = write_session_file(temp.path(), "sess-ticks", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 1);

        let conn = lock_conn!(db.conn);
        let total: String = conn.query_row(
            "SELECT total_cost_usd FROM proxy_request_logs WHERE data_source = 'grok_session'",
            [],
            |row| row.get(0),
        )?;
        let expected = Decimal::from(338_880_000u64) / Decimal::from(10_000_000_000u64);
        assert_eq!(Decimal::from_str(&total).expect("decimal"), expected);
        Ok(())
    }

    #[test]
    fn cost_matches_cli_reported_ticks_with_cache_reads() -> Result<(), AppError> {
        use std::str::FromStr;
        // 2026-07-23 实测带缓存样本：13793/21/13696，costUsdTicks=44288000。
        // 钉死 cache read 实测单价 0.30：billable_input=(13793-13696)×2/1M
        // + 21×6/1M + 13696×0.30/1M = 0.0044288。seed 若改回 0.50 此测试即红。
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let lines = vec![usage_event_line(
            OLD_EPOCH,
            "p1",
            &model_counters("grok-4.5-build", 13793, 21, 13696, 1),
        )];
        let path = write_session_file(temp.path(), "sess-ticks-cache", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 1);

        let conn = lock_conn!(db.conn);
        let total: String = conn.query_row(
            "SELECT total_cost_usd FROM proxy_request_logs WHERE data_source = 'grok_session'",
            [],
            |row| row.get(0),
        )?;
        let expected = Decimal::from(44_288_000u64) / Decimal::from(10_000_000_000u64);
        assert_eq!(Decimal::from_str(&total).expect("decimal"), expected);
        Ok(())
    }

    #[test]
    fn reported_ticks_override_stale_local_pricing() -> Result<(), AppError> {
        use std::str::FromStr;
        // 定价漂移窗口：CLI 自报为本地复算（338880000 ticks）的两倍，模拟
        // xAI 调价而 seed 未更新。total 必须以自报为准（回填不修正正值行，
        // 本地价错就永久错）；分项仍按本地价（暂不自洽，有漂移告警提示）。
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let lines = vec![usage_event_line(
            OLD_EPOCH,
            "p1",
            &model_counters_with_ticks("grok-4.5-build", 16632, 104, 0, 1, 677_760_000),
        )];
        let path = write_session_file(temp.path(), "sess-drift", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 1);

        let conn = lock_conn!(db.conn);
        let (input_cost, total): (String, String) = conn.query_row(
            "SELECT input_cost_usd, total_cost_usd FROM proxy_request_logs
             WHERE data_source = 'grok_session'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let expected_total = Decimal::from(677_760_000u64) / Decimal::from(10_000_000_000u64);
        assert_eq!(
            Decimal::from_str(&total).expect("decimal"),
            expected_total,
            "total 以自报为准"
        );
        assert!(
            Decimal::from_str(&input_cost).expect("decimal") > Decimal::ZERO,
            "分项仍按本地定价"
        );
        Ok(())
    }

    #[test]
    fn cache_writes_are_split_out_of_input() -> Result<(), AppError> {
        use std::str::FromStr;
        // inputTokens 含缓存读和缓存写（grok-build: cache_creation_tokens 是
        // input_tokens 的子集）
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let counters = r#""grok-4.5-build":{"inputTokens":1000,"outputTokens":10,"cachedReadTokens":300,"cacheCreationTokens":200,"modelCalls":1}"#;
        let lines = vec![usage_event_line(OLD_EPOCH, "p1", counters)];
        let path = write_session_file(temp.path(), "sess-cache-write", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 1);

        let conn = lock_conn!(db.conn);
        let tokens: (i64, i64, i64) = conn.query_row(
            "SELECT input_tokens, cache_read_tokens, cache_creation_tokens
             FROM proxy_request_logs WHERE data_source = 'grok_session'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(tokens, (500, 300, 200));

        // xAI 没有单独的缓存写价，缓存写按输入价 $2/M 计，不能记成免费
        let costs: (String, String) = conn.query_row(
            "SELECT cache_creation_cost_usd, total_cost_usd
             FROM proxy_request_logs WHERE data_source = 'grok_session'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let dec = |v: &str| Decimal::from_str(v).expect("decimal");
        assert_eq!(dec(&costs.0), dec("0.0004"));
        // 500×2 + 300×0.30 + 200×2 + 10×6，单位 $/M
        assert_eq!(dec(&costs.1), dec("0.00155"));
        Ok(())
    }

    #[test]
    fn unpriced_model_falls_back_to_reported_ticks() -> Result<(), AppError> {
        use std::str::FromStr;
        // 未 seed 的新别名：total_cost 采用 CLI 自报 ticks（分项记 0），
        // 不再整单记 0。
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let lines = vec![usage_event_line(
            OLD_EPOCH,
            "p1",
            &model_counters_with_ticks("grok-6-future-alias", 1000, 100, 0, 1, 56_540_000),
        )];
        let path = write_session_file(temp.path(), "sess-unpriced", &lines);

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 1);

        let conn = lock_conn!(db.conn);
        let (input_cost, total): (String, String) = conn.query_row(
            "SELECT input_cost_usd, total_cost_usd FROM proxy_request_logs
             WHERE data_source = 'grok_session'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(
            Decimal::from_str(&input_cost).expect("decimal"),
            Decimal::ZERO
        );
        let expected = Decimal::from(56_540_000u64) / Decimal::from(10_000_000_000u64);
        assert_eq!(Decimal::from_str(&total).expect("decimal"), expected);
        Ok(())
    }

    #[test]
    fn symlink_cycle_does_not_cause_stack_overflow() {
        let temp = tempdir().expect("tempdir");
        let sessions = temp.path().join("sessions");
        let enc = sessions.join("enc-project");
        let sub = enc.join("sub");
        std::fs::create_dir_all(&sub).expect("create dirs");

        // 构造循环：sub/cycle -> enc 父目录
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&enc, sub.join("cycle"));
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&enc, sub.join("cycle"));
        if let Err(err) = linked {
            // Windows 在 \\wsl.localhost 上建不了符号链接（Incorrect function），夹具无从构造
            assert!(crate::config::is_wsl_path(temp.path()), "symlink: {err}");
            eprintln!("cannot create symlinks on WSL share ({err}); skipping");
            return;
        }

        // 也放一个真实的目标文件，确认正常遍历仍工作
        std::fs::write(enc.join("updates.jsonl"), b"{}\n").expect("write real file");

        let mut files = Vec::new();
        collect_files_named(&sessions, "updates.jsonl", &mut files, 0);

        assert_eq!(
            files.len(),
            1,
            "only the real updates.jsonl should be collected; symlink cycle must not crash"
        );
    }

    fn write_summary(updates_path: &Path, json: &str) {
        std::fs::write(updates_path.with_file_name("summary.json"), json).expect("write summary");
    }

    fn row_ids(db: &Database) -> Vec<String> {
        let mut ids: Vec<String> = query_rows(db)
            .expect("query rows")
            .into_iter()
            .map(|row| row.0)
            .collect();
        ids.sort();
        ids
    }

    #[test]
    fn subagent_sessions_are_not_imported() -> Result<(), AppError> {
        // 子代理用量已并入父会话当轮的 turn_completed（本机实测：父会话各轮
        // modelCalls - numTurns 之和 311 = 子代理 modelCalls 之和 311）
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let line = usage_event_line(
            OLD_EPOCH,
            "p1",
            &model_counters("grok-4.5-build", 100, 10, 0, 1),
        );
        let path = write_session_file(temp.path(), "sess-sub", &[line]);
        write_summary(
            &path,
            r#"{"info":{"id":"sess-sub"},"session_kind":"subagent"}"#,
        );

        let result = sync_file(&db, &path)?;
        assert_eq!(result.imported, 0);
        assert!(row_ids(&db).is_empty());
        Ok(())
    }

    #[test]
    fn fork_skips_turns_copied_from_its_source_session() -> Result<(), AppError> {
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let counters = model_counters("grok-4.5-build", 100, 10, 0, 1);
        let parent = write_session_file(
            temp.path(),
            "sess-parent",
            &[
                usage_event_line(OLD_EPOCH, "p1", &counters),
                usage_event_line(OLD_EPOCH + 60, "p2", &counters),
            ],
        );
        // fork 复制源会话的事件（prompt_id 不变），之后追加自己的轮次
        let fork = write_session_file(
            temp.path(),
            "sess-fork",
            &[
                usage_event_line(OLD_EPOCH + 120, "p1", &counters),
                usage_event_line(OLD_EPOCH + 120, "p2", &counters),
                usage_event_line(OLD_EPOCH + 180, "p3", &counters),
            ],
        );
        write_summary(
            &fork,
            r#"{"info":{"id":"sess-fork"},"session_kind":"fork","parent_session_id":"sess-parent"}"#,
        );
        let files: HashMap<&str, &Path> = HashMap::from([
            ("sess-parent", parent.as_path()),
            ("sess-fork", fork.as_path()),
        ]);
        let cursors = crate::services::session_usage::load_sync_cursors(&db).unwrap();

        // 先同步 fork：源会话还没入库，靠源会话日志里的 prompt_id 识别
        let forked = sync_single_grok_file(&db, &fork, &cursors, &files)?;
        assert_eq!((forked.imported, forked.skipped), (1, 2));
        sync_single_grok_file(&db, &parent, &cursors, &files)?;

        assert_eq!(
            row_ids(&db),
            vec![
                "grok_session:p1:grok-4.5-build",
                "grok_session:p2:grok-4.5-build",
                "grok_session:p3:grok-4.5-build",
            ]
        );
        Ok(())
    }

    #[test]
    fn fork_of_a_deleted_session_skips_turns_already_recorded_for_it() -> Result<(), AppError> {
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let counters = model_counters("grok-4.5-build", 100, 10, 0, 1);
        let parent = write_session_file(
            temp.path(),
            "sess-gone",
            &[usage_event_line(OLD_EPOCH, "p1", &counters)],
        );
        sync_file(&db, &parent)?;
        std::fs::remove_dir_all(parent.parent().unwrap()).expect("delete source session");

        let fork = write_session_file(
            temp.path(),
            "sess-fork",
            &[
                usage_event_line(OLD_EPOCH + 120, "p1", &counters),
                usage_event_line(OLD_EPOCH + 180, "p2", &counters),
            ],
        );
        write_summary(
            &fork,
            r#"{"info":{"id":"sess-fork"},"session_kind":"fork","parent_session_id":"sess-gone"}"#,
        );

        sync_file(&db, &fork)?;
        assert_eq!(
            row_ids(&db),
            vec![
                "grok_session:p1:grok-4.5-build",
                "grok_session:p2:grok-4.5-build",
            ]
        );
        Ok(())
    }

    #[test]
    fn fork_of_a_fork_skips_turns_after_the_middle_session_is_deleted() -> Result<(), AppError> {
        let db = Database::memory()?;
        let temp = tempdir().expect("tempdir");
        let counters = model_counters("grok-4.5-build", 100, 10, 0, 1);
        let line = |at: i64, prompt: &str| usage_event_line(at, prompt, &counters);
        let a = write_session_file(temp.path(), "sess-a", &[line(OLD_EPOCH, "p1")]);
        sync_file(&db, &a)?;
        // B 是 A 的 fork，C 是 B 的 fork；同步 C 时 B 已删除
        let c = write_session_file(
            temp.path(),
            "sess-c",
            &[line(OLD_EPOCH + 60, "p1"), line(OLD_EPOCH + 120, "p2")],
        );
        write_summary(
            &c,
            r#"{"info":{"id":"sess-c"},"session_kind":"fork","parent_session_id":"sess-b"}"#,
        );

        let result = sync_file(&db, &c)?;
        assert_eq!((result.imported, result.skipped), (1, 1));
        let owners: Vec<(String, String)> = lock_conn!(db.conn)
            .prepare("SELECT request_id, session_id FROM proxy_request_logs ORDER BY request_id")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        assert_eq!(
            owners,
            vec![
                (
                    "grok_session:p1:grok-4.5-build".to_string(),
                    "sess-a".to_string()
                ),
                (
                    "grok_session:p2:grok-4.5-build".to_string(),
                    "sess-c".to_string()
                ),
            ]
        );
        Ok(())
    }
}
