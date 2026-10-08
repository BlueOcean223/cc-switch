//! Gemini CLI 会话：列表、阅读视图与删除。
//!
//! 会话在 `~/.gemini/tmp/<项目目录>/chats/` 下：2026-04-09 之前是整份 JSON 的
//! `session-*.json`，之后是追加写的 `session-*.jsonl`（gemini-cli PR #23749），读取与回放见
//! [`jsonl`]。项目目录从 sha256 hash 改成了 slug（映射在 `~/.gemini/projects.json`），旧 hash
//! 目录只被复制、不会删除；恢复旧 `.json` 会话时又会在旁边新建同名 `.jsonl`。所以同一会话
//! 可能有好几份文件，列表按 `(projectHash, sessionId)` 合并成一项。

pub(crate) mod jsonl;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::UNIX_EPOCH;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::session_manager::model::{
    ContentRef, DiffOp, EventKind, ImageRef, ImageSource, MessageMeta, SessionBlock, ToolStatus,
};
use crate::session_manager::{SessionMessage, SessionMeta};

use self::jsonl::{ReplayedMessage, Src};
use super::blocks::{
    assign_turn_ids, count_diff_lines, estimate_base64_size, large_text_block, single_file_diff,
    thinking_block, tool_call_block, tool_result_block, ToolSource,
};
use super::utils::{parse_timestamp_to_ms, truncate_summary, FileParseCache};

const PROVIDER_ID: &str = "gemini";
/// 列表标题的最大字符数
const TITLE_CHARS: usize = 160;

/// 会话页每次打开都会全量扫描；没变过的文件直接复用上次的解析结果。
static PARSE_CACHE: LazyLock<FileParseCache> = LazyLock::new(FileParseCache::new);

pub fn scan_sessions() -> Vec<SessionMeta> {
    scan_sessions_in(&PARSE_CACHE, &crate::gemini_config::get_gemini_dir())
}

fn scan_sessions_in(cache: &FileParseCache, gemini_dir: &Path) -> Vec<SessionMeta> {
    let tmp_dir = gemini_dir.join("tmp");
    let files = collect_session_files(&tmp_dir);
    let parsed = cache.scan(files, scan_session_file);
    merge_copies(parsed, &tmp_dir, &gemini_dir.join("projects.json"))
}

/// `tmp/*/chats/` 下的 `session-*.json` 与 `session-*.jsonl`（官方列表也只认这些）。
/// 子代理在 `chats/<主会话 id>/` 子目录里，不收；`*.jsonl.tmp-*`、`*.jsonl.unreadable-*`
/// 这类临时或改名保留的文件按后缀排除。
fn collect_session_files(tmp_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(projects) = std::fs::read_dir(tmp_dir) else {
        return files;
    };
    for project in projects.flatten() {
        let Ok(entries) = std::fs::read_dir(project.path().join("chats")) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_session = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("session-")
                        && (name.ends_with(".json") || name.ends_with(".jsonl"))
                });
            if is_session && path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// 列表扫描用（结果进 [`FileParseCache`]，只能取决于文件本身）。子代理和没有可恢复消息
/// 的会话不列出，与官方一致。`project_dir` 先暂存元数据里的 projectHash，取出缓存后由
/// [`merge_copies`] 换成项目路径。
fn scan_session_file(path: &Path) -> std::io::Result<Option<SessionMeta>> {
    let Some(header) = jsonl::scan_header(path)? else {
        return Ok(None);
    };
    if header.meta.is_subagent() || !header.has_resumable {
        return Ok(None);
    }
    let jsonl::SessionHeader {
        meta,
        first_user_text,
        ..
    } = header;
    let Some(session_id) = meta.session_id else {
        return Ok(None);
    };

    let modified = std::fs::metadata(path)?
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_millis() as i64);
    let timestamp =
        |value: Option<String>| value.and_then(|v| parse_timestamp_to_ms(&Value::String(v)));
    let created_at = timestamp(meta.start_time);
    // 应用所有 `$set` 后的 lastUpdated；缺失时用文件修改时间（官方用当前时间兜底）
    let last_active_at = timestamp(meta.last_updated).or(modified).or(created_at);
    // 官方列表的标题：有 AI 生成的 summary 用 summary，否则用第一条可恢复的用户消息
    let title = meta
        .summary
        .filter(|summary| !summary.trim().is_empty())
        .or(first_user_text)
        .map(|text| truncate_summary(&text, TITLE_CHARS))
        .filter(|text| !text.is_empty());

    Ok(Some(SessionMeta {
        provider_id: PROVIDER_ID.to_string(),
        resume_command: Some(format!("gemini --resume {session_id}")),
        session_id,
        title: title.clone(),
        summary: title,
        project_dir: meta.project_hash,
        created_at,
        last_active_at,
        source_path: Some(path.to_string_lossy().into_owned()),
    }))
}

/// 同一会话的多份文件（旧 hash 目录与 slug 目录各一份、旧 `.json` 与迁移出的 `.jsonl`）
/// 按 `(projectHash, sessionId)` 合并成一项：取 `lastUpdated` 最新的一份；相同时优先
/// `.jsonl`（迁移后内容是超集），再相同时优先所在目录有 `.project_root` 的那份（slug 目录）。
///
/// 同时把 `project_dir`（扫描时暂存的 projectHash）换成项目路径：先读同级 `.project_root`，
/// 没有时按 projectHash 反查。
fn merge_copies(
    parsed: Vec<SessionMeta>,
    tmp_dir: &Path,
    projects_json: &Path,
) -> Vec<SessionMeta> {
    struct FileCopy {
        meta: SessionMeta,
        hash: Option<String>,
        project_root: Option<String>,
        jsonl: bool,
    }
    fn rank(copy: &FileCopy) -> (Option<i64>, bool, bool) {
        (
            copy.meta.last_active_at,
            copy.jsonl,
            copy.project_root.is_some(),
        )
    }

    let mut roots: HashMap<PathBuf, Option<String>> = HashMap::new();
    let mut slots: HashMap<(String, String), usize> = HashMap::new();
    let mut kept: Vec<FileCopy> = Vec::new();
    for mut meta in parsed {
        let hash = meta.project_dir.take();
        let path = PathBuf::from(meta.source_path.as_deref().unwrap_or_default());
        let project_root = path.parent().and_then(Path::parent).and_then(|dir| {
            roots
                .entry(dir.to_path_buf())
                .or_insert_with(|| read_project_root(dir))
                .clone()
        });
        let key = (hash.clone().unwrap_or_default(), meta.session_id.clone());
        let copy = FileCopy {
            jsonl: jsonl::is_jsonl(&path),
            meta,
            hash,
            project_root,
        };
        match slots.get(&key) {
            Some(&index) => {
                if rank(&copy) > rank(&kept[index]) {
                    kept[index] = copy;
                }
            }
            None => {
                slots.insert(key, kept.len());
                kept.push(copy);
            }
        }
    }

    let needs_lookup = kept
        .iter()
        .any(|copy| copy.project_root.is_none() && copy.hash.is_some());
    let by_hash = if needs_lookup {
        project_paths_by_hash(tmp_dir, projects_json)
    } else {
        HashMap::new()
    };
    kept.into_iter()
        .map(|copy| SessionMeta {
            project_dir: copy
                .project_root
                .or_else(|| copy.hash.and_then(|hash| by_hash.get(&hash).cloned())),
            ..copy.meta
        })
        .collect()
}

