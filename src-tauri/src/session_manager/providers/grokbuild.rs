use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::session_manager::model::{ImageRef, SessionBlock, ToolStatus};
use crate::session_manager::{SessionMessage, SessionMeta};

use super::blocks::{
    assign_turn_ids, large_text_block, openai_tool_calls, parse_arguments, tool_call_block,
    tool_result_block, ToolSource,
};
use super::codex_items::image_from_url;
use super::utils::{
    extract_text, for_each_jsonl_value, parse_timestamp_to_ms, truncate_summary, JsonlSpan,
    TITLE_MAX_CHARS,
};

#[derive(Debug, Deserialize)]
struct GrokSessionInfo {
    id: String,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GrokSessionSummary {
    info: GrokSessionInfo,
    #[serde(default)]
    session_summary: Option<String>,
    #[serde(default)]
    generated_title: Option<String>,
    #[serde(default)]
    created_at: Option<Value>,
    #[serde(default)]
    updated_at: Option<Value>,
    #[serde(default)]
    last_active_at: Option<Value>,
    /// `subagent*` / `fork` / `worktree` 等；普通会话缺省
    #[serde(default)]
    session_kind: Option<String>,
    /// 显式的可见性覆盖
    #[serde(default)]
    hidden: Option<bool>,
}

impl GrokSessionSummary {
    /// 与 grok-build `Summary::is_hidden` 一致：显式 `hidden` 优先，否则子代理会话不进历史列表
    fn is_hidden(&self) -> bool {
        self.hidden.unwrap_or_else(|| {
            self.session_kind
                .as_deref()
                .is_some_and(|kind| kind.starts_with("subagent"))
        })
    }
}

pub fn session_roots() -> Vec<PathBuf> {
    let config_dir = crate::grok_config::get_grok_config_dir();
    vec![
        config_dir.join("sessions"),
        config_dir.join("archived_sessions"),
    ]
}

pub fn scan_sessions() -> Vec<SessionMeta> {
    let mut summaries = Vec::new();
    for root in session_roots() {
        collect_summary_files(&root, &mut summaries);
    }
    summaries
        .into_iter()
        .filter_map(|path| parse_summary(&path))
        .collect()
}

/// `chat_history.jsonl`：每行一个 `ConversationItem`（grok-build
/// `xai-grok-sampling-types/src/conversation.rs`），`type ∈ {system, user, assistant,
/// tool_result, backend_tool_call, reasoning}`。记录本身不带时间戳。
///
/// 会话的 sourcePath 是同目录的 `summary.json`；大内容的 Jsonl 引用指向 chat_history.jsonl
/// 的行（`content::resolve_content_ref` 对 grokbuild 固定改读该文件）。
pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let session_dir = path
        .parent()
        .ok_or_else(|| format!("Invalid Grok Build session path: {}", path.display()))?;
    let chat_path = session_dir.join("chat_history.jsonl");
    if !chat_path.is_file() {
        return Err(format!(
            "Failed to open Grok Build chat history: {}",
            chat_path.display()
        ));
    }
    let mut messages = Vec::new();

    for_each_jsonl_value(&chat_path, |span, value| {
        let (role, blocks, injected) = match value.get("type").and_then(Value::as_str) {
            // 开头的系统提示词，以及运行时追加的系统消息
            Some("system") => ("system", injected_text_blocks(&value, span), true),
            Some("user") => {
                let human = is_human_input(value.get("synthetic_reason").and_then(Value::as_str));
                let blocks = if human {
                    user_blocks(&value, span)
                } else {
                    injected_text_blocks(&value, span)
                };
                ("user", blocks, !human)
            }
            Some("assistant") => {
                let text = value.get("content").map(extract_text).unwrap_or_default();
                let mut blocks = Vec::new();
                if !text.trim().is_empty() {
                    blocks.push(SessionBlock::text(text));
                }
                // 官方形状是扁平的 `{id, name, arguments}`
                blocks.extend(openai_tool_calls(value.get("tool_calls"), |pointer| {
                    Some(span.content_ref(format!("/tool_calls{pointer}")))
                }));
                ("assistant", blocks, false)
            }
            Some("tool_result") => ("tool", tool_result_blocks(&value, span), false),
            Some("backend_tool_call") => ("assistant", backend_tool_blocks(&value, span), false),
            // reasoning 含加密的内部状态，Grok 自己的历史视图也不显示
            _ => return Ok(()),
        };
        let mut message = SessionMessage::from_blocks(role, None, blocks);
        if message.is_empty() {
            return Ok(());
        }
        message.injected = injected;
        messages.push(message);
        Ok(())
    })?;

