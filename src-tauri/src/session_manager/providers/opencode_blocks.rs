//! OpenCode 的 part → block 映射（§4.4）。文件存储、SQLite v1、SQLite v2 三条读取路径
//! 都把一条消息的 parts 交给 [`message_from_parts`]，只在「全文引用指向哪里」上不同
//! （见 [`PartLocator`]）。

use serde_json::Value;

use crate::session_manager::model::{
    ContentRef, DiffFile, DiffOp, DiffSummary, EventKind, ImageRef, ImageSource, MessageMeta,
    SessionBlock, SessionMessage, StepPhase, ToolKind, ToolStatus,
};

use super::blocks::{
    count_diff_lines, estimate_base64_size, parse_arguments, single_file_diff, str_field,
    thinking_block, title_path, tool_call_block, tool_result_block, ToolSource,
};
use super::utils::parse_timestamp_to_ms;

/// 一个 part 在源存储里的位置，用来生成按需取全文的 [`ContentRef`]。
#[derive(Debug, Clone)]
pub(super) enum PartLocator {
    /// SQLite 行：`table` 白名单内（`part` / `session_message`），`base` 是 part 在列 JSON 里的前缀
    Sqlite {
        table: &'static str,
        id: String,
        base: String,
    },
    /// 文件存储：相对 storage 根的 part 文件路径（`part/<msgId>/<partId>.json`）
    File { rel_path: String },
}

impl PartLocator {
    fn content_ref(&self, sub_pointer: &str) -> Option<ContentRef> {
        Some(match self {
            PartLocator::Sqlite { table, id, base } => ContentRef::Sqlite {
                table: (*table).to_string(),
                id: id.clone(),
                column: "data".to_string(),
                pointer: format!("{base}{sub_pointer}"),
            },
            PartLocator::File { rel_path } => ContentRef::File {
                rel_path: rel_path.clone(),
                pointer: sub_pointer.to_string(),
            },
        })
    }
}

/// 把一条 OpenCode 消息（`message.data` + 有序 parts）转成 [`SessionMessage`]。
///
/// `info` 是消息本体（v1 `message.data` / 文件存储的 message JSON / v2 `session_message.data`），
/// 用来取 meta 与消息级错误；`role` 由调用方给出（v2 的角色在 `type` 列）。
pub(super) fn message_from_parts(
    role: &str,
    id: Option<String>,
    ts: Option<i64>,
    info: &Value,
    parts: &[(PartLocator, Value)],
) -> SessionMessage {
    let mut blocks = Vec::new();
    for (locator, part) in parts {
        push_part_blocks(&mut blocks, locator, part);
    }
    if let Some(event) = message_error_event(info) {
        blocks.push(event);
    }

    let mut message = SessionMessage::from_blocks(role, ts, blocks);
    message.id = id;
    if role == "assistant" {
        message.meta = message_meta(info);
    }
    message
}

fn push_part_blocks(blocks: &mut Vec<SessionBlock>, locator: &PartLocator, part: &Value) {
    match part.get("type").and_then(Value::as_str) {
        Some("text") => {
            // synthetic（OpenCode 自己拼进去的提示）与 ignored 的文本不属于对话
            if part.get("synthetic").and_then(Value::as_bool) == Some(true)
                || part.get("ignored").and_then(Value::as_bool) == Some(true)
            {
                return;
            }
            if let Some(text) = part
                .get("text")
                .and_then(Value::as_str)
                .filter(|t| !t.trim().is_empty())
            {
                blocks.push(SessionBlock::text(text.to_string()));
            }
        }
        Some("reasoning") => {
            let text = part.get("text").and_then(Value::as_str).unwrap_or("");
            blocks.push(thinking_block(text, None, part_duration(part), || {
                locator.content_ref("/text")
            }));
        }
        Some("step-start") => blocks.push(SessionBlock::Step {
            phase: StepPhase::Start,
            tokens: None,
            cost_usd: None,
            reason: None,
        }),
        Some("step-finish") => blocks.push(SessionBlock::Step {
            phase: StepPhase::Finish,
            tokens: part.get("tokens").and_then(total_tokens),
            cost_usd: part.get("cost").and_then(Value::as_f64),
            reason: part
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        }),
        Some("tool") => push_tool_blocks(blocks, locator, part),
        Some("agent") => {
            if let Some(name) = part.get("name").and_then(Value::as_str) {
                blocks.push(SessionBlock::event(
                    EventKind::Other,
                    Some(format!("@{name}")),
                    None,
                ));
            }
        }
        Some("patch") => push_patch_blocks(blocks, part),
        Some("file") => push_file_block(blocks, locator, part),
        // snapshot / compaction / subtask / retry 等不进入阅读视图
        _ => {}
    }
}