/// 项目目录下的 `.project_root`：项目绝对路径（官方写入时不带换行，这里仍 trim 一下）
fn read_project_root(project_dir: &Path) -> Option<String> {
    std::fs::read_to_string(project_dir.join(".project_root"))
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// projectHash（项目绝对路径的 sha256）→ 项目路径。候选路径取 `projects.json` 的 key
/// 和各项目目录 `.project_root` 的内容，用来给没有 `.project_root` 的旧 hash 目录找回项目路径。
fn project_paths_by_hash(tmp_dir: &Path, projects_json: &Path) -> HashMap<String, String> {
    let mut paths = Vec::new();
    if let Ok(text) = std::fs::read_to_string(projects_json) {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            if let Some(projects) = value.get("projects").and_then(Value::as_object) {
                paths.extend(projects.keys().cloned());
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(tmp_dir) {
        paths.extend(
            entries
                .flatten()
                .filter_map(|e| read_project_root(&e.path())),
        );
    }
    paths
        .into_iter()
        .map(|path| (sha256_hex(&path), path))
        .collect()
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

// ─── 阅读视图 ────────────────────────────────────────────────────────────

/// 读取会话：`.jsonl` 按官方算法回放后渲染，看到的是回退、移除、压缩之后的状态
/// （与 `gemini --resume` 一致）；`.json` 整份解析。
pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let replayed = jsonl::replay(path)?;
    let refs = Refs {
        rel_path: relative_to_tmp_root(path),
    };
    Ok(render_messages(&replayed.messages, &refs))
}

/// 生成全文引用：JSONL 指向某一行 + 行内 JSON Pointer；旧格式整份 JSON 用相对 `tmp/` 的
/// 文件路径 + 文件内 JSON Pointer。
struct Refs {
    rel_path: String,
}

impl Refs {
    fn at(&self, src: &Src, sub: &str) -> ContentRef {
        let pointer = format!("{}{sub}", src.pointer);
        match src.span {
            Some(span) => span.content_ref(pointer),
            None => ContentRef::File {
                rel_path: self.rel_path.clone(),
                pointer,
            },
        }
    }
}

/// 一条消息里各字段的引用；被 `$patch` 替换过的字段指向补丁行。
struct MsgRefs<'a> {
    refs: &'a Refs,
    msg: &'a ReplayedMessage,
}

impl MsgRefs<'_> {
    /// 消息内的字段，如 `/thoughts`、`/toolCalls/0/args`
    fn field(&self, sub: &str) -> ContentRef {
        self.refs.at(&self.msg.src, sub)
    }

    /// `content` 内的位置（`sub` 相对 content）
    fn content(&self, sub: &str) -> ContentRef {
        match &self.msg.content_src {
            Some(src) => self.refs.at(src, sub),
            None => self.refs.at(&self.msg.src, &format!("/content{sub}")),
        }
    }

    /// 第 `index` 个工具调用的 `result` 内的位置（`sub` 相对 result）
    fn tool_result(&self, index: usize, call_id: Option<&str>, sub: &str) -> ContentRef {
        match call_id.and_then(|id| self.msg.tool_result_src.get(id)) {
            Some(src) => self.refs.at(src, sub),
            None => self
                .refs
                .at(&self.msg.src, &format!("/toolCalls/{index}/result{sub}")),
        }
    }
}

fn render_messages(messages: &[ReplayedMessage], refs: &Refs) -> Vec<SessionMessage> {
    // 回放后仍在的工具调用 id：工具结果消息里的结果都能对上调用时，已经显示在调用下面
    let call_ids: HashSet<&str> = messages
        .iter()
        .filter(|m| m.value.get("type").and_then(Value::as_str) == Some("gemini"))
        .flat_map(|m| m.value.get("toolCalls").and_then(Value::as_array))
        .flatten()
        .filter_map(|call| call.get("id").and_then(Value::as_str))
        .collect();

    // 合成的 gemini 消息没有 model：沿用前面最近一条消息的模型
    let mut last_model: Option<&str> = None;
    let mut result = Vec::new();
    for msg in messages {
        let value = &msg.value;
        let refs = MsgRefs { refs, msg };
        let ts = value.get("timestamp").and_then(parse_timestamp_to_ms);
        let message = match value.get("type").and_then(Value::as_str) {
            Some("user") => user_message(&refs, ts, &call_ids),
            Some("gemini") => {
                if let Some(model) = value
                    .get("model")
                    .and_then(Value::as_str)
                    .filter(|model| !model.is_empty())
                {
                    last_model = Some(model);
                }
                let mut message =
                    SessionMessage::from_blocks("assistant", ts, gemini_blocks(&refs));
                message.meta = gemini_meta(value, last_model);
                Some(message)
            }
            Some(kind @ ("info" | "error" | "warning")) => Some(event_message(&refs, ts, kind)),
            _ => None,
        };
        let Some(mut message) = message.filter(|message| !message.is_empty()) else {
            continue;
        };
        message.id = value.get("id").and_then(Value::as_str).map(str::to_string);
        result.push(message);
    }

    assign_turn_ids(&mut result);
    result
}

/// `type=user`：
/// - 工具结果（2026-05-18 起发回模型的结果也记成 user 消息）：结果都能对上回放后的某个
///   调用时不单独显示，对不上的渲染为 ToolResult；
/// - `<session_context>` / `<hook_context>` 开头的注入上下文：标为 injected；
/// - 普通提问：文本优先 `displayContent`（`@文件` 引用时 content 里还带着文件内容），
///   content 里的图片与其他附件附在后面。
fn user_message(
    refs: &MsgRefs,
    ts: Option<i64>,
    call_ids: &HashSet<&str>,
) -> Option<SessionMessage> {
    let content = refs.msg.value.get("content");
    let parts = part_list(content);
    if is_tool_result(&parts) {
        let blocks: Vec<SessionBlock> = parts
            .iter()
            .enumerate()
            .filter(|(_, (part, _))| {
                part.get("functionResponse").is_some()
                    && !part
                        .pointer("/functionResponse/id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| call_ids.contains(id))
            })
            .map(|(index, _)| unmatched_result_block(refs, &parts, index))
            .collect();
        return (!blocks.is_empty()).then(|| SessionMessage::from_blocks("tool", ts, blocks));
    }

    let text = parts_text(content);
    if is_injected(&text) {
        let full = single_text_ref(content, |sub| refs.content(sub));
        let mut message =
            SessionMessage::from_blocks("user", ts, vec![large_text_block(text, || full)]);
        message.injected = true;
        return Some(message);
    }

    let display = parts_text(refs.msg.value.get("displayContent"));
    let text = if display.trim().is_empty() {
        text
    } else {
        display
    };
    let mut blocks = Vec::new();
    if !text.trim().is_empty() {
        blocks.push(SessionBlock::text(text));
    }
    blocks.extend(media_blocks(&parts, |sub| refs.content(sub)));
    Some(SessionMessage::from_blocks("user", ts, blocks))
}

/// `info` / `error` / `warning`：content 一般是字符串；读到音频、视频时的二进制注入是
/// inlineData 数组。info（登录、刷新等提示）默认折叠，warning 与 error 照常显示。
fn event_message(refs: &MsgRefs, ts: Option<i64>, kind: &str) -> SessionMessage {
    let content = refs.msg.value.get("content");
    let text = parts_text(content);
    let event_kind = match kind {
        "info" => EventKind::Info,
        "error" => EventKind::Error,
        _ => EventKind::Other,
    };
    let mut blocks = Vec::new();
    if !text.trim().is_empty() {
        blocks.push(SessionBlock::event(event_kind, Some(text), None));
    }
    blocks.extend(media_blocks(&part_list(content), |sub| refs.content(sub)));
    let mut message = SessionMessage::from_blocks("system", ts, blocks);
    message.injected = kind == "info";
    message
}

/// `type=gemini` 消息：思考（多条合并为一块）→ 正文 → 工具调用与结果（按生成顺序）。
fn gemini_blocks(refs: &MsgRefs) -> Vec<SessionBlock> {
    let msg = &refs.msg.value;
    let mut blocks = Vec::new();

    if let Some(thoughts) = msg.get("thoughts").and_then(format_thoughts) {
        // 合并后的正文与取回的全文同一口径：引用指向整个数组，由 `content::resolve_content_ref`
        // 按 `format_thoughts` 格式化
        blocks.push(thinking_block(
            &thoughts.text,
            (!thoughts.summary.is_empty()).then_some(thoughts.summary),
            None,
            || Some(refs.field("/thoughts")),
        ));
    }

    // 写入时一般是字符串；被 `$patch` 替换后可能是 Part 数组（跳过 thought 与 functionCall）
    let content = msg.get("content");
    let text = parts_text(content);
    if !text.trim().is_empty() {
        blocks.push(SessionBlock::text(text));
    }
    blocks.extend(media_blocks(&part_list(content), |sub| refs.content(sub)));

    for (j, call) in msg
        .get("toolCalls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let Some(name) = call.get("name").and_then(Value::as_str) else {
            continue;
        };
        let call_base = format!("/toolCalls/{j}");
        let call_id = call.get("id").and_then(Value::as_str);
        let id = call_id
            .map(str::to_string)
            .unwrap_or_else(|| format!("{name}-{j}"));
        let args = call.get("args").cloned().unwrap_or(Value::Null);
        let mut call_block = tool_call_block(ToolSource::Gemini, id.clone(), name, &args, || {
            Some(refs.field(&format!("{call_base}/args")))
        });

        // resultDisplay 是 FileDiff 时用真实 diff 覆盖参数估算
        if let Some(diff_text) = call
            .pointer("/resultDisplay/fileDiff")
            .and_then(Value::as_str)
        {
            if let SessionBlock::ToolCall { diff, title, .. } = &mut call_block {
                let (added, removed) = count_diff_lines(diff_text);
                let path = call
                    .pointer("/resultDisplay/filePath")
                    .or_else(|| call.pointer("/resultDisplay/fileName"))
                    .and_then(Value::as_str)
                    .unwrap_or(title.as_str())
                    .to_string();
                let op = diff
                    .as_ref()
                    .and_then(|d| d.files.first())
                    .map_or(DiffOp::Update, |f| f.op);
                let mut summary = single_file_diff(&path, op, added, removed);
                summary.full = Some(refs.field(&format!("{call_base}/resultDisplay/fileDiff")));
                *diff = Some(summary);
            }
        }
        blocks.push(call_block);

        let output = tool_output(refs, j, call, call_id, name);
        let mut result = tool_result_block(id, tool_status(call), &output.text, || output.full);
        if let SessionBlock::ToolResult { images, .. } = &mut result {
            *images = output.images;
        }
        blocks.push(result);
    }

    blocks
}

fn tool_status(call: &Value) -> ToolStatus {
    match call.get("status").and_then(Value::as_str) {
        Some("success" | "completed") => ToolStatus::Success,
        Some("error") => ToolStatus::Error,
        Some("cancelled" | "canceled") => ToolStatus::Interrupted,
        Some("executing" | "scheduled" | "validating" | "awaiting_approval") => ToolStatus::Pending,
        _ => ToolStatus::Unknown,
    }
}

struct ToolOutput {
    text: String,
    full: Option<ContentRef>,
    images: Vec<ImageRef>,
}

/// 工具输出：优先 `resultDisplay`（见 [`format_result_display`]），否则取 `result` 里
/// 这次调用的 `functionResponse.response.output|error`，再不行取字符串形式的 result。
/// 图片总是取自这次调用的 functionResponse。
fn tool_output(
    refs: &MsgRefs,
    index: usize,
    call: &Value,
    call_id: Option<&str>,
    name: &str,
) -> ToolOutput {
    let result = call.get("result");
    let parts = part_list(result);
    let result_ref = |sub: &str| refs.tool_result(index, call_id, sub);
    let matched = matching_response(&parts, call_id, name);
    let images = matched
        .map(|k| response_images(&parts, k, &result_ref))
        .unwrap_or_default();

    if let Some((text, sub)) = call.get("resultDisplay").and_then(format_result_display) {
        return ToolOutput {
            text,
            full: Some(refs.field(&format!("/toolCalls/{index}/resultDisplay{sub}"))),
            images,
        };
    }
    if let Some((text, full)) = matched.and_then(|k| response_text(&parts, k, &result_ref)) {
        return ToolOutput {
            text,
            full: Some(full),
            images,
        };
    }
    match result {
        Some(Value::String(text)) => ToolOutput {
            text: text.clone(),
            full: Some(result_ref("")),
            images,
        },
        _ => ToolOutput {
            text: String::new(),
            full: None,
            images,
        },
    }
}

/// 在 result 的 Part 里找这次调用的 functionResponse：先按 id，再按工具名，最后取第一个。
/// masking 同步后 result 是整个用户回合的 parts，会夹着同一批其他调用的结果。
fn matching_response(
    parts: &[(&Value, String)],
    call_id: Option<&str>,
    name: &str,
) -> Option<usize> {
    let responses: Vec<usize> = parts
        .iter()
        .enumerate()
        .filter(|(_, (part, _))| part.get("functionResponse").is_some_and(Value::is_object))
        .map(|(k, _)| k)
        .collect();
    let field = |k: usize, key: &str| {
        parts[k]
            .0
            .get("functionResponse")
            .and_then(|response| response.get(key))
            .and_then(Value::as_str)
    };
    call_id
        .and_then(|id| {
            responses
                .iter()
                .copied()
                .find(|&k| field(k, "id") == Some(id))
        })
        .or_else(|| {
            responses
                .iter()
                .copied()
                .find(|&k| field(k, "name") == Some(name))
        })
        .or_else(|| responses.first().copied())
}

/// functionResponse 的 `response.output` / `response.error`；都不是字符串时给整个 response
fn response_text(
    parts: &[(&Value, String)],
    k: usize,
    part_ref: &dyn Fn(&str) -> ContentRef,
) -> Option<(String, ContentRef)> {
    let (part, p) = &parts[k];
    let response = part.pointer("/functionResponse/response")?;
    for key in ["output", "error"] {
        if let Some(text) = response.get(key).and_then(Value::as_str) {
            return Some((
                text.to_string(),
                part_ref(&format!("{p}/functionResponse/response/{key}")),
            ));
        }
    }
    match response {
        Value::Object(obj) if !obj.is_empty() => Some((
            serde_json::to_string_pretty(response).unwrap_or_default(),
            part_ref(&format!("{p}/functionResponse/response")),
        )),
        _ => None,
    }
}

/// functionResponse 带的图片：嵌套在 `functionResponse.parts` 里的（模型支持多模态结果时），
/// 以及紧跟在它后面、下一个 functionResponse 之前的并列 inlineData。
fn response_images(
    parts: &[(&Value, String)],
    k: usize,
    part_ref: &dyn Fn(&str) -> ContentRef,
) -> Vec<ImageRef> {
    let (part, p) = &parts[k];
    let mut images = Vec::new();
    let nested = part
        .pointer("/functionResponse/parts")
        .and_then(Value::as_array);
    for (q, nested) in nested.into_iter().flatten().enumerate() {
        images.extend(inline_image(nested, || {
            part_ref(&format!("{p}/functionResponse/parts/{q}/inlineData/data"))
        }));
    }
    for (sibling, sp) in parts[k + 1..]
        .iter()
        .take_while(|(sibling, _)| sibling.get("functionResponse").is_none())
    {
        images.extend(inline_image(sibling, || {
            part_ref(&format!("{sp}/inlineData/data"))
        }));
    }
    images
}

/// 对不上调用的工具结果（如 `/rewind` 后官方重建历史时生成的新回合）单独显示
fn unmatched_result_block(refs: &MsgRefs, parts: &[(&Value, String)], k: usize) -> SessionBlock {
    let content_ref = |sub: &str| refs.content(sub);
    let part = parts[k].0;
    let call_id = part
        .pointer("/functionResponse/id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let status = if part.pointer("/functionResponse/response/error").is_some() {
        ToolStatus::Error
    } else {
        ToolStatus::Success
    };
    let (text, full) = match response_text(parts, k, &content_ref) {
        Some((text, full)) => (text, Some(full)),
        None => (String::new(), None),
    };
    let mut block = tool_result_block(call_id, status, &text, || full);
    if let SessionBlock::ToolResult { images, .. } = &mut block {
        *images = response_images(parts, k, &content_ref);
    }
    block
}

/// PartListUnion（字符串、单个 Part 或 Part 数组）里的各个 Part 与相对指针：
/// 数组按下标，单个 Part 为 `""`；字符串没有 Part。
fn part_list(value: Option<&Value>) -> Vec<(&Value, String)> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(i, part)| (part, format!("/{i}")))
            .collect(),
        Some(part @ Value::Object(_)) => vec![(part, String::new())],
        _ => Vec::new(),
    }
}