    assign_turn_ids(&mut messages);
    Ok(messages)
}

/// user 条目是否是人输入的：`synthetic_reason` 缺省即 `human`；插话（Ctrl+Enter）、父会话转来的
/// 真人消息、`!cmd` 也算。其余（system_reminder、project_instructions、compaction_meta、
/// task_completed 等，含将来新增的未知值）都是运行时注入的。
fn is_human_input(synthetic_reason: Option<&str>) -> bool {
    matches!(
        synthetic_reason,
        None | Some("human" | "interjection" | "parent_human_message" | "direct_bash")
    )
}

/// 人输入的 user 条目：`content` 是 `[{type:"text",text} | {type:"image",url}]`
fn user_blocks(value: &Value, span: JsonlSpan) -> Vec<SessionBlock> {
    let parts = value.get("content");
    let text = parts.map(extract_text).unwrap_or_default();
    let mut blocks = Vec::new();
    if !text.trim().is_empty() {
        blocks.push(SessionBlock::text(text));
    }
    blocks.extend(
        content_images(parts, "/content", span)
            .into_iter()
            .map(|image| SessionBlock::Image { image }),
    );
    blocks
}

/// 注入内容（系统提示词、system reminder、项目说明等）默认折叠，超长时只放预览。
/// system 的 `content` 是字符串，user 的是 content part 数组。
fn injected_text_blocks(value: &Value, span: JsonlSpan) -> Vec<SessionBlock> {
    match value.get("content") {
        Some(Value::String(text)) if !text.trim().is_empty() => {
            vec![large_text_block(text.as_str(), || {
                Some(span.content_ref("/content"))
            })]
        }
        Some(Value::Array(parts)) => parts
            .iter()
            .enumerate()
            .filter_map(|(i, part)| {
                let text = part.get("text").and_then(Value::as_str)?;
                (!text.trim().is_empty()).then(|| {
                    large_text_block(text, || {
                        Some(span.content_ref(format!("/content/{i}/text")))
                    })
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// `{type:"tool_result", tool_call_id, content, images?}`：格式里没有成败状态
fn tool_result_blocks(value: &Value, span: JsonlSpan) -> Vec<SessionBlock> {
    let call_id = value
        .get("tool_call_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let text = value.get("content").map(extract_text).unwrap_or_default();
    let mut block = tool_result_block(call_id, ToolStatus::Unknown, &text, || {
        Some(span.content_ref("/content"))
    });
    if let SessionBlock::ToolResult { images, .. } = &mut block {
        *images = content_images(value.get("images"), "/images", span);
    }
    vec![block]
}

/// 服务端执行的工具（`{type:"backend_tool_call", kind:{tool_type, id, ...}}`）：
/// web_search 的 `action` 是 search（query + sources）/ open_page（url）/ find_in_page（url + pattern），
/// x_search 是 `{name, input}` 自定义调用，code_interpreter 带 `code`。
fn backend_tool_blocks(value: &Value, span: JsonlSpan) -> Vec<SessionBlock> {
    let Some(kind) = value.get("kind") else {
        return Vec::new();
    };
    let id = kind.get("id").and_then(Value::as_str).unwrap_or_default();
    let (name, input) = match kind.get("tool_type").and_then(Value::as_str) {
        Some("web_search") => {
            let mut action = kind.get("action").cloned().unwrap_or(Value::Null);
            if let Some(action) = action.as_object_mut() {
                action.remove("sources");
            }
            ("web_search", action)
        }
        Some("x_search") => (
            kind.get("name")
                .and_then(Value::as_str)
                .unwrap_or("x_search"),
            kind.get("input")
                .map(parse_arguments)
                .unwrap_or(Value::Null),
        ),
        Some("code_interpreter") => (
            "code_interpreter",
            serde_json::json!({ "code": kind.get("code") }),
        ),
        _ => return Vec::new(),
    };
    let mut blocks = vec![tool_call_block(
        ToolSource::Generic,
        id,
        name,
        &input,
        || Some(span.content_ref("/kind")),
    )];

    let sources: Vec<&str> = kind
        .pointer("/action/sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|source| source.get("url").and_then(Value::as_str))
        .collect();
    let status = match kind.get("status").and_then(Value::as_str) {
        Some("completed") => Some(ToolStatus::Success),
        Some("failed") => Some(ToolStatus::Error),
        _ => None,
    };
    if !sources.is_empty() || status.is_some() {
        blocks.push(tool_result_block(
            id,
            status.unwrap_or(ToolStatus::Unknown),
            &sources.join("\n"),
            || Some(span.content_ref("/kind/action/sources")),
        ));
    }
    blocks
}

/// content part 数组里的图片（`{type:"image", url}`，url 是 data URL 或本地路径）
fn content_images(parts: Option<&Value>, base: &str, span: JsonlSpan) -> Vec<ImageRef> {
    parts
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, part)| part.get("type").and_then(Value::as_str) == Some("image"))
        .filter_map(|(i, part)| {
            let url = part.get("url").and_then(Value::as_str)?;
            image_from_url(url, span, format!("{base}/{i}/url"))
        })
        .collect()
}

pub fn delete_session(root: &Path, path: &Path, session_id: &str) -> Result<bool, String> {
    if !path.starts_with(root) {
        return Err(format!(
            "Grok Build session source is outside the session root: {}",
            path.display()
        ));
    }
    if path.file_name().and_then(|name| name.to_str()) != Some("summary.json") {
        return Err(format!(
            "Unexpected Grok Build session source: {}",
            path.display()
        ));
    }
    let summary = read_summary(path)?;
    if summary.info.id != session_id {
        return Err(format!(
            "Grok Build session ID mismatch: expected {session_id}, found {}",
            summary.info.id
        ));
    }
    let session_dir = path
        .parent()
        .ok_or_else(|| format!("Invalid Grok Build session path: {}", path.display()))?;
    if session_dir == root || !session_dir.starts_with(root) {
        return Err(format!(
            "Refusing to delete Grok Build session directory outside its root: {}",
            session_dir.display()
        ));
    }
    if session_dir.file_name().and_then(|name| name.to_str()) != Some(session_id) {
        return Err(format!(
            "Grok Build session directory does not match session ID: {}",
            session_dir.display()
        ));
    }
    std::fs::remove_dir_all(session_dir).map_err(|e| {
        format!(
            "Failed to delete Grok Build session directory {}: {e}",
            session_dir.display()
        )
    })?;
    Ok(true)
}

fn collect_summary_files(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_summary_files(&path, files);
        } else if path.file_name().and_then(|name| name.to_str()) == Some("summary.json") {
            files.push(path);
        }
    }
}

fn read_summary(path: &Path) -> Result<GrokSessionSummary, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read Grok Build session summary: {e}"))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Grok Build session summary: {e}"))
}

fn parse_summary(path: &Path) -> Option<SessionMeta> {
    let summary = read_summary(path).ok().filter(|s| !s.is_hidden())?;
    let session_id = summary.info.id;
    let title = summary
        .generated_title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            summary
                .session_summary
                .as_deref()
                .filter(|value| !value.trim().is_empty())
        })
        .map(|value| truncate_summary(value, TITLE_MAX_CHARS));
    let session_summary = summary
        .session_summary
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| truncate_summary(value, 160));
    let created_at = summary.created_at.as_ref().and_then(parse_timestamp_to_ms);
    let last_active_at = summary
        .last_active_at
        .as_ref()
        .or(summary.updated_at.as_ref())
        .and_then(parse_timestamp_to_ms);

    Some(SessionMeta {
        provider_id: "grokbuild".to_string(),
        session_id: session_id.clone(),
        title,
        summary: session_summary,
        project_dir: summary.info.cwd,
        created_at,
        last_active_at,
        source_path: Some(path.to_string_lossy().to_string()),
        resume_command: Some(format!("grok --resume {session_id}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn scans_native_grokbuild_session_layout() {
        let temp = tempdir().expect("tempdir");
        let sessions_dir = temp.path().join("sessions");
        let session_id = "019f6af2-18b0-7673-958e-d25be650e172";
        let session_dir = sessions_dir.join("encoded-project").join(session_id);
        std::fs::create_dir_all(&session_dir).expect("create session dir");
        std::fs::write(
            session_dir.join("summary.json"),
            format!(
                r#"{{"info":{{"id":"{session_id}","cwd":"C:/work"}},"session_summary":"hello grok","generated_title":"Grok session","created_at":"2026-07-16T12:00:00Z","last_active_at":"2026-07-16T12:00:01Z"}}"#
            ),
        )
        .expect("write summary");
        let mut files = Vec::new();
        collect_summary_files(&sessions_dir, &mut files);
        let sessions = files
            .iter()
            .filter_map(|path| parse_summary(path))
            .collect::<Vec<_>>();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider_id, "grokbuild");
        assert_eq!(sessions[0].session_id, session_id);
        assert_eq!(sessions[0].title.as_deref(), Some("Grok session"));
        let expected_resume = format!("grok --resume {session_id}");
        assert_eq!(
            sessions[0].resume_command.as_deref(),
            Some(expected_resume.as_str())
        );
    }

    #[test]
    fn loads_native_grokbuild_chat_history() {
        let temp = tempdir().expect("tempdir");
        let summary_path = temp.path().join("summary.json");
        std::fs::write(&summary_path, "{}").expect("write summary placeholder");
        std::fs::write(
            temp.path().join("chat_history.jsonl"),
            concat!(
                "{\"type\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"hello\"}]}\n",
                "{\"type\":\"reasoning\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"private\"}]}\n",
                "{\"type\":\"assistant\",\"content\":\"Hi there\"}\n"
            ),
        )
        .expect("write chat history");

        let messages = load_messages(&summary_path).expect("load messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "hello");
        assert_eq!(messages[1].content, "Hi there");
    }

    #[test]
    fn delete_session_removes_only_the_matching_session_directory() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("sessions");
        let session_id = "session-to-delete";
        let session_dir = root.join("project").join(session_id);
        let sibling_dir = root.join("project").join("session-to-keep");
        std::fs::create_dir_all(&session_dir).expect("create session directory");
        std::fs::create_dir_all(&sibling_dir).expect("create sibling directory");
        let summary_path = session_dir.join("summary.json");
        std::fs::write(
            &summary_path,
            format!(r#"{{"info":{{"id":"{session_id}"}}}}"#),
        )
        .expect("write summary");
        std::fs::write(sibling_dir.join("keep.txt"), "keep").expect("write sibling file");

        let deleted = delete_session(&root, &summary_path, session_id).expect("delete session");

        assert!(deleted);
        assert!(!session_dir.exists());
        assert!(sibling_dir.exists());
    }

    #[test]
    fn delete_session_rejects_remove_dir_all_target_outside_root() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("sessions");
        let outside_dir = temp.path().join("outside").join("session-outside");
        std::fs::create_dir_all(&root).expect("create root");
        std::fs::create_dir_all(&outside_dir).expect("create outside directory");
        let summary_path = outside_dir.join("summary.json");
        std::fs::write(&summary_path, r#"{"info":{"id":"session-outside"}}"#)
            .expect("write summary");

        let error = delete_session(&root, &summary_path, "session-outside")
            .expect_err("outside path must be rejected");

        assert!(error.contains("outside the session root"));
        assert!(outside_dir.exists());
    }

    fn load_chat_history(lines: &[Value]) -> Vec<SessionMessage> {
        let temp = tempdir().expect("tempdir");
        let summary_path = temp.path().join("summary.json");
        std::fs::write(&summary_path, "{}").expect("write summary placeholder");
        std::fs::write(
            temp.path().join("chat_history.jsonl"),
            lines.iter().map(|l| format!("{l}\n")).collect::<String>(),
        )
        .expect("write chat history");
        load_messages(&summary_path).expect("load messages")
    }

    #[test]
    fn grokbuild_tool_results_pair_with_flat_tool_calls() {
        // 字段形状取自 grok-build ConversationItem 与本机 chat_history.jsonl
        let messages = load_chat_history(&[
            json!({"type": "system", "content": "You are Grok."}),
            json!({"type": "user", "content": [{"type": "text", "text": "clean journal"}], "prompt_index": 0}),
            json!({"type": "reasoning", "id": "rs_1", "encrypted_content": "x", "status": "completed", "summary": []}),
            json!({"type": "assistant", "content": "", "model_id": "grok-4.6", "tool_calls": [
                {"id": "call_1", "name": "run_terminal_cmd", "arguments": "{\"command\":\"journalctl --vacuum-time=3d\"}"}
            ]}),
            json!({"type": "tool_result", "tool_call_id": "call_1", "content": "Vacuuming done"}),
            json!({"type": "assistant", "content": "freed 2.1G"}),
        ]);

        assert_eq!(messages.len(), 5);
        assert!(messages[0].injected);
        assert_eq!(messages[0].turn_id.as_deref(), Some("t0"));
        assert_eq!(messages[1].content, "clean journal");
        assert_eq!(
            messages[2].content,
            "[Tool: run_terminal_cmd] journalctl --vacuum-time=3d"
        );
        assert_eq!(messages[3].role, "tool");
        match &messages[3].blocks[0] {
            SessionBlock::ToolResult {
                call_id,
                status,
                preview,
                ..
            } => {
                assert_eq!(call_id, "call_1");
                assert_eq!(*status, ToolStatus::Unknown);
                assert_eq!(preview, "Vacuuming done");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(messages[4].turn_id.as_deref(), Some("t1"));
    }

    #[test]
    fn grokbuild_synthetic_user_items_are_injected_and_do_not_start_turns() {
        let messages = load_chat_history(&[
            json!({"type": "user", "content": [{"type": "text", "text": "first"}]}),
            json!({"type": "user", "synthetic_reason": "project_instructions", "content": [{"type": "text", "text": "# AGENTS.md"}]}),
            json!({"type": "user", "synthetic_reason": "system_reminder", "content": [{"type": "text", "text": "<system-reminder>todo</system-reminder>"}]}),
            json!({"type": "assistant", "content": "ok"}),
            json!({"type": "user", "synthetic_reason": "interjection", "content": [{"type": "text", "text": "also this"}]}),
            json!({"type": "user", "synthetic_reason": "some_future_reason", "content": [{"type": "text", "text": "internal"}]}),
        ]);

        let flags: Vec<(bool, Option<&str>)> = messages
            .iter()
            .map(|m| (m.injected, m.turn_id.as_deref()))
            .collect();
        assert_eq!(
            flags,
            vec![
                (false, Some("t1")),
                (true, Some("t1")),
                (true, Some("t1")),
                (false, Some("t1")),
                (false, Some("t2")),
                (true, Some("t2")),
            ]
        );
    }

    #[test]
    fn grokbuild_backend_web_search_becomes_tool_call_with_sources() {
        let messages = load_chat_history(&[
            json!({"type": "user", "content": [{"type": "text", "text": "news?"}]}),
            json!({"type": "backend_tool_call", "kind": {
                "tool_type": "web_search", "id": "ws_1", "status": "completed",
                "action": {"type": "search", "query": "grok build", "sources": [
                    {"type": "url", "url": "https://x.ai/news"},
                    {"type": "url", "url": "https://github.com/xai-org/grok-build"}
                ]}
            }}),
            json!({"type": "backend_tool_call", "kind": {
                "tool_type": "web_search", "id": "ws_2", "status": "failed",
                "action": {"type": "open_page", "url": "https://example.com"}
            }}),
        ]);

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1].role, "assistant");
        match &messages[1].blocks[..] {
            [SessionBlock::ToolCall { id, title, .. }, SessionBlock::ToolResult {
                status, preview, ..
            }] => {
                assert_eq!(id, "ws_1");
                assert!(title.contains("grok build"), "{title}");
                assert_eq!(*status, ToolStatus::Success);
                assert_eq!(
                    preview,
                    "https://x.ai/news\nhttps://github.com/xai-org/grok-build"
                );
            }
            other => panic!("{other:?}"),
        }
        match &messages[2].blocks[..] {
            [SessionBlock::ToolCall { title, .. }, SessionBlock::ToolResult { status, .. }] => {
                assert!(title.contains("example.com"), "{title}");
                assert_eq!(*status, ToolStatus::Error);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn grokbuild_images_in_user_and_tool_result_items() {
        let messages = load_chat_history(&[
            json!({"type": "user", "content": [
                {"type": "text", "text": "what is this"},
                {"type": "image", "url": "data:image/png;base64,AAAABBBB"}
            ]}),
            json!({"type": "tool_result", "tool_call_id": "c1", "content": "read image",
                "images": [{"type": "image", "url": "data:image/jpeg;base64,AAAA"}]}),
        ]);

        assert!(matches!(
            &messages[0].blocks[..],
            [SessionBlock::Text { .. }, SessionBlock::Image { image }] if image.media_type == "image/png"
        ));
        match &messages[1].blocks[0] {
            SessionBlock::ToolResult { images, .. } => {
                assert_eq!(images.len(), 1);
                assert_eq!(images[0].media_type, "image/jpeg");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn subagent_sessions_are_hidden_from_the_list() {
        let temp = tempdir().expect("tempdir");
        let write = |id: &str, extra: &str| {
            let dir = temp.path().join("project").join(id);
            std::fs::create_dir_all(&dir).expect("create session dir");
            std::fs::write(
                dir.join("summary.json"),
                format!(r#"{{"info":{{"id":"{id}"}}{extra}}}"#),
            )
            .expect("write summary");
        };
        write("plain", "");
        write("sub", r#","session_kind":"subagent""#);
        write("sub-shown", r#","session_kind":"subagent","hidden":false"#);
        write("worktree", r#","session_kind":"worktree""#);

        let mut files = Vec::new();
        collect_summary_files(temp.path(), &mut files);
        let mut ids: Vec<String> = files
            .iter()
            .filter_map(|path| parse_summary(path))
            .map(|s| s.session_id)
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["plain", "sub-shown", "worktree"]);
    }
}