/// `tool` → ToolCall + ToolResult。
///
/// v1 part：`{callID, tool, state{status, input, output, error, title, metadata, time{start,end}}}`；
/// v2 内容项：`{id, name, state{status, input, content[], error{type,message}, metadata},
/// time{created, ran, completed}}`，输出在 `content[]`，没有 `output`/`title`。
fn push_tool_blocks(blocks: &mut Vec<SessionBlock>, locator: &PartLocator, part: &Value) {
    let name = part
        .get("tool")
        .or_else(|| part.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let call_id = part
        .get("callID")
        .or_else(|| part.get("id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let state = part.get("state");
    // v2 `streaming` 状态的 input 是尚未写完的 JSON 字符串
    let input = state
        .and_then(|s| s.get("input"))
        .or_else(|| part.get("input"))
        .map(parse_arguments)
        .unwrap_or(Value::Null);
    let metadata = state.and_then(|s| s.get("metadata"));
    let state_title = state
        .and_then(|s| s.get("title"))
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty());

    let mut call = tool_call_block(ToolSource::OpenCode, call_id.clone(), name, &input, || {
        locator.content_ref("/state/input")
    });
    let mut kind = ToolKind::Other;
    if let SessionBlock::ToolCall {
        kind: call_kind,
        title,
        detail,
        diff,
        ..
    } = &mut call
    {
        kind = *call_kind;
        if let Some(summary) = metadata.and_then(|meta| files_diff(meta, locator)) {
            // v2 edit/patch 与 v1 apply_patch 的逐文件统计
            *title = title_path(&summary.files[0].path);
            *detail = (summary.files.len() > 1).then(|| format!("{} 个文件", summary.files.len()));
            *diff = Some(summary);
        } else {
            // v1 edit/write 的 state.title 是相对项目的路径，比参数里的绝对路径更短
            if matches!(kind, ToolKind::Edit | ToolKind::Write) || title.is_empty() {
                if let Some(t) = state_title {
                    *title = title_path(t);
                }
            }
            if let Some(meta) = metadata {
                apply_v1_diff_metadata(meta, kind, &input, title, diff, locator);
            }
        }
    }
    blocks.push(call);

    let status_raw = state.and_then(|s| s.get("status")).and_then(Value::as_str);
    let error_value = state.and_then(|s| s.get("error"));
    // v1 是字符串，v2 是 `{type, message}`
    let (error, error_pointer) = match error_value {
        Some(Value::String(e)) => (e.as_str(), "/state/error"),
        Some(e) => (
            e.get("message").and_then(Value::as_str).unwrap_or(""),
            "/state/error/message",
        ),
        None => ("", "/state/error"),
    };
    let interrupted = matches!(
        error_value
            .and_then(|e| e.get("type"))
            .and_then(Value::as_str),
        Some("aborted" | "tool.interrupted")
    ) || error.to_ascii_lowercase().contains("abort");
    let exit = metadata
        .and_then(|m| m.get("exit"))
        .and_then(Value::as_i64)
        .and_then(|code| i32::try_from(code).ok());
    // 和官方桌面端一致：shell 正常结束但退出码非 0 或超时，视为失败
    let shell_failed = kind == ToolKind::Shell
        && (exit.is_some_and(|code| code != 0)
            || metadata
                .and_then(|m| m.get("timeout"))
                .and_then(Value::as_bool)
                == Some(true));
    let status = match status_raw {
        Some("completed") if shell_failed => ToolStatus::Error,
        Some("completed") => ToolStatus::Success,
        Some("error") if interrupted => ToolStatus::Interrupted,
        Some("error") => ToolStatus::Error,
        Some("running" | "pending" | "streaming") => ToolStatus::Pending,
        _ => ToolStatus::Unknown,
    };
    let (output, output_pointer) = match state.and_then(|s| s.get("output")) {
        Some(Value::String(output)) => (output.clone(), "/state/output"),
        _ => (
            state
                .and_then(|s| s.get("content"))
                .map(content_text)
                .unwrap_or_default(),
            "/state/content",
        ),
    };
    let (text, pointer) = if !error.is_empty() && (status == ToolStatus::Error || output.is_empty())
    {
        (error, error_pointer)
    } else {
        (output.as_str(), output_pointer)
    };
    let mut result = tool_result_block(call_id, status, text, || locator.content_ref(pointer));
    if let SessionBlock::ToolResult {
        exit_code,
        duration_ms,
        images,
        ..
    } = &mut result
    {
        *exit_code = exit;
        *duration_ms = state.and_then(time_span).or_else(|| time_span(part));
        *images = content_images(state, locator);
    }
    blocks.push(result);
}

/// `metadata.files[]` → 逐文件改动摘要。v2 是 `{file, patch, additions, deletions, status}`，
/// v1 `apply_patch` 是 `{filePath, relativePath, type, patch, additions, deletions}`。
fn files_diff(meta: &Value, locator: &PartLocator) -> Option<DiffSummary> {
    let entries = meta.get("files")?.as_array()?;
    let files: Vec<DiffFile> = entries
        .iter()
        .filter_map(|entry| {
            let path = str_field(entry, &["file", "relativePath", "filePath"])?;
            let op = match entry
                .get("status")
                .or_else(|| entry.get("type"))
                .and_then(Value::as_str)
            {
                Some("added" | "add") => DiffOp::Add,
                Some("deleted" | "delete") => DiffOp::Delete,
                Some("move") => DiffOp::Rename,
                _ => DiffOp::Update,
            };
            let (added, removed) = match (
                entry.get("additions").and_then(Value::as_u64),
                entry.get("deletions").and_then(Value::as_u64),
            ) {
                (Some(a), Some(d)) => (saturate(a), saturate(d)),
                _ => entry
                    .get("patch")
                    .and_then(Value::as_str)
                    .map(count_diff_lines)
                    .unwrap_or_default(),
            };
            Some(DiffFile {
                path: path.to_string(),
                op,
                added,
                removed,
            })
        })
        .collect();
    if files.is_empty() {
        return None;
    }
    let full = if entries.len() == 1 && entries[0].get("patch").and_then(Value::as_str).is_some() {
        locator.content_ref("/state/metadata/files/0/patch")
    } else if meta.get("diff").and_then(Value::as_str).is_some() {
        locator.content_ref("/state/metadata/diff")
    } else {
        None
    };
    Some(DiffSummary {
        added: files.iter().fold(0, |sum, f| sum.saturating_add(f.added)),
        removed: files.iter().fold(0, |sum, f| sum.saturating_add(f.removed)),
        files,
        full,
    })
}

/// v1 edit/write 的 metadata：`filediff{additions, deletions}` / `diff` 字符串 / `exists`。
fn apply_v1_diff_metadata(
    meta: &Value,
    kind: ToolKind,
    input: &Value,
    title: &str,
    diff: &mut Option<DiffSummary>,
    locator: &PartLocator,
) {
    let path = str_field(input, &["filePath", "file_path", "path"]).unwrap_or(title);
    let op = if kind == ToolKind::Write {
        if meta.get("exists").and_then(Value::as_bool) == Some(true) {
            DiffOp::Update
        } else {
            DiffOp::Add
        }
    } else {
        DiffOp::Update
    };
    let counts = match (
        meta.pointer("/filediff/additions").and_then(Value::as_u64),
        meta.pointer("/filediff/deletions").and_then(Value::as_u64),
    ) {
        (Some(a), Some(d)) => Some((saturate(a), saturate(d))),
        _ => meta
            .get("diff")
            .and_then(Value::as_str)
            .map(count_diff_lines),
    };
    if let Some((added, removed)) = counts {
        let mut summary = single_file_diff(path, op, added, removed);
        if meta.get("diff").and_then(Value::as_str).is_some() {
            summary.full = locator.content_ref("/state/metadata/diff");
        }
        *diff = Some(summary);
    } else if let Some(d) = diff.as_mut() {
        for file in &mut d.files {
            file.op = op;
        }
    }
}

/// v2 `content[]` 的文本项按 `\n` 拼接（与官方展示及 [`ContentRef`] 取全文的结果一致）。
fn content_text(content: &Value) -> String {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}

/// v2 `content[]` 里的图片文件项（`{type:"file", uri, mime, name?}`）。
fn content_images(state: Option<&Value>, locator: &PartLocator) -> Vec<ImageRef> {
    state
        .and_then(|s| s.get("content"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, item)| item.get("type").and_then(Value::as_str) == Some("file"))
        .filter_map(|(i, item)| {
            file_image(
                item.get("mime").and_then(Value::as_str).unwrap_or(""),
                item.get("uri").and_then(Value::as_str).unwrap_or(""),
                item.get("name").and_then(Value::as_str),
                || locator.content_ref(&format!("/state/content/{i}/uri")),
            )
        })
        .collect()
}

/// `patch{hash, files[]}`：一步结束时的文件快照，只有文件列表、没有行数（待核实）。
fn push_patch_blocks(blocks: &mut Vec<SessionBlock>, part: &Value) {
    let files: Vec<&str> = part
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let Some(first) = files.first() else {
        return;
    };
    let id = part
        .get("id")
        .or_else(|| part.get("hash"))
        .and_then(Value::as_str)
        .unwrap_or("patch")
        .to_string();
    let input = part.get("files").cloned().unwrap_or(Value::Null);
    let mut call = tool_call_block(ToolSource::OpenCode, id.clone(), "patch", &input, || None);
    if let SessionBlock::ToolCall {
        title,
        detail,
        diff,
        ..
    } = &mut call
    {
        *title = title_path(first);
        *detail = (files.len() > 1).then(|| format!("{} 个文件", files.len()));
        let mut summary = single_file_diff(first, DiffOp::Update, 0, 0);
        summary.files = files
            .iter()
            .map(|path| single_file_diff(path, DiffOp::Update, 0, 0).files.remove(0))
            .collect();
        *diff = Some(summary);
    }
    blocks.push(call);
    blocks.push(tool_result_block(
        id,
        ToolStatus::Success,
        &files.join("\n"),
        || None,
    ));
}

/// `file{mime, filename, url}`：图片 → Image，其余 → 文件名事件（字段待核实）。
fn push_file_block(blocks: &mut Vec<SessionBlock>, locator: &PartLocator, part: &Value) {
    let mime = part.get("mime").and_then(Value::as_str).unwrap_or("");
    let url = part.get("url").and_then(Value::as_str).unwrap_or("");
    let filename = part
        .get("filename")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if let Some(image) = file_image(mime, url, filename, || locator.content_ref("/url")) {
        blocks.push(SessionBlock::Image { image });
        return;
    }
    if let Some(text) = filename.or_else(|| (!url.is_empty()).then_some(url)) {
        blocks.push(SessionBlock::event(
            EventKind::Other,
            Some(text.to_string()),
            None,
        ));
    }
}

/// 图片文件 → [`ImageRef`]：`file://` → 本地文件；`data:` URL → 内联引用（`content` 指向整串 URL，
/// 加载时去掉前缀）；不是图片或无法定位时返回 None。
fn file_image(
    mime: &str,
    url: &str,
    name: Option<&str>,
    content: impl FnOnce() -> Option<ContentRef>,
) -> Option<ImageRef> {
    if !mime.starts_with("image/") {
        return None;
    }
    let (source, size) = if let Some(path) = url.strip_prefix("file://") {
        (ImageSource::LocalFile { path: path.into() }, 0)
    } else if url.starts_with("data:") {
        let base64_len = url.split_once(',').map_or(0, |(_, data)| data.len());
        (
            ImageSource::Inline {
                content: content()?,
            },
            estimate_base64_size(base64_len),
        )
    } else {
        return None;
    };
    Some(ImageRef {
        source,
        media_type: mime.to_string(),
        size,
        alt: name.filter(|s| !s.is_empty()).map(str::to_string),
    })
}

/// 消息级错误：v1 `{name, data{message}}`，v2 `{type, message}`。中断 → Aborted，其余 → Error。
fn message_error_event(info: &Value) -> Option<SessionBlock> {
    let error = info.get("error")?;
    let name = error
        .get("name")
        .or_else(|| error.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if matches!(name, "MessageAbortedError" | "aborted") {
        return Some(SessionBlock::event(EventKind::Aborted, None, None));
    }
    let text = error
        .pointer("/data/message")
        .or_else(|| error.get("message"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(name);
    Some(SessionBlock::event(
        EventKind::Error,
        (!text.is_empty()).then(|| text.to_string()),
        None,
    ))
}

/// 消息级 `cost/tokens/modelID/providerID/time/finish` → meta（0 视为缺省）。
/// v2 的模型在 `model{id, providerID}`。
fn message_meta(info: &Value) -> Option<MessageMeta> {
    let tokens = info.get("tokens");
    let count = |pointer: &str| {
        tokens
            .and_then(|t| t.pointer(pointer))
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
    };
    let model_id = info
        .get("modelID")
        .or_else(|| info.pointer("/model/id"))
        .and_then(Value::as_str);
    let provider_id = info
        .get("providerID")
        .or_else(|| info.pointer("/model/providerID"))
        .and_then(Value::as_str);
    let model = match (provider_id, model_id) {
        (Some(p), Some(m)) if !p.is_empty() && !m.is_empty() => Some(format!("{p}/{m}")),
        (_, Some(m)) if !m.is_empty() => Some(m.to_string()),
        _ => None,
    };
    let created = info
        .pointer("/time/created")
        .and_then(parse_timestamp_to_ms);
    let completed = info
        .pointer("/time/completed")
        .and_then(parse_timestamp_to_ms);
    let meta = MessageMeta {
        model,
        input_tokens: count("/input"),
        output_tokens: count("/output"),
        cache_read_tokens: count("/cache/read"),
        cache_write_tokens: count("/cache/write"),
        reasoning_tokens: count("/reasoning"),
        cost_usd: info
            .get("cost")
            .and_then(Value::as_f64)
            .filter(|c| *c > 0.0),
        duration_ms: match (created, completed) {
            (Some(start), Some(end)) if end >= start => u64::try_from(end - start).ok(),
            _ => None,
        },
        stop_reason: info
            .get("finish")
            .and_then(Value::as_str)
            .map(str::to_string),
    };
    (meta != MessageMeta::default()).then_some(meta)
}

/// step-finish 的 tokens：有 `total` 用之，否则把各项相加（本机数据没有 total）。
fn total_tokens(tokens: &Value) -> Option<u64> {
    if let Some(total) = tokens.get("total").and_then(Value::as_u64) {
        return Some(total);
    }
    let sum: u64 = [
        "/input",
        "/output",
        "/reasoning",
        "/cache/read",
        "/cache/write",
    ]
    .iter()
    .filter_map(|p| tokens.pointer(p).and_then(Value::as_u64))
    .sum();
    (sum > 0).then_some(sum)
}

/// 毫秒时长：v1 `time{start, end}`；v2 `time{created, ran?, completed?}`，工具从 `ran` 起算。
fn time_span(value: &Value) -> Option<u64> {
    let time = value.get("time")?;
    let at = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| time.get(*key).and_then(Value::as_i64))
    };
    let start = at(&["start", "ran", "created"])?;
    let end = at(&["end", "completed"])?;
    u64::try_from(end - start).ok()
}

fn part_duration(part: &Value) -> Option<u64> {
    time_span(part).filter(|ms| *ms > 0)
}

fn saturate(n: u64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::model::ToolKind;
    use serde_json::json;

    fn sqlite(id: &str) -> PartLocator {
        PartLocator::Sqlite {
            table: "part",
            id: id.into(),
            base: String::new(),
        }
    }

    #[test]
    fn tool_state_maps_to_call_and_result() {
        let long_output: String = (1..=40).map(|i| format!("{i:05}| line\n")).collect();
        let parts = vec![
            (
                sqlite("prt_1"),
                json!({"type":"tool","callID":"c1","tool":"bash","state":{
                    "status":"error","input":{"command":"bunx biome --version","description":"Check biome version"},
                    "output":"error: could not determine executable","title":"Check biome version",
                    "metadata":{"exit":1},"time":{"start":1000,"end":2312}}}),
            ),
            (
                sqlite("prt_2"),
                json!({"type":"tool","callID":"c2","tool":"read","state":{
                    "status":"completed","input":{"filePath":"/p/package.json"},
                    "output": long_output,"title":"package.json","time":{"start":1,"end":22}}}),
            ),
            (
                sqlite("prt_3"),
                json!({"type":"tool","callID":"c3","tool":"task","state":{
                    "status":"running","input":{"description":"Find configs","subagent_type":"explore"}}}),
            ),
            (
                sqlite("prt_4"),
                json!({"type":"tool","callID":"c4","tool":"bash","state":{
                    "status":"error","input":{"command":"sleep 100"},"error":"Tool execution aborted"}}),
            ),
        ];
        let msg = message_from_parts("assistant", Some("m".into()), None, &json!({}), &parts);
        let b = &msg.blocks;
        assert_eq!(b.len(), 8);

        match &b[0] {
            SessionBlock::ToolCall {
                kind,
                title,
                detail,
                ..
            } => {
                assert_eq!(*kind, ToolKind::Shell);
                assert_eq!(title, "bunx biome --version");
                assert_eq!(detail.as_deref(), Some("Check biome version"));
            }
            other => panic!("{other:?}"),
        }
        match &b[1] {
            SessionBlock::ToolResult {
                call_id,
                status,
                exit_code,
                duration_ms,
                ..
            } => {
                assert_eq!(call_id, "c1");
                assert_eq!(*status, ToolStatus::Error);
                assert_eq!(*exit_code, Some(1));
                assert_eq!(*duration_ms, Some(1312));
            }
            other => panic!("{other:?}"),
        }
        match &b[3] {
            SessionBlock::ToolResult {
                status,
                truncated,
                full,
                line_count,
                ..
            } => {
                assert_eq!(*status, ToolStatus::Success);
                assert!(*truncated);
                assert_eq!(*line_count, 40);
                assert_eq!(
                    full,
                    &Some(ContentRef::Sqlite {
                        table: "part".into(),
                        id: "prt_2".into(),
                        column: "data".into(),
                        pointer: "/state/output".into(),
                    })
                );
            }
            other => panic!("{other:?}"),
        }
        match (&b[4], &b[5]) {
            (
                SessionBlock::ToolCall {
                    kind,
                    title,
                    detail,
                    ..
                },
                SessionBlock::ToolResult { status, .. },
            ) => {
                assert_eq!(*kind, ToolKind::Agent);
                assert_eq!(title, "Find configs");
                assert_eq!(detail.as_deref(), Some("explore"));
                assert_eq!(*status, ToolStatus::Pending);
            }
            other => panic!("{other:?}"),
        }
        match &b[7] {
            SessionBlock::ToolResult {
                status, preview, ..
            } => {
                assert_eq!(*status, ToolStatus::Interrupted);
                assert_eq!(preview, "Tool execution aborted");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn steps_reasoning_agent_and_meta() {
        let parts = vec![
            (sqlite("p0"), json!({"type":"agent","name":"explore"})),
            (sqlite("p1"), json!({"type":"step-start"})),
            (
                sqlite("p2"),
                json!({"type":"reasoning","text":"先确认 biome 是否已安装。","time":{"start":10,"end":1810}}),
            ),
            (
                sqlite("p3"),
                json!({"type":"reasoning","text":"","metadata":{"openai":{"reasoningEncryptedContent":"gAAA"}}}),
            ),
            (
                sqlite("p4"),
                json!({"type":"text","text":" Use the above message","synthetic":true}),
            ),
            (sqlite("p5"), json!({"type":"text","text":"done"})),
            (
                sqlite("p6"),
                json!({"type":"step-finish","reason":"stop","cost":0.0042,
                       "tokens":{"input":100,"output":10,"reasoning":0,"cache":{"read":22300,"write":0}}}),
            ),
            (
                sqlite("p7"),
                json!({"type":"step-finish","reason":"tool-calls","cost":0.0123,"tokens":{"total":18734}}),
            ),
        ];
        let info = json!({
            "role":"assistant","modelID":"claude-sonnet-4-5","providerID":"anthropic",
            "cost":0.0246,"finish":"stop","time":{"created":1791014404000_i64,"completed":1791014418200_i64},
            "tokens":{"input":61204,"output":1900,"reasoning":96,"cache":{"read":52100,"write":3010}}
        });
        let msg = message_from_parts("assistant", None, Some(1000), &info, &parts);
        assert_eq!(msg.content, "@explore\n\ndone");
        assert!(matches!(
            &msg.blocks[0],
            SessionBlock::Event { kind: EventKind::Other, text: Some(t), .. } if t == "@explore"
        ));
        assert!(matches!(
            &msg.blocks[1],
            SessionBlock::Step {
                phase: StepPhase::Start,
                ..
            }
        ));
        assert!(matches!(
            &msg.blocks[2],
            SessionBlock::Thinking {
                duration_ms: Some(1800),
                redacted: false,
                ..
            }
        ));
        assert!(matches!(
            &msg.blocks[3],
            SessionBlock::Thinking { redacted: true, .. }
        ));
        assert_eq!(
            msg.blocks[5],
            SessionBlock::Step {
                phase: StepPhase::Finish,
                tokens: Some(22410),
                cost_usd: Some(0.0042),
                reason: Some("stop".into()),
            }
        );
        assert!(matches!(
            &msg.blocks[6],
            SessionBlock::Step {
                tokens: Some(18734),
                ..
            }
        ));
        let meta = msg.meta.expect("meta");
        assert_eq!(meta.model.as_deref(), Some("anthropic/claude-sonnet-4-5"));
        assert_eq!(meta.cache_write_tokens, Some(3010));
        assert_eq!(meta.cost_usd, Some(0.0246));
        assert_eq!(meta.duration_ms, Some(14200));
        assert_eq!(meta.stop_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn edit_diff_patch_file_and_message_errors() {
        let parts = vec![
            (
                PartLocator::File {
                    rel_path: "part/msg_1/prt_1.json".into(),
                },
                json!({"type":"tool","callID":"e1","tool":"edit","state":{
                    "status":"completed","title":"package.json",
                    "input":{"filePath":"/p/package.json","oldString":"a","newString":"a,\nb"},
                    "output":"","metadata":{"diff":"--- a\n+++ b\n-a\n+a,\n+b\n"}}}),
            ),
            (
                sqlite("p2"),
                json!({"type":"patch","hash":"abc","files":["/p/a.ts","/p/b.ts"]}),
            ),
            (
                sqlite("p3"),
                json!({"type":"file","mime":"image/png","filename":"shot.png","url":"data:image/png;base64,AAAAAAAA"}),
            ),
            (
                sqlite("p4"),
                json!({"type":"file","mime":"text/plain","filename":"notes.txt","url":"file:///p/notes.txt"}),
            ),
        ];
        let info = json!({"error":{"name":"MessageAbortedError","data":{"message":"aborted"}}});
        let msg = message_from_parts("assistant", None, None, &info, &parts);
        match &msg.blocks[0] {
            SessionBlock::ToolCall {
                kind, title, diff, ..
            } => {
                assert_eq!(*kind, ToolKind::Edit);
                assert_eq!(title, "package.json");
                let diff = diff.as_ref().expect("diff");
                assert_eq!((diff.added, diff.removed), (2, 1));
                assert_eq!(diff.files[0].path, "/p/package.json");
                assert_eq!(
                    diff.full,
                    Some(ContentRef::File {
                        rel_path: "part/msg_1/prt_1.json".into(),
                        pointer: "/state/metadata/diff".into(),
                    })
                );
            }
            other => panic!("{other:?}"),
        }
        match &msg.blocks[2] {
            SessionBlock::ToolCall {
                kind,
                title,
                detail,
                diff,
                ..
            } => {
                assert_eq!(*kind, ToolKind::Edit);
                assert_eq!(title, "/p/a.ts");
                assert_eq!(detail.as_deref(), Some("2 个文件"));
                assert_eq!(diff.as_ref().map(|d| d.files.len()), Some(2));
            }
            other => panic!("{other:?}"),
        }
        match &msg.blocks[4] {
            SessionBlock::Image { image } => {
                assert_eq!(image.media_type, "image/png");
                assert_eq!(image.size, 6);
                assert!(matches!(
                    &image.source,
                    ImageSource::Inline { content: ContentRef::Sqlite { pointer, .. } } if pointer == "/url"
                ));
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            &msg.blocks[5],
            SessionBlock::Event { kind: EventKind::Other, text: Some(t), .. } if t == "notes.txt"
        ));
        assert!(matches!(
            msg.blocks.last(),
            Some(SessionBlock::Event {
                kind: EventKind::Aborted,
                ..
            })
        ));

        let api_error =
            json!({"error":{"name":"APIError","data":{"message":"Model not supported"}}});
        let msg = message_from_parts("assistant", None, None, &api_error, &[]);
        assert_eq!(msg.content, "Model not supported");
    }

    #[test]
    fn v2_tool_item_without_state_is_unknown() {
        let locator = PartLocator::Sqlite {
            table: "session_message",
            id: "msg_2".into(),
            base: "/content/1".into(),
        };
        let parts = vec![(locator, json!({"type":"tool","name":"shell","id":"call_1"}))];
        let msg = message_from_parts("assistant", None, None, &json!({}), &parts);
        assert!(matches!(
            &msg.blocks[0],
            SessionBlock::ToolCall { kind: ToolKind::Shell, title, .. } if title.is_empty()
        ));
        assert!(matches!(
            &msg.blocks[1],
            SessionBlock::ToolResult { call_id, status: ToolStatus::Unknown, .. } if call_id == "call_1"
        ));
    }

    fn v2(i: usize) -> PartLocator {
        PartLocator::Sqlite {
            table: "session_message",
            id: "msg_2".into(),
            base: format!("/content/{i}"),
        }
    }

    /// 字段形状取自 v2.0.24 `packages/schema/src/session-message.ts` 与各工具插件的返回值
    #[test]
    fn v2_assistant_content_follows_session_message_schema() {
        let long_output: String = (1..=40).map(|i| format!("file_{i:02}.rs\n")).collect();
        let items = vec![
            json!({"type":"reasoning","text":"先列目录","state":{"anthropic":{"signature":"sig"}},
                   "time":{"created":1791014401000_i64,"completed":1791014402500_i64}}),
            json!({"type":"tool","id":"t_ls","name":"shell","executed":false,
                   "state":{"status":"completed","input":{"command":"ls src"},
                            "content":[{"type":"text","text":long_output}],
                            "metadata":{"status":"completed","truncated":false,"exit":0}},
                   "time":{"created":1791014402600_i64,"ran":1791014402700_i64,"completed":1791014403100_i64}}),
            json!({"type":"tool","id":"t_edit","name":"edit",
                   "state":{"status":"completed","input":{"path":"src/a.ts","oldString":"a","newString":"b"},
                            "content":[{"type":"text","text":"Edited src/a.ts (1 replacement)"}],
                            "metadata":{"files":[{"file":"src/a.ts","patch":"--- a\n+++ b\n-a\n+b\n",
                                                  "additions":1,"deletions":1,"status":"modified"}]}},
                   "time":{"created":1791014403200_i64,"completed":1791014403500_i64}}),
            json!({"type":"tool","id":"t_sleep","name":"shell",
                   "state":{"status":"error","input":{"command":"sleep 100"},
                            "error":{"type":"aborted","message":"Tool execution interrupted"},
                            "metadata":{"shellID":"sh_1"}},
                   "time":{"created":1791014403600_i64}}),
            json!({"type":"tool","id":"t_test","name":"shell",
                   "state":{"status":"completed","input":{"command":"cargo test"},
                            "content":[{"type":"text","text":"boom"},{"type":"text","text":"Exited with code 101"}],
                            "metadata":{"truncated":false,"exit":101}},
                   "time":{"created":1791014404000_i64}}),
            json!({"type":"tool","id":"t_patch","name":"patch",
                   "state":{"status":"completed","input":{"patchText":"*** Begin Patch\n…"},
                            "content":[{"type":"text","text":"Applied patch"}],
                            "metadata":{"files":[
                                {"file":"src/new.ts","patch":"+x\n+y\n","additions":2,"deletions":0,"status":"added"},
                                {"file":"src/old.ts","patch":"-z\n","additions":0,"deletions":1,"status":"deleted"}]}},
                   "time":{"created":1791014405000_i64}}),
            json!({"type":"tool","id":"t_read","name":"read",
                   "state":{"status":"completed","input":{"path":"shot.png"},
                            "content":[{"type":"text","text":"Image read"},
                                       {"type":"file","uri":"data:image/png;base64,AAAAAAAA","mime":"image/png","name":"shot.png"}]},
                   "time":{"created":1791014406000_i64}}),
            json!({"type":"tool","id":"t_missing","name":"read",
                   "state":{"status":"error","input":{"path":"nope.ts"},
                            "error":{"type":"tool.execution","message":"File not found: nope.ts"}},
                   "time":{"created":1791014407000_i64}}),
            json!({"type":"tool","id":"t_sub","name":"subagent",
                   "state":{"status":"running","input":{"agent":"explore","description":"Find configs","prompt":"…"},
                            "metadata":{}},
                   "time":{"created":1791014408000_i64}}),
            json!({"type":"tool","id":"t_stream","name":"write",
                   "state":{"status":"streaming","input":"{\"path\":\"src/b.ts\",\"cont"},
                   "time":{"created":1791014409000_i64}}),
        ];
        let parts: Vec<_> = items
            .into_iter()
            .enumerate()
            .map(|(i, item)| (v2(i), item))
            .collect();
        let info = json!({
            "agent":"build","model":{"id":"claude-sonnet-4-5","providerID":"anthropic","variant":"default"},
            "finish":"tool-calls","cost":0.0123,
            "tokens":{"input":812,"output":210,"reasoning":96,"cache":{"read":22300,"write":0}},
            "error":{"type":"aborted","message":"Step interrupted"},
            "time":{"created":1791014401000_i64,"completed":1791014409100_i64}
        });
        let msg = message_from_parts("assistant", Some("msg_2".into()), None, &info, &parts);
        let b = &msg.blocks;
        let result = |i: usize| match &b[i] {
            SessionBlock::ToolResult {
                status,
                preview,
                full,
                exit_code,
                duration_ms,
                images,
                ..
            } => (
                *status,
                preview.as_str(),
                full.clone(),
                *exit_code,
                *duration_ms,
                images.clone(),
            ),
            other => panic!("{other:?}"),
        };

        assert!(matches!(
            &b[0],
            SessionBlock::Thinking {
                duration_ms: Some(1500),
                ..
            }
        ));

        assert!(
            matches!(&b[1], SessionBlock::ToolCall { kind: ToolKind::Shell, title, .. } if title == "ls src")
        );
        let (status, preview, full, exit, duration, _) = result(2);
        assert_eq!(status, ToolStatus::Success);
        assert!(preview.starts_with("file_01.rs\nfile_02.rs"));
        assert_eq!(
            full,
            Some(ContentRef::Sqlite {
                table: "session_message".into(),
                id: "msg_2".into(),
                column: "data".into(),
                pointer: "/content/1/state/content".into(),
            })
        );
        assert_eq!((exit, duration), (Some(0), Some(400)));

        match &b[3] {
            SessionBlock::ToolCall {
                kind, title, diff, ..
            } => {
                assert_eq!(*kind, ToolKind::Edit);
                assert_eq!(title, "src/a.ts");
                let diff = diff.as_ref().expect("diff");
                assert_eq!((diff.added, diff.removed), (1, 1));
                assert_eq!(diff.files[0].op, DiffOp::Update);
                assert!(matches!(
                    &diff.full,
                    Some(ContentRef::Sqlite { pointer, .. }) if pointer == "/content/2/state/metadata/files/0/patch"
                ));
            }
            other => panic!("{other:?}"),
        }
        let (status, preview, ..) = result(4);
        assert_eq!(
            (status, preview),
            (ToolStatus::Success, "Edited src/a.ts (1 replacement)")
        );

        let (status, preview, ..) = result(6);
        assert_eq!(
            (status, preview),
            (ToolStatus::Interrupted, "Tool execution interrupted")
        );

        let (status, preview, _, exit, ..) = result(8);
        assert_eq!(status, ToolStatus::Error);
        assert_eq!(preview, "boom\nExited with code 101");
        assert_eq!(exit, Some(101));

        match &b[9] {
            SessionBlock::ToolCall {
                kind,
                title,
                detail,
                diff,
                ..
            } => {
                assert_eq!(*kind, ToolKind::Edit);
                assert_eq!(title, "src/new.ts");
                assert_eq!(detail.as_deref(), Some("2 个文件"));
                let diff = diff.as_ref().expect("diff");
                let ops: Vec<_> = diff.files.iter().map(|f| f.op).collect();
                assert_eq!(ops, [DiffOp::Add, DiffOp::Delete]);
                assert_eq!((diff.added, diff.removed), (2, 1));
                assert_eq!(diff.full, None);
            }
            other => panic!("{other:?}"),
        }

        let (status, preview, .., images) = result(12);
        assert_eq!((status, preview), (ToolStatus::Success, "Image read"));
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].alt.as_deref(), Some("shot.png"));
        assert!(matches!(
            &images[0].source,
            ImageSource::Inline { content: ContentRef::Sqlite { pointer, .. } } if pointer == "/content/6/state/content/1/uri"
        ));

        let (status, preview, ..) = result(14);
        assert_eq!(
            (status, preview),
            (ToolStatus::Error, "File not found: nope.ts")
        );

        match &b[15] {
            SessionBlock::ToolCall {
                kind,
                title,
                detail,
                ..
            } => {
                assert_eq!(*kind, ToolKind::Agent);
                assert_eq!(title, "Find configs");
                assert_eq!(detail.as_deref(), Some("explore"));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(result(16).0, ToolStatus::Pending);

        // 写到一半的参数保持原字符串
        assert!(matches!(
            &b[17],
            SessionBlock::ToolCall { kind: ToolKind::Write, input_preview, .. } if input_preview.starts_with("{\"path\"")
        ));
        assert_eq!(result(18).0, ToolStatus::Pending);

        assert!(matches!(
            b.last(),
            Some(SessionBlock::Event {
                kind: EventKind::Aborted,
                ..
            })
        ));
        let meta = msg.meta.expect("meta");
        assert_eq!(meta.model.as_deref(), Some("anthropic/claude-sonnet-4-5"));
        assert_eq!(meta.duration_ms, Some(8100));

        let failed = json!({"error":{"type":"provider.error","message":"Overloaded"}});
        let msg = message_from_parts("assistant", None, None, &failed, &[]);
        assert_eq!(msg.content, "Overloaded");
    }

    #[test]
    fn v1_apply_patch_and_failed_bash() {
        let parts = vec![
            (
                sqlite("p1"),
                json!({"type":"tool","callID":"ap","tool":"apply_patch","state":{
                    "status":"completed","input":{"patchText":"*** Begin Patch"},
                    "output":"Success. Updated the following files:\nM a.ts\nR b.ts",
                    "title":"Success. Updated the following files:\nM a.ts\nR b.ts",
                    "metadata":{"diff":"--- a\n+++ b\n-a\n+a2\n+a3\n","files":[
                        {"filePath":"/p/a.ts","relativePath":"a.ts","type":"update","patch":"-a\n+a2\n+a3\n","additions":2,"deletions":1},
                        {"filePath":"/p/b.ts","relativePath":"c.ts","type":"move","movePath":"/p/c.ts","patch":"","additions":0,"deletions":0}]},
                    "time":{"start":1,"end":5}}}),
            ),
            (
                sqlite("p2"),
                json!({"type":"tool","callID":"bt","tool":"bash","state":{
                    "status":"completed","input":{"command":"false"},"output":"",
                    "metadata":{"exit":1},"time":{"start":1,"end":3}}}),
            ),
        ];
        let msg = message_from_parts("assistant", None, None, &json!({}), &parts);
        match &msg.blocks[0] {
            SessionBlock::ToolCall {
                kind,
                title,
                detail,
                diff,
                ..
            } => {
                assert_eq!(*kind, ToolKind::Edit);
                assert_eq!(title, "a.ts");
                assert_eq!(detail.as_deref(), Some("2 个文件"));
                let diff = diff.as_ref().expect("diff");
                let ops: Vec<_> = diff.files.iter().map(|f| f.op).collect();
                assert_eq!(ops, [DiffOp::Update, DiffOp::Rename]);
                assert!(matches!(
                    &diff.full,
                    Some(ContentRef::Sqlite { pointer, .. }) if pointer == "/state/metadata/diff"
                ));
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            &msg.blocks[3],
            SessionBlock::ToolResult {
                status: ToolStatus::Error,
                exit_code: Some(1),
                ..
            }
        ));
    }
}