/// PartListUnion 的文本：字符串原样；Part 取 `text`（跳过 `thought: true`），与官方
/// `partListUnionToString` 一样不加分隔符直接拼接。
fn parts_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items.iter().filter_map(part_text).collect(),
        Some(part @ Value::Object(_)) => part_text(part).unwrap_or_default().to_string(),
        _ => String::new(),
    }
}

fn part_text(part: &Value) -> Option<&str> {
    match part {
        Value::String(text) => Some(text),
        Value::Object(obj) if obj.get("thought") != Some(&Value::Bool(true)) => {
            obj.get("text").and_then(Value::as_str)
        }
        _ => None,
    }
}

/// 只有 functionResponse 与并列的 inlineData / fileData（工具结果消息）
fn is_tool_result(parts: &[(&Value, String)]) -> bool {
    let has = |part: &Value, key: &str| part.get(key).is_some();
    parts.iter().any(|(part, _)| has(part, "functionResponse"))
        && parts.iter().all(|(part, _)| {
            ["functionResponse", "inlineData", "fileData"]
                .iter()
                .any(|key| has(part, key))
        })
}

fn is_injected(text: &str) -> bool {
    let text = text.trim_start();
    jsonl::INJECTED_PREFIXES
        .iter()
        .any(|prefix| text.starts_with(prefix))
}

/// content 只有一段文本时给出它的引用（字符串本身或唯一的 text 部件）
fn single_text_ref(
    content: Option<&Value>,
    content_ref: impl Fn(&str) -> ContentRef,
) -> Option<ContentRef> {
    match content? {
        Value::String(_) => Some(content_ref("")),
        Value::Object(part) if part.get("text").is_some_and(Value::is_string) => {
            Some(content_ref("/text"))
        }
        Value::Array(items) => {
            let mut texts = items
                .iter()
                .enumerate()
                .filter(|(_, part)| part_text(part).is_some());
            match (texts.next(), texts.next()) {
                (Some((i, part)), None) if part.is_object() => {
                    Some(content_ref(&format!("/{i}/text")))
                }
                (Some((i, _)), None) => Some(content_ref(&format!("/{i}"))),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Part 里的附件：`image/*` 的 inlineData 渲染为图片，其余 inlineData / fileData 用占位文本
fn media_blocks(
    parts: &[(&Value, String)],
    content_ref: impl Fn(&str) -> ContentRef,
) -> Vec<SessionBlock> {
    let mut blocks = Vec::new();
    for (part, p) in parts {
        if let Some(inline) = part.get("inlineData") {
            if let Some(image) = inline_image(part, || content_ref(&format!("{p}/inlineData/data")))
            {
                blocks.push(SessionBlock::Image { image });
                continue;
            }
            let mime = inline
                .get("mimeType")
                .and_then(Value::as_str)
                .unwrap_or("application/octet-stream");
            let size = inline
                .get("data")
                .and_then(Value::as_str)
                .map_or(0, str::len);
            blocks.push(SessionBlock::text(format!(
                "[Media: {mime}, {}]",
                format_size(estimate_base64_size(size))
            )));
        } else if let Some(file) = part.get("fileData") {
            let field = |key: &str| file.get(key).and_then(Value::as_str).unwrap_or_default();
            let label = [field("mimeType"), field("fileUri")]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            blocks.push(SessionBlock::text(format!("[File: {label}]")));
        }
    }
    blocks
}

/// `{inlineData: {mimeType: "image/*", data}}` → 内联图片引用（data 是不带 `data:` 前缀的 base64）
fn inline_image(part: &Value, data_ref: impl FnOnce() -> ContentRef) -> Option<ImageRef> {
    let inline = part.get("inlineData")?;
    let mime = inline.get("mimeType").and_then(Value::as_str)?;
    let data = inline.get("data").and_then(Value::as_str)?;
    if !mime.starts_with("image/") {
        return None;
    }
    Some(ImageRef {
        source: ImageSource::Inline {
            content: data_ref(),
        },
        media_type: mime.to_string(),
        size: estimate_base64_size(data.len()),
        alt: None,
    })
}

fn format_size(bytes: u32) -> String {
    const KB: f64 = 1024.0;
    let bytes = f64::from(bytes);
    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < KB * KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{:.1} MB", bytes / KB / KB)
    }
}

/// 多条 `thoughts[{subject, description}]` 合并后的思考：`summary` 为各 subject 以 ` · ` 连接，
/// `text` 为 `**subject**\n\ndescription` 以空行连接。
pub(crate) struct MergedThoughts {
    pub summary: String,
    pub text: String,
}

/// 合并 Gemini 的 `thoughts` 数组；不是对象数组、或没有任何非空 subject/description 时返回 `None`。
/// 解析器生成预览与按引用取全文共用这一份格式。
pub(crate) fn format_thoughts(value: &Value) -> Option<MergedThoughts> {
    let items = value.as_array()?;
    let mut thoughts = Vec::new();
    for item in items {
        let object = item.as_object()?;
        let field = |key: &str| object.get(key).and_then(Value::as_str).unwrap_or("").trim();
        let (subject, description) = (field("subject"), field("description"));
        if !subject.is_empty() || !description.is_empty() {
            thoughts.push((subject, description));
        }
    }
    if thoughts.is_empty() {
        return None;
    }
    let summary = thoughts
        .iter()
        .map(|(subject, _)| *subject)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let text = thoughts
        .iter()
        .map(
            |(subject, description)| match (subject.is_empty(), description.is_empty()) {
                (false, false) => format!("**{subject}**\n\n{description}"),
                (false, true) => format!("**{subject}**"),
                _ => description.to_string(),
            },
        )
        .collect::<Vec<_>>()
        .join("\n\n");
    Some(MergedThoughts { summary, text })
}

/// `resultDisplay`（ToolResultDisplay）的可读文本，以及这段文本在 resultDisplay 内的
/// JSON Pointer：文本正好是其中某个字符串字段时给出该字段；AnsiOutput、TodoList 这类
/// 需要拼接的为 `""`（按引用取全文时由 `content::resolve_content_ref` 用本函数同样格式化）。
/// 认不出或没有内容时返回 `None`，调用方改用 functionResponse 的输出。
pub(crate) fn format_result_display(display: &Value) -> Option<(String, &'static str)> {
    match display {
        Value::String(text) => (!text.trim().is_empty()).then(|| (text.clone(), "")),
        // AnsiOutput：`[[{text, bold, italic, ...}]]`，每行拼接各 token 的 text
        Value::Array(lines) => {
            if lines.is_empty() || !lines.iter().all(Value::is_array) {
                return None;
            }
            let text = lines
                .iter()
                .map(|line| {
                    let line: String = line
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|token| token.get("text").and_then(Value::as_str))
                        .collect();
                    line.trim_end().to_string()
                })
                .collect::<Vec<_>>()
                .join("\n");
            let text = text.trim_end();
            (!text.is_empty()).then(|| (text.to_string(), ""))
        }
        Value::Object(obj) => {
            let non_empty = |key: &str| {
                obj.get(key)
                    .and_then(Value::as_str)
                    .filter(|text| !text.trim().is_empty())
                    .map(str::to_string)
            };
            if let Some(diff) = obj.get("fileDiff").and_then(Value::as_str) {
                return Some((diff.to_string(), "/fileDiff"));
            }
            // SubagentProgress：最终结果在 `result`；还没有结果时退回 functionResponse
            if obj.get("isSubagentProgress") == Some(&Value::Bool(true)) {
                return non_empty("result").map(|result| (result, "/result"));
            }
            if let Some(todos) = obj.get("todos").and_then(Value::as_array) {
                let text = todos
                    .iter()
                    .filter_map(|todo| {
                        let description = todo.get("description").and_then(Value::as_str)?;
                        let mark = match todo.get("status").and_then(Value::as_str) {
                            Some("completed") => "[x]",
                            Some("in_progress") => "[~]",
                            Some("cancelled") => "[-]",
                            _ => "[ ]",
                        };
                        Some(format!("{mark} {description}"))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                return (!text.is_empty()).then_some((text, ""));
            }
            // GrepResult / ListDirectoryResult / ReadManyFilesResult 等带 summary 的结构化结果
            non_empty("summary").map(|summary| (summary, "/summary"))
        }
        _ => None,
    }
}

/// 会话文件相对 `tmp/` 的路径（旧格式整份 JSON 的 `ContentRef::File` 用，`content.rs` 按
/// 校验过的 tmp 根拼回去）。不在默认根下时（测试、改过目录）从最近的 `chats` 的上一级算起。
fn relative_to_tmp_root(path: &Path) -> String {
    let join = |rel: &Path| {
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    };
    let root = crate::gemini_config::get_gemini_dir().join("tmp");
    if let Some(rel) = root
        .canonicalize()
        .ok()
        .and_then(|root| path.strip_prefix(root).ok().map(join))
    {
        return rel;
    }
    if let Ok(rel) = path.strip_prefix(&root) {
        return join(rel);
    }
    let parts: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect();
    let start = parts
        .iter()
        .rposition(|part| part == "chats")
        .map_or(parts.len().saturating_sub(3), |i| i.saturating_sub(1));
    parts[start..].join("/")
}

/// `tokens{input, output, cached, thoughts, tool, total}` + 模型 → meta（0 视为缺省）。
fn gemini_meta(msg: &Value, model: Option<&str>) -> Option<MessageMeta> {
    let tokens = msg.get("tokens");
    let count = |key: &str| {
        tokens
            .and_then(|t| t.get(key))
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
    };
    let meta = MessageMeta {
        model: model.map(str::to_string),
        input_tokens: count("input"),
        output_tokens: count("output"),
        cache_read_tokens: count("cached"),
        reasoning_tokens: count("thoughts"),
        ..MessageMeta::default()
    };
    (meta != MessageMeta::default()).then_some(meta)
}

// ─── 删除 ────────────────────────────────────────────────────────────────

/// 删除列表里的一项：除了这份文件，还删掉被去重合并进这一项的其他文件（同一 `tmp` 根下
/// projectHash 与 sessionId 都相同的 `.json` / `.jsonl`，否则删掉一份后另一份又会出现在
/// 列表里），以及它们旁边的子代理目录 `chats/<sanitize(sessionId)>/`。
/// logs、tool-outputs 等附属文件不删。
pub fn delete_session(root: &Path, path: &Path, session_id: &str) -> Result<bool, String> {
    let header = jsonl::scan_header(path).ok().flatten().ok_or_else(|| {
        format!(
            "Failed to parse Gemini session metadata: {}",
            path.display()
        )
    })?;
    let found = header.meta.session_id.as_deref().unwrap_or_default();
    if found != session_id {
        return Err(format!(
            "Gemini session ID mismatch: expected {session_id}, found {found}"
        ));
    }

    let root = root.canonicalize().map_err(|e| {
        format!(
            "Failed to resolve Gemini session root {}: {e}",
            root.display()
        )
    })?;
    let source = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve Gemini session {}: {e}", path.display()))?;
    if !source.starts_with(&root) {
        return Err(format!(
            "Gemini session file is outside {}: {}",
            root.display(),
            source.display()
        ));
    }

    let copies = session_copies(&root, &source, &header.meta);
    let agent_dir = sanitize_session_id(session_id);
    if !agent_dir.is_empty() {
        let chats_dirs: BTreeSet<&Path> = copies
            .iter()
            .chain([&source])
            .filter_map(|file| file.parent())
            .collect();
        for chats in chats_dirs {
            remove_agent_dir(&root, &chats.join(&agent_dir))?;
        }
    }
    // 列表项自己的文件最后删：中途失败时这一项还在，可以重试
    for file in copies.iter().chain([&source]) {
        std::fs::remove_file(file).map_err(|e| {
            format!(
                "Failed to delete Gemini session file {}: {e}",
                file.display()
            )
        })?;
    }

    Ok(true)
}

/// 同一 `tmp` 根下与这份会话 projectHash、sessionId 都相同的其他会话文件（规范化路径）。
/// 先按官方命名 `session-*-<sessionId 前 8 位>.json(l)` 筛文件名，再读元数据确认。
fn session_copies(root: &Path, source: &Path, meta: &jsonl::Meta) -> Vec<PathBuf> {
    let Some(session_id) = meta.session_id.as_deref() else {
        return Vec::new();
    };
    let id8: String = sanitize_session_id(session_id).chars().take(8).collect();
    let suffixes = [format!("-{id8}.json"), format!("-{id8}.jsonl")];
    let mut copies = BTreeSet::new();
    for file in collect_session_files(root) {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if !suffixes
            .iter()
            .any(|suffix| name.ends_with(suffix.as_str()))
        {
            continue;
        }
        let Ok(file) = file.canonicalize() else {
            continue;
        };
        if file == source || !file.starts_with(root) {
            continue;
        }
        let Ok(Some(other)) = jsonl::scan_header(&file) else {
            continue;
        };
        if other.meta.session_id == meta.session_id && other.meta.project_hash == meta.project_hash
        {
            copies.insert(file);
        }
    }
    copies.into_iter().collect()
}

/// 删除子代理目录：不存在或不是真正的目录（符号链接等）时不动；规范化后必须仍在 tmp 根内。
fn remove_agent_dir(root: &Path, dir: &Path) -> Result<(), String> {
    let Ok(meta) = std::fs::symlink_metadata(dir) else {
        return Ok(());
    };
    if !meta.is_dir() {
        return Ok(());
    }
    let canonical = dir.canonicalize().map_err(|e| {
        format!(
            "Failed to resolve Gemini subagent directory {}: {e}",
            dir.display()
        )
    })?;
    if canonical == root || !canonical.starts_with(root) {
        return Err(format!(
            "Gemini subagent directory is outside {}: {}",
            root.display(),
            canonical.display()
        ));
    }
    std::fs::remove_dir_all(&canonical).map_err(|e| {
        format!(
            "Failed to delete Gemini subagent directory {}: {e}",
            canonical.display()
        )
    })
}

/// 官方子代理目录名、文件名里 id 的清洗规则：`[^a-zA-Z0-9_-]` → `_`
fn sanitize_session_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::model::ToolKind;
    use jsonl::tests::{MAIN_SESSION, PROJECT_HASH, SUBAGENT_SESSION};
    use serde_json::json;
    use tempfile::tempdir;

    const MAIN_ID: &str = "5f0c1a2b-3c4d-4e5f-8a9b-0c1d2e3f4a5b";
    const MAIN_FILE: &str = "session-2026-10-07T08-30-5f0c1a2b.jsonl";
    const PROJECT: &str = "/Users/alice/code/my-app";

    fn write(path: &Path, text: &str) -> PathBuf {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        path.to_path_buf()
    }

    fn lines(values: &[Value]) -> String {
        values.iter().map(|v| format!("{v}\n")).collect()
    }

    fn meta_line(session_id: &str) -> Value {
        json!({ "sessionId": session_id, "projectHash": "h1", "startTime": "2026-10-07T00:00:00Z",
                "lastUpdated": "2026-10-07T00:00:00Z", "kind": "main" })
    }

    fn texts(message: &SessionMessage) -> Vec<&str> {
        message
            .blocks
            .iter()
            .filter_map(|block| match block {
                SessionBlock::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn tool_results(message: &SessionMessage) -> Vec<(&str, &str, &[ImageRef])> {
        message
            .blocks
            .iter()
            .filter_map(|block| match block {
                SessionBlock::ToolResult {
                    call_id,
                    preview,
                    images,
                    ..
                } => Some((call_id.as_str(), preview.as_str(), images.as_slice())),
                _ => None,
            })
            .collect()
    }

    fn inline_pointer(image: &ImageRef) -> (&str, &str) {
        match &image.source {
            ImageSource::Inline {
                content: ContentRef::Jsonl { pointer, .. },
            } => (image.media_type.as_str(), pointer.as_str()),
            other => panic!("{other:?}"),
        }
    }

    /// 规格 8.1：阅读视图显示回放后的状态；注入上下文标 injected，能对上调用的工具结果消息不单独显示
    #[test]
    fn load_messages_renders_replayed_jsonl_session() {
        let temp = tempdir().unwrap();
        let path = write(
            &temp.path().join("my-app/chats").join(MAIN_FILE),
            MAIN_SESSION,
        );
        let msgs = load_messages(&path).unwrap();

        let ids: Vec<_> = msgs.iter().map(|m| m.id.as_deref().unwrap()).collect();
        assert_eq!(
            ids,
            ["m-ctx", "m-u1", "m-g1", "m-g2", "m-g3", "m-u5", "m-g5", "m-g6"]
        );
        let turns: Vec<_> = msgs.iter().map(|m| m.turn_id.as_deref().unwrap()).collect();
        assert_eq!(turns, ["t0", "t1", "t1", "t1", "t1", "t2", "t2", "t2"]);

        assert!(msgs[0].injected);
        assert!(texts(&msgs[0])[0].starts_with("<session_context>"));

        // 提问文本取 displayContent；@ 引用的图片照常显示
        let question = &msgs[1];
        assert_eq!(
            texts(question),
            ["看一下 @logo.png，再把 README 标题改成 My App"]
        );
        let image = question
            .blocks
            .iter()
            .find_map(|b| match b {
                SessionBlock::Image { image } => Some(image),
                _ => None,
            })
            .expect("image block");
        assert_eq!(
            inline_pointer(image),
            ("image/png", "/content/2/inlineData/data")
        );

        // read_file 的结果被 `$patch` 换成 masked 文本
        let g1 = &msgs[2];
        assert!(matches!(g1.blocks[0], SessionBlock::Thinking { .. }));
        assert_eq!(texts(g1), ["这是一个蓝色圆形 logo。我先读 README。"]);
        assert_eq!(
            tool_results(g1),
            [(
                "read_file__read_file_1791366625500_0",
                "<tool_output_masked>README.md, 3 lines</tool_output_masked>",
                &[][..]
            )]
        );

        match &msgs[3].blocks[0] {
            SessionBlock::ToolCall { kind, diff, .. } => {
                assert_eq!(*kind, ToolKind::Edit);
                let diff = diff.as_ref().unwrap();
                assert_eq!((diff.added, diff.removed), (1, 1));
                assert_eq!(diff.files[0].path, "/Users/alice/code/my-app/README.md");
            }
            other => panic!("{other:?}"),
        }
        // 子代理调用显示 SubagentProgress 的最终结果，而不是原始 JSON
        assert_eq!(
            tool_results(&msgs[6])[0].1,
            "src/ 下分为 core 与 cli 两个包……"
        );
        let meta = msgs[7].meta.as_ref().unwrap();
        assert_eq!(meta.model.as_deref(), Some("gemini-3-pro-preview"));
        assert_eq!(meta.input_tokens, Some(15200));
    }

    /// 被补丁替换过的工具结果、content 引用指向补丁行
    #[test]
    fn refs_into_patched_fields_point_at_the_patch_line() {
        let temp = tempdir().unwrap();
        let long: String = (0..30).map(|i| format!("line {i}\n")).collect();
        let call = |output: &str| {
            json!({ "id": "c1", "name": "run_shell_command", "args": { "command": "ls" }, "status": "success",
                    "result": [{ "functionResponse": { "id": "c1", "name": "run_shell_command", "response": { "output": output } } }] })
        };
        let text = lines(&[
            meta_line("s1"),
            json!({ "id": "u1", "type": "user", "content": [{ "text": "list" }] }),
            json!({ "id": "g1", "type": "gemini", "content": "", "toolCalls": [call("short")] }),
            json!({ "$patch": { "updates": [
                { "id": "g1", "content": [{ "text": "thinking", "thought": true }, { "text": "patched reply" }],
                  "toolCalls": [{ "id": "c1", "result": [{ "functionResponse": { "id": "c1", "name": "run_shell_command", "response": { "output": long } } }] }] }
            ] } }),
        ]);
        let path = write(&temp.path().join("p/chats/session-a.jsonl"), &text);
        let msgs = load_messages(&path).unwrap();
        let g1 = &msgs[1];
        assert_eq!(texts(g1), ["patched reply"]);
        let full = g1
            .blocks
            .iter()
            .find_map(|b| match b {
                SessionBlock::ToolResult { full, .. } => full.clone(),
                _ => None,
            })
            .expect("long output has a ref");
        let patch_line = text.lines().nth(3).unwrap();
        let offset = text.find(patch_line).unwrap() as u64;
        assert_eq!(
            full,
            ContentRef::Jsonl {
                offset,
                len: patch_line.len() as u32 + 1,
                pointer: "/$patch/updates/0/toolCalls/0/result/0/functionResponse/response/output"
                    .into()
            }
        );
    }

    /// masking 同步后 result 是整个回合的 parts：按 functionResponse.id 取各自的结果
    #[test]
    fn tool_results_match_function_responses_by_id() {
        let temp = tempdir().unwrap();
        let turn = json!([
            { "functionResponse": { "id": "a", "name": "read_file", "response": { "output": "content of a" } } },
            { "functionResponse": { "id": "b", "name": "read_file", "response": { "error": "b failed" } } },
        ]);
        let text = lines(&[
            meta_line("s1"),
            json!({ "id": "u1", "type": "user", "content": "read both" }),
            json!({ "id": "g1", "type": "gemini", "content": "", "toolCalls": [
                { "id": "b", "name": "read_file", "args": {}, "status": "error", "result": turn },
                { "id": "a", "name": "read_file", "args": {}, "status": "success", "result": turn },
                // 没有 id 时按工具名找
                { "name": "glob", "args": {}, "status": "success",
                  "result": [turn[0], { "functionResponse": { "name": "glob", "response": { "output": "3 files" } } }] },
            ] }),
            json!({ "id": "u2", "type": "user", "content": turn }),
        ]);
        let path = write(&temp.path().join("p/chats/session-a.jsonl"), &text);
        let msgs = load_messages(&path).unwrap();
        // 工具结果消息的 id 都能对上调用，不单独显示
        assert_eq!(msgs.len(), 2);
        let results: Vec<_> = tool_results(&msgs[1])
            .into_iter()
            .map(|(id, preview, _)| (id, preview))
            .collect();
        assert_eq!(
            results,
            [
                ("b", "b failed"),
                ("a", "content of a"),
                ("glob-2", "3 files")
            ]
        );
    }

    /// inlineData 出现在用户提问、嵌套的 functionResponse.parts、并列 part 三处都生成图片引用
    #[test]
    fn inline_images_in_prompts_and_tool_results() {
        let temp = tempdir().unwrap();
        let png = json!({ "inlineData": { "mimeType": "image/png", "data": "iVBORw0KGgo=" } });
        let pdf =
            json!({ "inlineData": { "mimeType": "application/pdf", "data": "A".repeat(4096) } });
        let nested = json!({ "functionResponse": { "id": "n", "name": "read_file",
            "response": { "output": "Binary content provided (1 item(s))." }, "parts": [png] } });
        let flat =
            json!({ "functionResponse": { "id": "f", "name": "read_file", "response": {} } });
        let text = lines(&[
            meta_line("s1"),
            json!({ "id": "u1", "type": "user", "content": [{ "text": "看图" }, png, pdf] }),
            json!({ "id": "g1", "type": "gemini", "content": "", "toolCalls": [
                { "id": "n", "name": "read_file", "args": {}, "status": "success", "result": [nested] },
                { "id": "f", "name": "read_file", "args": {}, "status": "success", "result": [nested, flat, png] },
            ] }),
            json!({ "id": "i1", "type": "info", "content": [{ "inlineData": { "mimeType": "audio/mpeg", "data": "AAAA" } }] }),
        ]);
        let path = write(&temp.path().join("p/chats/session-a.jsonl"), &text);
        let msgs = load_messages(&path).unwrap();

        let question = &msgs[0];
        assert_eq!(
            texts(question),
            ["看图", "[Media: application/pdf, 3.0 KB]"]
        );
        let image = match &question.blocks[1] {
            SessionBlock::Image { image } => image,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            inline_pointer(image),
            ("image/png", "/content/1/inlineData/data")
        );
        assert_eq!(image.size, 9);

        let results = tool_results(&msgs[1]);
        let pointers = |images: &[ImageRef]| -> Vec<String> {
            images
                .iter()
                .map(|i| inline_pointer(i).1.to_string())
                .collect()
        };
        assert_eq!(
            pointers(results[0].2),
            ["/toolCalls/0/result/0/functionResponse/parts/0/inlineData/data"]
        );
        // 并列的 inlineData 属于它前面最近的 functionResponse
        assert_eq!(
            pointers(results[1].2),
            ["/toolCalls/1/result/2/inlineData/data"]
        );

        // 二进制注入的 info 消息：非图片用占位文本
        assert_eq!(texts(&msgs[2]), ["[Media: audio/mpeg, 3 B]"]);
    }

    #[test]
    fn result_display_shapes_are_rendered_as_text() {
        let ansi = json!([
            [{ "text": "> build", "bold": true }],
            [{ "text": "error ", "fg": "red" }, { "text": "TS2322  " }],
            [{ "text": "" }],
        ]);
        assert_eq!(
            format_result_display(&ansi),
            Some(("> build\nerror TS2322".to_string(), ""))
        );
        let todos = json!({ "todos": [
            { "description": "Fix port", "status": "completed" },
            { "description": "Re-run build", "status": "in_progress" },
            { "description": "Ship", "status": "pending" },
        ] });
        assert_eq!(
            format_result_display(&todos),
            Some(("[x] Fix port\n[~] Re-run build\n[ ] Ship".to_string(), ""))
        );
        assert_eq!(
            format_result_display(&json!({ "summary": "Found 3 matches", "matches": [] })),
            Some(("Found 3 matches".to_string(), "/summary"))
        );
        assert_eq!(
            format_result_display(&json!({ "isSubagentProgress": true, "result": "done" })),
            Some(("done".to_string(), "/result"))
        );
        // 还没有结果的子代理、空字符串、认不出的对象：交给 functionResponse
        assert_eq!(
            format_result_display(&json!({ "isSubagentProgress": true, "summary": "x" })),
            None
        );
        assert_eq!(format_result_display(&json!("")), None);
        assert_eq!(format_result_display(&json!({ "foo": 1 })), None);

        let temp = tempdir().unwrap();
        let text = lines(&[
            meta_line("s1"),
            json!({ "id": "u1", "type": "user", "content": "go" }),
            json!({ "id": "g1", "type": "gemini", "content": "", "toolCalls": [
                { "id": "x", "name": "custom", "args": {}, "status": "success", "resultDisplay": { "foo": 1 },
                  "result": [{ "functionResponse": { "id": "x", "name": "custom", "response": { "output": "plain output" } } }] },
                { "id": "s", "name": "run_shell_command", "args": {}, "status": "success", "resultDisplay": ansi },
            ] }),
            json!({ "id": "w1", "type": "warning", "content": "Context window almost full" }),
        ]);
        let path = write(&temp.path().join("p/chats/session-a.jsonl"), &text);
        let msgs = load_messages(&path).unwrap();
        let results = tool_results(&msgs[1]);
        assert_eq!(results[0].1, "plain output");
        assert_eq!(results[1].1, "> build\nerror TS2322");
        // warning 不再被丢弃，照常显示
        assert!(!msgs[2].injected);
        assert!(matches!(
            &msgs[2].blocks[0],
            SessionBlock::Event { kind: EventKind::Other, text: Some(t), .. } if t == "Context window almost full"
        ));
    }

    /// 对不上调用的工具结果消息（如 `/rewind` 后官方重建历史生成的 `<id>_response` 回合）单独显示
    #[test]
    fn unmatched_tool_result_messages_render_as_tool_results() {
        let temp = tempdir().unwrap();
        let text = lines(&[
            meta_line("s1"),
            json!({ "id": "u1", "type": "user", "content": "go" }),
            json!({ "id": "u1_response", "type": "user", "content": [
                { "functionResponse": { "id": "gone", "name": "read_file", "response": { "output": "orphan output" } } },
                { "inlineData": { "mimeType": "image/jpeg", "data": "/9j/" } },
            ] }),
        ]);
        let path = write(&temp.path().join("p/chats/session-a.jsonl"), &text);
        let msgs = load_messages(&path).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[1].role, "tool");
        let results = tool_results(&msgs[1]);
        assert_eq!((results[0].0, results[0].1), ("gone", "orphan output"));
        assert_eq!(
            inline_pointer(&results[0].2[0]).1,
            "/content/1/inlineData/data"
        );
        assert_eq!(msgs[1].turn_id.as_deref(), Some("t1"));
    }

    /// 测试用的 `~/.gemini`：slug 目录、旧 hash 目录副本、旧 `.json` 与迁移出的 `.jsonl`、子代理
    struct GeminiHome {
        _temp: tempfile::TempDir,
        dir: PathBuf,
    }

    impl GeminiHome {
        fn tmp(&self) -> PathBuf {
            self.dir.join("tmp")
        }

        fn chats(&self, project: &str) -> PathBuf {
            self.tmp().join(project).join("chats")
        }
    }

    fn gemini_home() -> GeminiHome {
        let temp = tempdir().unwrap();
        let dir = temp.path().join(".gemini");
        let home = GeminiHome { _temp: temp, dir };
        let other_project = "/Users/alice/code/other";
        write(
            &home.dir.join("projects.json"),
            &json!({ "projects": { PROJECT: "my-app", other_project: "other" } }).to_string(),
        );
        write(&home.tmp().join("my-app/.project_root"), PROJECT);

        // 主会话：slug 目录一份，旧 hash 目录复制来的一份（没有 .project_root）
        write(&home.chats("my-app").join(MAIN_FILE), MAIN_SESSION);
        write(&home.chats(PROJECT_HASH).join(MAIN_FILE), MAIN_SESSION);
        // 子代理：在主会话 id 命名的子目录里，不进列表
        write(
            &home
                .chats("my-app")
                .join(MAIN_ID)
                .join("a1b2c3d4-0000-4000-8000-000000000001.jsonl"),
            SUBAGENT_SESSION,
        );
        // 附属文件：删除会话时不动
        write(
            &home
                .tmp()
                .join(format!("my-app/logs/session-{MAIN_ID}.jsonl")),
            "{}\n",
        );

        // 规格 8.3：旧 `.json` 与恢复时迁移出的 `.jsonl`（lastUpdated 相同）
        let legacy = json!({
            "sessionId": "7e8f9a0b-1111-4222-8333-444455556666", "projectHash": PROJECT_HASH,
            "startTime": "2026-03-20T09:15:02.000Z", "lastUpdated": "2026-03-20T09:20:41.000Z", "kind": "main",
            "messages": [
                { "id": "o-u1", "timestamp": "2026-03-20T09:15:10.000Z", "type": "user", "content": [{ "text": "hello" }] },
                { "id": "o-g1", "timestamp": "2026-03-20T09:15:12.000Z", "type": "gemini", "content": "Hi!", "thoughts": [],
                  "tokens": { "input": 9000, "output": 5, "cached": 0, "thoughts": 30, "tool": 0, "total": 9035 }, "model": "gemini-2.5-pro" }
            ]
        });
        let migrated = lines(&[
            json!({ "sessionId": "7e8f9a0b-1111-4222-8333-444455556666", "projectHash": PROJECT_HASH,
                    "startTime": "2026-03-20T09:15:02.000Z", "lastUpdated": "2026-03-20T09:20:41.000Z", "kind": "main" }),
            legacy["messages"][0].clone(),
            legacy["messages"][1].clone(),
            json!({ "$set": { "sessionId": "7e8f9a0b-1111-4222-8333-444455556666" } }),
        ]);
        let legacy_name = "session-2026-03-20T09-15-7e8f9a0b";
        write(
            &home.chats("my-app").join(format!("{legacy_name}.json")),
            &serde_json::to_string_pretty(&legacy).unwrap(),
        );
        write(
            &home.chats("my-app").join(format!("{legacy_name}.jsonl")),
            &migrated,
        );

        // 只在旧 hash 目录里的会话：项目路径按 projectHash 反查 projects.json
        let other_hash = sha256_hex(other_project);
        write(
            &home.chats(&other_hash).join("session-2026-01-02T03-04-0a0b0c0d.json"),
            &json!({ "sessionId": "0a0b0c0d-0000", "projectHash": other_hash, "lastUpdated": "2026-01-02T03:05:00Z",
                     "messages": [{ "id": "u", "type": "user", "content": [{ "text": "fix " }, { "text": "@a.ts" }],
                                    "displayContent": [{ "text": "fix @a.ts please" }] }] })
                .to_string(),
        );
        // 2026-03-26 之前的子代理（chats 根下、kind = subagent）、没有可恢复消息的会话、临时文件：都不列出
        write(
            &home
                .chats(&other_hash)
                .join("session-2026-01-02T03-06-1a1b1c1d.json"),
            &json!({ "sessionId": "1a1b1c1d", "projectHash": other_hash, "kind": "subagent",
                     "messages": [{ "id": "u", "type": "user", "content": "task" }] })
            .to_string(),
        );
        write(
            &home
                .chats("my-app")
                .join("session-2026-10-07T09-00-2b2b2b2b.jsonl"),
            &lines(&[
                meta_line("2b2b2b2b"),
                json!({ "id": "ctx", "type": "user", "content": [{ "text": "<session_context>x</session_context>" }] }),
            ]),
        );
        write(
            &home.chats("my-app").join(format!("{MAIN_FILE}.tmp-123")),
            MAIN_SESSION,
        );
        home
    }

    fn by_id(sessions: &[SessionMeta]) -> HashMap<&str, &SessionMeta> {
        sessions
            .iter()
            .map(|s| (s.session_id.as_str(), s))
            .collect()
    }

    #[test]
    fn scan_lists_each_session_once_with_project_dir_and_title() {
        let home = gemini_home();
        let cache = FileParseCache::new();
        let sessions = scan_sessions_in(&cache, &home.dir);
        let sessions_by_id = by_id(&sessions);
        let mut ids: Vec<_> = sessions_by_id.keys().copied().collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            [
                "0a0b0c0d-0000",
                MAIN_ID,
                "7e8f9a0b-1111-4222-8333-444455556666"
            ]
        );
        assert_eq!(sessions.len(), 3);

        // 主会话：lastUpdated 相同，取 slug 目录那份；标题用 summary
        let main = sessions_by_id[MAIN_ID];
        assert_eq!(
            main.source_path.as_deref(),
            Some(home.chats("my-app").join(MAIN_FILE).to_str().unwrap())
        );
        assert_eq!(main.project_dir.as_deref(), Some(PROJECT));
        assert_eq!(
            main.title.as_deref(),
            Some("修改 README 标题并分析项目结构")
        );
        assert_eq!(
            main.resume_command.as_deref(),
            Some(format!("gemini --resume {MAIN_ID}").as_str())
        );
        assert_eq!(
            main.last_active_at,
            parse_timestamp_to_ms(&json!("2026-10-07T08:33:00.001Z"))
        );
        assert_eq!(
            main.created_at,
            parse_timestamp_to_ms(&json!("2026-10-07T08:30:12.001Z"))
        );

        // 旧 `.json` 与迁移出的 `.jsonl`：lastUpdated 相同，取 `.jsonl`；content 是 Part 数组也能取到标题
        let legacy = sessions_by_id["7e8f9a0b-1111-4222-8333-444455556666"];
        assert!(legacy.source_path.as_deref().unwrap().ends_with(".jsonl"));
        assert_eq!(legacy.title.as_deref(), Some("hello"));

        // 只在 hash 目录里：按 projectHash 反查项目路径；标题优先 displayContent
        let other = sessions_by_id["0a0b0c0d-0000"];
        assert_eq!(
            other.project_dir.as_deref(),
            Some("/Users/alice/code/other")
        );
        assert_eq!(other.title.as_deref(), Some("fix @a.ts please"));

        // 缓存命中时结果不变（projectHash 不会漏进 project_dir）
        let again = scan_sessions_in(&cache, &home.dir);
        let again_by_id = by_id(&again);
        assert_eq!(again.len(), 3);
        assert_eq!(again_by_id[MAIN_ID].project_dir.as_deref(), Some(PROJECT));
    }

    #[test]
    fn scan_prefers_the_copy_updated_last() {
        let home = gemini_home();
        // hash 目录里的副本之后又被追加（lastUpdated 更新）：取这一份
        let newer = format!(
            "{MAIN_SESSION}{}\n",
            json!({ "$set": { "lastUpdated": "2026-10-08T00:00:00Z" } })
        );
        let copy = write(&home.chats(PROJECT_HASH).join(MAIN_FILE), &newer);
        let sessions = scan_sessions_in(&FileParseCache::new(), &home.dir);
        let main = by_id(&sessions)[MAIN_ID];
        assert_eq!(main.source_path.as_deref(), copy.to_str());
        // hash 目录没有 .project_root：样例的 projectHash 就是 PROJECT 的 sha256，反查得到
        assert_eq!(sha256_hex(PROJECT), PROJECT_HASH);
        assert_eq!(main.project_dir.as_deref(), Some(PROJECT));

        // 反查不到时项目目录为空
        std::fs::remove_file(home.dir.join("projects.json")).unwrap();
        std::fs::remove_file(home.tmp().join("my-app/.project_root")).unwrap();
        let sessions = scan_sessions_in(&FileParseCache::new(), &home.dir);
        assert_eq!(by_id(&sessions)[MAIN_ID].project_dir, None);
    }

    #[test]
    fn delete_removes_merged_copies_and_subagent_dirs() {
        let home = gemini_home();
        let tmp = home.tmp();
        let main = home.chats("my-app").join(MAIN_FILE);

        let err = delete_session(&tmp, &main, "other-id").unwrap_err();
        assert!(err.contains("mismatch"), "{err}");
        assert!(main.exists());

        assert!(delete_session(&tmp, &main, MAIN_ID).unwrap());
        assert!(!main.exists());
        assert!(!home.chats(PROJECT_HASH).join(MAIN_FILE).exists());
        assert!(!home.chats("my-app").join(MAIN_ID).exists());
        // 附属文件与其他会话不动
        assert!(tmp
            .join(format!("my-app/logs/session-{MAIN_ID}.jsonl"))
            .exists());
        assert!(home
            .chats("my-app")
            .join("session-2026-03-20T09-15-7e8f9a0b.json")
            .exists());
        assert!(home
            .chats("my-app")
            .join(format!("{MAIN_FILE}.tmp-123"))
            .exists());

        // 旧 `.json` 与迁移出的 `.jsonl` 一起删掉，否则删一份后另一份又会出现在列表里
        let legacy = home
            .chats("my-app")
            .join("session-2026-03-20T09-15-7e8f9a0b.jsonl");
        assert!(delete_session(&tmp, &legacy, "7e8f9a0b-1111-4222-8333-444455556666").unwrap());
        assert!(!legacy.exists());
        assert!(!legacy.with_extension("json").exists());

        let sessions = scan_sessions_in(&FileParseCache::new(), &home.dir);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "0a0b0c0d-0000");
    }

    /// 子代理目录是指向 tmp 根外的符号链接时不跟过去删
    #[cfg(unix)]
    #[test]
    fn delete_does_not_follow_symlinked_subagent_dirs() {
        let home = gemini_home();
        let outside = tempdir().unwrap();
        std::fs::write(outside.path().join("keep.txt"), "x").unwrap();
        let agents = home.chats("my-app").join(MAIN_ID);
        std::fs::remove_dir_all(&agents).unwrap();
        std::os::unix::fs::symlink(outside.path(), &agents).unwrap();

        let main = home.chats("my-app").join(MAIN_FILE);
        assert!(delete_session(&home.tmp(), &main, MAIN_ID).unwrap());
        assert!(outside.path().join("keep.txt").exists());
    }

    /// 契约 fixture `tests/fixtures/sessions/gemini.messages.json` 就是这份构造的 JSONL 会话的
    /// 解析结果（前端 reader 测试也用它）。渲染逻辑改动后，按失败信息里的路径拿到新的输出更新 fixture。
    #[test]
    fn shared_fixture_matches_parser_output() {
        let temp = tempdir().unwrap();
        let path = write(
            &temp
                .path()
                .join("demo-app/chats/session-2026-10-02T08-00-3c1e7a52.jsonl"),
            FIXTURE_SESSION,
        );
        let actual = serde_json::to_value(load_messages(&path).unwrap()).unwrap();
        let expected: Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/sessions/gemini.messages.json"
        ))
        .unwrap();
        if actual != expected {
            let out = std::env::temp_dir().join("gemini.messages.actual.json");
            std::fs::write(&out, serde_json::to_string_pretty(&actual).unwrap() + "\n").unwrap();
            panic!("解析结果与 fixture 不一致，实际输出见 {}", out.display());
        }
    }

    /// 与契约 fixture 对应的会话：注入上下文、登录提示、带图片的提问、一次并行工具调用
    /// （结果消息不单独显示）、最终回复、配额错误；第二轮被 `/rewind` 撤掉一次提问后重问，
    /// 改文件、搜索、更新待办、取消命令。
    /// 其中 fileDiff 末尾没有换行：预览会去掉末尾换行，而前端的 fixture 校验要求未截断的
    /// 结果 totalLen 等于预览字数。
    const FIXTURE_SESSION: &str = r##"{"sessionId":"3c1e7a52-9b4d-4f20-8e6a-1d2c3b4a5f60","projectHash":"6c8f5a8f0e7b3c1d2a4b6c8d0e2f4a6b8c0d2e4f6a8b0c2d4e6f8a0b2c4d6e8f","startTime":"2026-10-02T08:00:00.000Z","lastUpdated":"2026-10-02T08:00:00.000Z","kind":"main"}
{"id":"d04923d38bb0f6017037e74183378ef4","timestamp":"2026-10-02T08:00:00.000Z","type":"user","content":[{"text":"<session_context>\nThis is the Gemini CLI. We are setting up the context for our chat.\nToday's date is Friday, October 2, 2026.\nMy operating system is: darwin\nI'm currently working in the directory: /Users/yovinchen/Projects/demo-app\n</session_context>"}]}
{"$set":{"lastUpdated":"2026-10-02T08:00:00.001Z"}}
{"id":"a8d1c3e5-7f90-4b2d-9e4f-6a8b0c2d4e55","timestamp":"2026-10-02T08:00:00.500Z","type":"info","content":"Authenticated via \"Login with Google\"."}
{"$set":{"lastUpdated":"2026-10-02T08:00:00.501Z"}}
{"id":"0b6f2c1e-1d4a-4b7e-9a35-6c8d2e0f4a11","timestamp":"2026-10-02T08:00:03.000Z","type":"user","content":[{"text":"解释一下 src/main.ts 的启动流程，然后把构建跑一下。"},{"inlineData":{"mimeType":"image/png","data":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="}}]}
{"$set":{"lastUpdated":"2026-10-02T08:00:03.001Z"}}
{"id":"7c2e9a40-5b13-4f6d-8e21-0a9b3c7d1e22","timestamp":"2026-10-02T08:00:09.000Z","type":"gemini","content":"","thoughts":[{"subject":"Locating the entry point","description":"I need to read src/main.ts to see how the app boots before running the build.","timestamp":"2026-10-02T08:00:07.000Z"},{"subject":"Planning the build check","description":"After reading, run `npm run build` to surface compile errors.","timestamp":"2026-10-02T08:00:08.000Z"}],"tokens":{"input":10342,"output":182,"cached":8192,"thoughts":236,"tool":0,"total":10760},"model":"gemini-2.5-pro"}
{"$set":{"lastUpdated":"2026-10-02T08:00:09.001Z"}}
{"id":"7c2e9a40-5b13-4f6d-8e21-0a9b3c7d1e22","timestamp":"2026-10-02T08:00:09.000Z","type":"gemini","content":"","thoughts":[{"subject":"Locating the entry point","description":"I need to read src/main.ts to see how the app boots before running the build.","timestamp":"2026-10-02T08:00:07.000Z"},{"subject":"Planning the build check","description":"After reading, run `npm run build` to surface compile errors.","timestamp":"2026-10-02T08:00:08.000Z"}],"tokens":{"input":10342,"output":182,"cached":8192,"thoughts":236,"tool":0,"total":10760},"model":"gemini-2.5-pro","toolCalls":[{"id":"read_file-1791014409123-3f2a","name":"read_file","args":{"absolute_path":"/Users/yovinchen/Projects/demo-app/src/main.ts"},"result":[{"functionResponse":{"id":"read_file-1791014409123-3f2a","name":"read_file","response":{"output":"import { createApp } from \"./app\";\nimport { loadConfig } from \"./config\";\n\nconst config = await loadConfig();\nconst app = createApp(config);\napp.listen(config.port);"}}}],"status":"success","timestamp":"2026-10-02T08:00:10.000Z","resultDisplay":"","description":"src/main.ts","displayName":"ReadFile","renderOutputAsMarkdown":true},{"id":"run_shell_command-1791014410456-8c1d","name":"run_shell_command","args":{"command":"npm run build","description":"Build the project"},"result":[{"functionResponse":{"id":"run_shell_command-1791014410456-8c1d","name":"run_shell_command","response":{"output":"Command: npm run build\nDirectory: (root)\nOutput: > demo-app@0.3.1 build\n> tsc -p .\n\nsrc/config.ts(18,7): error TS2322: Type 'string' is not assignable to type 'number'.\nError: (none)\nExit Code: 2"}}}],"status":"error","timestamp":"2026-10-02T08:00:14.000Z","resultDisplay":[[{"text":"> demo-app@0.3.1 build","bold":false,"italic":false,"underline":false,"dim":false,"inverse":false,"fg":"","bg":""}],[{"text":"> tsc -p .","bold":false,"italic":false,"underline":false,"dim":false,"inverse":false,"fg":"","bg":""}],[{"text":"","bold":false,"italic":false,"underline":false,"dim":false,"inverse":false,"fg":"","bg":""}],[{"text":"src/config.ts(18,7): ","bold":false,"italic":false,"underline":false,"dim":false,"inverse":false,"fg":"","bg":""},{"text":"error TS2322: Type 'string' is not assignable to type 'number'.","bold":false,"italic":false,"underline":false,"dim":false,"inverse":false,"fg":"#ff5555","bg":""}],[{"text":"","bold":false,"italic":false,"underline":false,"dim":false,"inverse":false,"fg":"","bg":""}]],"description":"Build the project","displayName":"Shell","renderOutputAsMarkdown":false}]}
{"id":"5e7a9c1b-3d5f-4a7c-9e1b-3d5f7a9c1b33","timestamp":"2026-10-02T08:00:14.010Z","type":"user","content":[{"functionResponse":{"id":"read_file-1791014409123-3f2a","name":"read_file","response":{"output":"import { createApp } from \"./app\";\nimport { loadConfig } from \"./config\";\n\nconst config = await loadConfig();\nconst app = createApp(config);\napp.listen(config.port);"}}},{"functionResponse":{"id":"run_shell_command-1791014410456-8c1d","name":"run_shell_command","response":{"output":"Command: npm run build\nDirectory: (root)\nOutput: > demo-app@0.3.1 build\n> tsc -p .\n\nsrc/config.ts(18,7): error TS2322: Type 'string' is not assignable to type 'number'.\nError: (none)\nExit Code: 2"}}}]}
{"$set":{"lastUpdated":"2026-10-02T08:00:14.011Z"}}
{"id":"2f4a6c8e-0b1d-4e3f-8a5c-7e9b1d3f5a77","timestamp":"2026-10-02T08:00:21.000Z","type":"gemini","content":"`src/main.ts` 先 `await loadConfig()` 读配置，再 `createApp(config)` 创建应用并监听 `config.port`。\n\n构建失败在 `src/config.ts:18`：`port` 从环境变量读出来是字符串，需要 `Number(...)` 转换。","thoughts":[],"tokens":{"input":11020,"output":236,"cached":10240,"thoughts":0,"tool":0,"total":11256},"model":"gemini-2.5-pro"}
{"$set":{"lastUpdated":"2026-10-02T08:00:21.001Z"}}
{"id":"c3e5a7b9-1d2f-4b6d-8f0a-2c4e6a8b0d88","timestamp":"2026-10-02T08:01:30.000Z","type":"error","content":"[API Error: You have exhausted your capacity on this model. Quota will reset after 1m.]"}
{"$set":{"lastUpdated":"2026-10-02T08:01:30.001Z"}}
{"id":"e1f2a3b4-c5d6-4e7f-8a9b-0c1d2e3f4a99","timestamp":"2026-10-02T08:02:40.000Z","type":"user","content":[{"text":"把 port 的类型改一下"}]}
{"$set":{"lastUpdated":"2026-10-02T08:02:40.001Z"}}
{"$rewindTo":"e1f2a3b4-c5d6-4e7f-8a9b-0c1d2e3f4a99"}
{"id":"d41f7e0a-2c6b-4a19-b8e3-5f0d1c2a9b33","timestamp":"2026-10-02T08:03:00.000Z","type":"user","content":[{"text":"修一下，顺便查查 Node 22 下 top-level await 有没有坑。"}]}
{"$set":{"lastUpdated":"2026-10-02T08:03:00.001Z"}}
{"id":"9a0c3e51-6d24-4b8f-a172-3e5f7b9d0c44","timestamp":"2026-10-02T08:03:08.000Z","type":"gemini","content":"","thoughts":[{"subject":"Fixing the port type","description":"Wrap the env value with Number() and fall back to 3000.","timestamp":"2026-10-02T08:03:06.000Z"}],"tokens":{"input":12011,"output":147,"cached":0,"thoughts":140,"tool":0,"total":12298},"model":"gemini-2.5-pro"}
{"$set":{"lastUpdated":"2026-10-02T08:03:08.001Z"}}
{"id":"9a0c3e51-6d24-4b8f-a172-3e5f7b9d0c44","timestamp":"2026-10-02T08:03:08.000Z","type":"gemini","content":"","thoughts":[{"subject":"Fixing the port type","description":"Wrap the env value with Number() and fall back to 3000.","timestamp":"2026-10-02T08:03:06.000Z"}],"tokens":{"input":12011,"output":147,"cached":0,"thoughts":140,"tool":0,"total":12298},"model":"gemini-2.5-pro","toolCalls":[{"id":"replace-1791014588001-a7e4","name":"replace","args":{"file_path":"/Users/yovinchen/Projects/demo-app/src/config.ts","old_string":"  port: process.env.PORT ?? 3000,","new_string":"  port: Number(process.env.PORT ?? 3000),"},"result":[{"functionResponse":{"id":"replace-1791014588001-a7e4","name":"replace","response":{"output":"Successfully modified file: /Users/yovinchen/Projects/demo-app/src/config.ts (1 replacements)."}}}],"status":"success","timestamp":"2026-10-02T08:03:09.000Z","resultDisplay":{"fileDiff":"Index: config.ts\n===================================================================\n--- config.ts\tCurrent\n+++ config.ts\tProposed\n@@ -15,7 +15,7 @@\n export const loadConfig = async () => ({\n-  port: process.env.PORT ?? 3000,\n+  port: Number(process.env.PORT ?? 3000),\n });","fileName":"config.ts","filePath":"/Users/yovinchen/Projects/demo-app/src/config.ts","originalContent":"export const loadConfig = async () => ({\n  port: process.env.PORT ?? 3000,\n});\n","newContent":"export const loadConfig = async () => ({\n  port: Number(process.env.PORT ?? 3000),\n});\n","diffStat":{"model_added_lines":1,"model_removed_lines":1,"model_added_chars":40,"model_removed_chars":32,"user_added_lines":0,"user_removed_lines":0,"user_added_chars":0,"user_removed_chars":0}},"description":"src/config.ts:   port: process.env.PORT ?? 3000, =>   port: Number(process.env.PORT ?? 3000),","displayName":"Edit","renderOutputAsMarkdown":true},{"id":"google_web_search-1791014590412-55b0","name":"google_web_search","args":{"query":"Node 22 top-level await ESM pitfalls"},"result":[{"functionResponse":{"id":"google_web_search-1791014590412-55b0","name":"google_web_search","response":{"output":"Web search results for \"Node 22 top-level await ESM pitfalls\":\n\nTop-level await only works in ES modules (\"type\": \"module\" or .mjs)."}}}],"status":"success","timestamp":"2026-10-02T08:03:11.000Z","resultDisplay":"Search results for \"Node 22 top-level await ESM pitfalls\" returned.","description":"Searching the web for: \"Node 22 top-level await ESM pitfalls\"","displayName":"GoogleSearch","renderOutputAsMarkdown":true},{"id":"write_todos-1791014592230-9e1c","name":"write_todos","args":{"todos":[{"description":"Fix port type","status":"completed"},{"description":"Re-run build","status":"in_progress"}]},"result":[{"functionResponse":{"id":"write_todos-1791014592230-9e1c","name":"write_todos","response":{"output":"Successfully updated the todo list."}}}],"status":"success","timestamp":"2026-10-02T08:03:12.000Z","resultDisplay":{"todos":[{"description":"Fix port type","status":"completed"},{"description":"Re-run build","status":"in_progress"}]},"description":"Set 2 todo(s)","displayName":"WriteTodos","renderOutputAsMarkdown":true},{"id":"run_shell_command-1791014593000-1b7f","name":"run_shell_command","args":{"command":"npm run build"},"result":[{"functionResponse":{"id":"run_shell_command-1791014593000-1b7f","name":"run_shell_command","response":{"error":"Command was cancelled by the user."}}}],"status":"cancelled","timestamp":"2026-10-02T08:03:20.000Z","description":"npm run build","displayName":"Shell","renderOutputAsMarkdown":false}]}
{"$set":{"lastUpdated":"2026-10-02T08:03:20.001Z"}}
"##;
    #[test]
    fn delete_session_removes_json_file() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("session-2026-03-06T10-17-test.json");
        std::fs::write(
            &path,
            r#"{
              "sessionId": "gemini-session-123",
              "startTime": "2026-03-06T10:17:58.000Z",
              "lastUpdated": "2026-03-06T10:20:00.000Z",
              "messages": [
                {
                  "id": "msg-1",
                  "timestamp": "2026-03-06T10:17:58.000Z",
                  "type": "user",
                  "content": "hello"
                }
              ]
            }"#,
        )
        .expect("write session");

        delete_session(temp.path(), &path, "gemini-session-123").expect("delete session");

        assert!(!path.exists());
    }

    #[test]
    fn load_messages_handles_array_content() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("session.json");
        std::fs::write(
            &path,
            r#"{
              "sessionId": "test",
              "messages": [
                {"id":"1","timestamp":"2026-03-06T10:00:00Z","type":"user","content":[{"text":"hello"}]},
                {"id":"2","timestamp":"2026-03-06T10:00:01Z","type":"gemini","content":"world"},
                {"id":"3","timestamp":"2026-03-06T10:00:02Z","type":"info","content":"system info"},
                {"id":"4","timestamp":"2026-03-06T10:00:03Z","type":"error","content":"MCP ERROR"}
              ]
            }"#,
        )
        .expect("write");

        let msgs = load_messages(&path).expect("load");
        // info / error 现在作为 system 事件保留（info 标记为注入内容）
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "hello");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[1].content, "world");
        assert_eq!((msgs[2].role.as_str(), msgs[2].injected), ("system", true));
        assert!(matches!(
            &msgs[2].blocks[0],
            SessionBlock::Event { kind: EventKind::Info, text: Some(t), .. } if t == "system info"
        ));
        assert!(!msgs[3].injected);
        assert!(matches!(
            &msgs[3].blocks[0],
            SessionBlock::Event { kind: EventKind::Error, text: Some(t), .. } if t == "MCP ERROR"
        ));
    }

    #[test]
    fn load_messages_includes_tool_calls() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("session.json");
        std::fs::write(
            &path,
            r#"{
              "sessionId": "test",
              "messages": [
                {"id":"1","timestamp":"2026-03-10T08:24:50Z","type":"gemini","content":"","toolCalls":[{"id":"call_1","name":"web_search","args":{"query":"test"}}]},
                {"id":"2","timestamp":"2026-03-10T08:25:00Z","type":"gemini","content":"Here are the results.","toolCalls":[{"id":"call_2","name":"web_fetch","args":{"url":"http://example.com"}}]}
              ]
            }"#,
        )
        .expect("write");

        let msgs = load_messages(&path).expect("load");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "assistant");
        assert!(msgs[0].content.contains("[Tool: web_search]"));
        assert_eq!(msgs[1].role, "assistant");
        assert!(msgs[1].content.contains("Here are the results."));
        assert!(msgs[1].content.contains("[Tool: web_fetch]"));
    }

    #[test]
    fn load_messages_maps_thoughts_tool_calls_and_meta() {
        let temp = tempdir().expect("tempdir");
        let chats = temp.path().join("tmp").join("hash1").join("chats");
        std::fs::create_dir_all(&chats).expect("chats dir");
        let path = chats.join("session-x.json");
        let long_output: String = (1..=20).map(|i| format!("line {i}\n")).collect();
        let session = serde_json::json!({
            "sessionId": "test",
            "messages": [
                {"id":"i0","timestamp":"2026-03-10T08:24:40Z","type":"info","content":"Authenticated"},
                {"id":"u1","timestamp":"2026-03-10T08:24:45Z","type":"user","content":"build it"},
                {"id":"g1","timestamp":"2026-03-10T08:24:50Z","type":"gemini","content":"Done.",
                 "model":"gemini-2.5-pro",
                 "tokens":{"input":10342,"output":418,"cached":0,"thoughts":236,"tool":0,"total":10996},
                 "thoughts":[
                    {"subject":"Locating the entry point","description":"Read src/main.ts first."},
                    {"subject":"Planning the build check","description":"Then run npm run build."}],
                 "toolCalls":[
                    {"id":"run-1","name":"run_shell_command","args":{"command":"npm run build","description":"Build the project"},
                     "status":"error","resultDisplay":long_output},
                    {"id":"rep-1","name":"replace","status":"success",
                     "args":{"file_path":"/p/config.ts","old_string":"a","new_string":"b"},
                     "resultDisplay":{"fileName":"config.ts","fileDiff":"--- a\n+++ b\n-a\n+b\n+c\n"}},
                    {"id":"cancel-1","name":"run_shell_command","args":{"command":"sleep 9"},"status":"cancelled",
                     "result":[{"functionResponse":{"id":"cancel-1","name":"run_shell_command",
                       "response":{"error":"Command was cancelled by the user."}}}]}
                 ]}
            ]
        });
        std::fs::write(&path, session.to_string()).expect("write");

        let msgs = load_messages(&path).expect("load");
        assert_eq!(msgs.len(), 3);
        let turns: Vec<_> = msgs.iter().map(|m| m.turn_id.as_deref().unwrap()).collect();
        assert_eq!(turns, ["t0", "t1", "t1"]);
        assert_eq!(msgs[1].id.as_deref(), Some("u1"));

        let a = &msgs[2];
        match &a.blocks[0] {
            SessionBlock::Thinking {
                text,
                summary,
                full,
                ..
            } => {
                assert_eq!(
                    summary.as_deref(),
                    Some("Locating the entry point · Planning the build check")
                );
                assert!(text.starts_with("**Locating the entry point**\n\nRead src/main.ts first."));
                assert!(full.is_none());
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(&a.blocks[1], SessionBlock::Text { text, .. } if text == "Done."));
        match &a.blocks[2] {
            SessionBlock::ToolCall {
                kind,
                title,
                detail,
                ..
            } => {
                assert_eq!(*kind, ToolKind::Shell);
                assert_eq!(title, "npm run build");
                assert_eq!(detail.as_deref(), Some("Build the project"));
            }
            other => panic!("{other:?}"),
        }
        match &a.blocks[3] {
            SessionBlock::ToolResult {
                status,
                truncated,
                full,
                ..
            } => {
                assert_eq!(*status, ToolStatus::Error);
                assert!(*truncated);
                assert_eq!(
                    full,
                    &Some(ContentRef::File {
                        rel_path: "hash1/chats/session-x.json".into(),
                        pointer: "/messages/2/toolCalls/0/resultDisplay".into(),
                    })
                );
            }
            other => panic!("{other:?}"),
        }
        match &a.blocks[4] {
            SessionBlock::ToolCall { kind, diff, .. } => {
                assert_eq!(*kind, ToolKind::Edit);
                let diff = diff.as_ref().expect("diff");
                assert_eq!((diff.added, diff.removed), (2, 1));
                assert_eq!(diff.files[0].path, "config.ts");
            }
            other => panic!("{other:?}"),
        }
        match &a.blocks[7] {
            SessionBlock::ToolResult {
                status, preview, ..
            } => {
                assert_eq!(*status, ToolStatus::Interrupted);
                assert_eq!(preview, "Command was cancelled by the user.");
            }
            other => panic!("{other:?}"),
        }
        let meta = a.meta.as_ref().expect("meta");
        assert_eq!(meta.model.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(meta.input_tokens, Some(10342));
        assert_eq!(meta.cache_read_tokens, None);
        assert_eq!(meta.reasoning_tokens, Some(236));
        // 思考不进入 content
        assert!(a
            .content
            .starts_with("Done.\n\n[Tool: run_shell_command] npm run build"));
    }
}
