use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use crate::openclaw_config::get_openclaw_dir;
use crate::{
    config::write_json_file,
    session_manager::{SessionMessage, SessionMeta},
};

use super::pi_blocks::PiTranscript;
use super::utils::for_each_jsonl_value;
use super::utils::{
    extract_text, parse_timestamp_to_ms, path_basename, read_head_tail_lines, truncate_summary,
    TITLE_MAX_CHARS,
};

const PROVIDER_ID: &str = "openclaw";

/// 渠道/时间信封前缀，如 `[Telegram 2026-03-01 10:14] `。
static ENVELOPE_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[([^\]]+)\]\s*").unwrap());
static ENVELOPE_TIMESTAMP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}Z\b|\d{4}-\d{2}-\d{2} \d{2}:\d{2}\b").unwrap()
});
const ENVELOPE_CHANNELS: [&str; 12] = [
    "WebChat",
    "WhatsApp",
    "Telegram",
    "Signal",
    "Slack",
    "Discord",
    "Google Chat",
    "iMessage",
    "Teams",
    "Matrix",
    "Zalo",
    "Zalo Personal",
];
static MESSAGE_ID_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*\[message_id:\s*[^\]]+\]\s*$").unwrap());

/// 去掉 OpenClaw 网关写进用户消息的元数据，规则同官方展示时用的
/// `stripEnvelope` 与 `stripMessageIdHints`（openclaw/openclaw
/// `src/shared/chat-envelope.ts`、`src/shared/text/message-id-hints.ts`）：
/// - 开头的渠道/时间信封，只认带时间戳或已知渠道名的，其他 `[...]` 可能是用户原文；
/// - 独立成行的 `[message_id: …]`（飞书放在开头，其他渠道放在末尾），代码块里的不动。
fn strip_gateway_metadata(text: &str) -> String {
    let text = match ENVELOPE_PREFIX_RE.captures(text) {
        Some(caps)
            if ENVELOPE_TIMESTAMP_RE.is_match(&caps[1])
                || ENVELOPE_CHANNELS
                    .iter()
                    .any(|label| caps[1].starts_with(&format!("{label} "))) =>
        {
            &text[caps[0].len()..]
        }
        _ => text,
    };
    if !text.to_ascii_lowercase().contains("[message_id:") {
        return text.to_string();
    }
    // 每行连同自己的换行符（`\n` 或 `\r\n`）一起处理，原样保留换行
    let lines: Vec<(&str, &str)> = text
        .split_inclusive('\n')
        .map(|piece| {
            let content = piece.trim_end_matches(['\r', '\n']);
            (content, &piece[content.len()..])
        })
        .collect();
    let tail_ending = lines.last().map_or("", |(_, ending)| *ending);
    let mut open_fence: Option<(u8, usize)> = None;
    let kept: Vec<(&str, &str)> = lines
        .into_iter()
        .filter(|(line, _)| match open_fence {
            Some((ch, len)) => {
                if fence_of(line)
                    .is_some_and(|(c, n, info)| c == ch && n >= len && info.trim().is_empty())
                {
                    open_fence = None;
                }
                true
            }
            None => {
                if let Some((ch, len, _)) = fence_of(line) {
                    open_fence = Some((ch, len));
                    return true;
                }
                !MESSAGE_ID_LINE_RE.is_match(line)
            }
        })
        .collect();
    // 删掉的是最后几行时，结尾换行跟原文一致
    let mut out = String::with_capacity(text.len());
    for (i, (line, ending)) in kept.iter().enumerate() {
        out.push_str(line);
        out.push_str(if i + 1 == kept.len() {
            tail_ending
        } else {
            ending
        });
    }
    out
}

/// 代码块围栏（CommonMark）：最多缩进 3 个空格，3 个以上的 ` 或 ~，返回字符、长度和
/// 后面的信息串。反引号围栏的信息串里不能再有反引号。结束围栏要用同一种字符、不短于
/// 开头的，后面只能有空白。
fn fence_of(line: &str) -> Option<(u8, usize, &str)> {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 {
        return None;
    }
    let ch = *rest
        .as_bytes()
        .first()
        .filter(|b| matches!(b, b'`' | b'~'))?;
    let len = rest.bytes().take_while(|b| *b == ch).count();
    let info = &rest[len..];
    (len >= 3 && !(ch == b'`' && info.contains('`'))).then_some((ch, len, info))
}

pub fn scan_sessions() -> Vec<SessionMeta> {
    let agents_dir = get_openclaw_dir().join("agents");
    if !agents_dir.exists() {
        return Vec::new();
    }

    let mut sessions = Vec::new();

    // Traverse each agent directory
    let agent_entries = match std::fs::read_dir(&agents_dir) {
        Ok(entries) => entries,
        Err(_) => return sessions,
    };

    for agent_entry in agent_entries.flatten() {
        let agent_path = agent_entry.path();
        if !agent_path.is_dir() {
            continue;
        }

        let sessions_dir = agent_path.join("sessions");
        if !sessions_dir.is_dir() {
            continue;
        }

        let session_entries = match std::fs::read_dir(&sessions_dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        let display_names = load_display_names(&sessions_dir);

        for entry in session_entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }

            if let Some(meta) = parse_session(&path, Some(&display_names)) {
                sessions.push(meta);
            }
        }
    }

    sessions
}

/// OpenClaw 与 Pi 同构，复用 Pi 的 block 映射；没有树形分支，按文件顺序全取。
pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let mut transcript = PiTranscript::new();
    for_each_jsonl_value(path, |span, mut value| {
        strip_user_text_metadata(&mut value);
        let id = value.get("id").and_then(Value::as_str).map(str::to_string);
        transcript.push_entry(&value, span, id);
        Ok(())
    })?;
    Ok(transcript.finish())
}

/// 对用户消息记录的正文（字符串或 `{type:"text"}` 项）就地执行
/// [`strip_gateway_metadata`]，再交给 Pi 的映射。
fn strip_user_text_metadata(value: &mut Value) {
    let Some(message) = value.get_mut("message") else {
        return;
    };
    if message.get("role").and_then(Value::as_str) != Some("user") {
        return;
    }
    match message.get_mut("content") {
        Some(Value::String(text)) => *text = strip_gateway_metadata(text),
        Some(Value::Array(items)) => {
            for item in items {
                if item.get("type").and_then(Value::as_str) == Some("text") {
                    if let Some(Value::String(text)) = item.get_mut("text") {
                        *text = strip_gateway_metadata(text);
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn delete_session(_root: &Path, path: &Path, session_id: &str) -> Result<bool, String> {
    let meta = parse_session(path, None).ok_or_else(|| {
        format!(
            "Failed to parse OpenClaw session metadata: {}",
            path.display()
        )
    })?;

    if meta.session_id != session_id {
        return Err(format!(
            "OpenClaw session ID mismatch: expected {session_id}, found {}",
            meta.session_id
        ));
    }

    let index_path = path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("sessions.json");
    prune_sessions_index(&index_path, session_id, path)?;

    std::fs::remove_file(path).map_err(|e| {
        format!(
            "Failed to delete OpenClaw session file {}: {e}",
            path.display()
        )
    })?;

    Ok(true)
}

/// Read `sessions.json` index and build a sessionId → displayName lookup map.
/// Returns an empty map if the file does not exist or cannot be parsed.
fn load_display_names(sessions_dir: &Path) -> HashMap<String, String> {
    let index_path = sessions_dir.join("sessions.json");
    let content = match std::fs::read_to_string(&index_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    let index: serde_json::Map<String, Value> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(_) => return HashMap::new(),
    };

    let mut map = HashMap::new();
    for (_key, entry) in &index {
        if let (Some(id), Some(name)) = (
            entry.get("sessionId").and_then(Value::as_str),
            entry.get("displayName").and_then(Value::as_str),
        ) {
            if !name.is_empty() {
                map.insert(id.to_string(), name.to_string());
            }
        }
    }
    map
}

fn parse_session(
    path: &Path,
    display_names: Option<&HashMap<String, String>>,
) -> Option<SessionMeta> {
    let (head, tail) = read_head_tail_lines(path, 10, 30).ok()?;

    let mut session_id: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut created_at: Option<i64> = None;
    let mut summary: Option<String> = None;
    let mut first_user_message: Option<String> = None;

    // Extract metadata, summary, and first user message from head lines
    for line in &head {
        let value: Value = match serde_json::from_str(line) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };

        if created_at.is_none() {
            created_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }

        let event_type = value.get("type").and_then(Value::as_str).unwrap_or("");

        if event_type == "session" {
            if session_id.is_none() {
                session_id = value
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
            }
            if cwd.is_none() {
                cwd = value
                    .get("cwd")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
            }
            if let Some(ts) = value.get("timestamp").and_then(parse_timestamp_to_ms) {
                created_at.get_or_insert(ts);
            }
            continue;
        }

        if event_type == "message" {
            if let Some(message) = value.get("message") {
                let text = message.get("content").map(extract_text).unwrap_or_default();
                let cleaned = strip_gateway_metadata(&text);
                if !cleaned.trim().is_empty() {
                    if first_user_message.is_none()
                        && message.get("role").and_then(Value::as_str) == Some("user")
                    {
                        first_user_message = Some(cleaned.trim().to_string());
                    }
                    if summary.is_none() {
                        summary = Some(cleaned.trim().to_string());
                    }
                }
            }
        }

        if session_id.is_some()
            && cwd.is_some()
            && created_at.is_some()
            && summary.is_some()
            && first_user_message.is_some()
        {
            break;
        }
    }

    // Extract last_active_at from tail lines (reverse order)
    let mut last_active_at: Option<i64> = None;
    for line in tail.iter().rev() {
        let value: Value = match serde_json::from_str(line) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        if let Some(ts) = value.get("timestamp").and_then(parse_timestamp_to_ms) {
            last_active_at = Some(ts);
            break;
        }
    }

    // Fall back to filename as session ID
    let session_id = session_id.or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    });
    let session_id = session_id?;

    // Title priority: displayName (from sessions.json) > first user message > dir basename
    let title = display_names
        .and_then(|m| m.get(&session_id))
        .filter(|s| !s.is_empty())
        .map(|t| truncate_summary(t, TITLE_MAX_CHARS))
        .or_else(|| first_user_message.map(|t| truncate_summary(&t, TITLE_MAX_CHARS)))
        .or_else(|| {
            cwd.as_deref()
                .and_then(path_basename)
                .map(|s| s.to_string())
        });

    let summary = summary.map(|text| truncate_summary(&text, 160));

    Some(SessionMeta {
        provider_id: PROVIDER_ID.to_string(),
        session_id: session_id.clone(),
        title,
        summary,
        project_dir: cwd,
        created_at,
        last_active_at,
        source_path: Some(path.to_string_lossy().to_string()),
        resume_command: None, // OpenClaw sessions are gateway-managed, no CLI resume
    })
}

fn prune_sessions_index(
    index_path: &Path,
    session_id: &str,
    source_path: &Path,
) -> Result<(), String> {
    if !index_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(index_path).map_err(|e| {
        format!(
            "Failed to read OpenClaw sessions index {}: {e}",
            index_path.display()
        )
    })?;
    let mut index: serde_json::Map<String, Value> =
        serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse OpenClaw sessions index {}: {e}",
                index_path.display()
            )
        })?;

    let source = source_path.to_string_lossy();
    index.retain(|_, entry| {
        let same_id = entry.get("sessionId").and_then(Value::as_str) == Some(session_id);
        let same_file = entry.get("sessionFile").and_then(Value::as_str) == Some(source.as_ref());
        !(same_id || same_file)
    });

    write_json_file(index_path, &index).map_err(|e| {
        format!(
            "Failed to update OpenClaw sessions index {}: {e}",
            index_path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn strip_gateway_metadata_follows_openclaw_display_rules() {
        // 末尾（多数渠道）与开头（飞书）的 message_id 行
        assert_eq!(
            strip_gateway_metadata("hello\n[message_id: abc123]"),
            "hello"
        );
        assert_eq!(
            strip_gateway_metadata("[message_id: om_x1]\nPlease review"),
            "Please review"
        );
        // 信封前缀只认带时间戳或已知渠道名的
        assert_eq!(
            strip_gateway_metadata(
                "[Telegram 2026-03-01 10:14] Hello there\n[message_id: abc-123]\nActual message"
            ),
            "Hello there\nActual message"
        );
        assert_eq!(strip_gateway_metadata("[draft] keep me"), "[draft] keep me");
        // 行内提到、代码块里的都保留
        assert_eq!(
            strip_gateway_metadata("I typed [message_id: 123] on purpose"),
            "I typed [message_id: 123] on purpose"
        );
        let fenced = "```\n[message_id: literal]\n```";
        assert_eq!(strip_gateway_metadata(fenced), fenced);
        // 长围栏里的短围栏不算结束；信息串不同的同长围栏也不算
        let nested = "````md\n```\n[message_id: literal]\n```\n[message_id: still inside]\n````\n[message_id: x]";
        assert_eq!(
            strip_gateway_metadata(nested),
            "````md\n```\n[message_id: literal]\n```\n[message_id: still inside]\n````"
        );
        let tilde = "~~~\n```\n[message_id: literal]\n~~~ not a close\n~~~";
        assert_eq!(strip_gateway_metadata(tilde), tilde);
        // CRLF 原样保留，结尾换行跟原文一致
        assert_eq!(
            strip_gateway_metadata("a\r\n[message_id: x]\r\nb\r\n"),
            "a\r\nb\r\n"
        );
        assert_eq!(
            strip_gateway_metadata("a\r\nb\r\n[message_id: x]"),
            "a\r\nb"
        );
    }

    #[test]
    fn load_messages_strips_gateway_metadata_from_user_text() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("s.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"session\",\"id\":\"s\",\"timestamp\":\"2026-03-06T10:00:00Z\"}\n",
                "{\"type\":\"message\",\"id\":\"m1\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"[message_id: om_1]\\nHow do I deploy?\"}]}}\n",
                "{\"type\":\"message\",\"id\":\"m2\",\"message\":{\"role\":\"user\",\"content\":\"Thanks\\n[message_id: om_2]\"}}\n"
            ),
        )
        .expect("write");

        let messages = load_messages(&path).expect("load");
        let texts: Vec<_> = messages.iter().map(|m| m.content.as_str()).collect();
        assert_eq!(texts, ["How do I deploy?", "Thanks"]);
    }

    #[test]
    fn parse_session_uses_first_user_message_as_title() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("session-abc.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"session\",\"id\":\"session-abc\",\"cwd\":\"/tmp/project\",\"timestamp\":\"2026-03-06T10:00:00Z\"}\n",
                "{\"type\":\"message\",\"message\":{\"role\":\"user\",\"content\":\"How do I deploy?\"},\"timestamp\":\"2026-03-06T10:01:00Z\"}\n",
                "{\"type\":\"message\",\"message\":{\"role\":\"assistant\",\"content\":\"Here is how...\"},\"timestamp\":\"2026-03-06T10:02:00Z\"}\n"
            ),
        )
        .expect("write");

        let meta = parse_session(&path, None).unwrap();
        assert_eq!(meta.title.as_deref(), Some("How do I deploy?"));
    }

    #[test]
    fn parse_session_display_name_overrides_user_message() {
        let temp = tempdir().expect("tempdir");
        let sessions_dir = temp.path();

        let path = sessions_dir.join("session-abc.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"session\",\"id\":\"session-abc\",\"cwd\":\"/tmp/project\",\"timestamp\":\"2026-03-06T10:00:00Z\"}\n",
                "{\"type\":\"message\",\"message\":{\"role\":\"user\",\"content\":\"fix something\"},\"timestamp\":\"2026-03-06T10:01:00Z\"}\n"
            ),
        )
        .expect("write session");

        std::fs::write(
            sessions_dir.join("sessions.json"),
            r#"{
                "agent:main:main": {
                    "sessionId": "session-abc",
                    "displayName": "重构登录模块"
                }
            }"#,
        )
        .expect("write index");

        let display_names = load_display_names(sessions_dir);
        let meta = parse_session(&path, Some(&display_names)).unwrap();
        assert_eq!(meta.title.as_deref(), Some("重构登录模块"));
    }

    #[test]
    fn parse_session_falls_back_to_dir_basename() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("session-def.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"session\",\"id\":\"session-def\",\"cwd\":\"/tmp/my-project\",\"timestamp\":\"2026-03-06T10:00:00Z\"}\n",
                "{\"type\":\"message\",\"message\":{\"role\":\"assistant\",\"content\":\"Hello\"},\"timestamp\":\"2026-03-06T10:01:00Z\"}\n"
            ),
        )
        .expect("write");

        let meta = parse_session(&path, None).unwrap();
        // No user message and no displayName → falls back to dir basename
        assert_eq!(meta.title.as_deref(), Some("my-project"));
    }

    #[test]
    fn parse_session_truncates_long_title() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("session-trunc.jsonl");
        let long_msg = "a".repeat(200);
        std::fs::write(
            &path,
            format!(
                "{{\"type\":\"session\",\"id\":\"session-trunc\",\"cwd\":\"/tmp/p\",\"timestamp\":\"2026-03-06T10:00:00Z\"}}\n\
                 {{\"type\":\"message\",\"message\":{{\"role\":\"user\",\"content\":\"{long_msg}\"}},\"timestamp\":\"2026-03-06T10:01:00Z\"}}\n",
            ),
        )
        .expect("write");

        let meta = parse_session(&path, None).unwrap();
        let title = meta.title.unwrap();
        assert!(title.len() <= TITLE_MAX_CHARS + 3); // +3 for "..."
        assert!(title.ends_with("..."));
    }

    #[test]
    fn delete_session_updates_index_and_removes_jsonl() {
        let temp = tempdir().expect("tempdir");
        let sessions_dir = temp.path().join("main").join("sessions");
        std::fs::create_dir_all(&sessions_dir).expect("create sessions dir");

        let session_path = sessions_dir.join("session-123.jsonl");
        std::fs::write(
            &session_path,
            concat!(
                "{\"type\":\"session\",\"id\":\"session-123\",\"cwd\":\"/tmp/project\",\"timestamp\":\"2026-03-06T10:00:00Z\"}\n",
                "{\"type\":\"message\",\"message\":{\"role\":\"user\",\"content\":\"hello\"},\"timestamp\":\"2026-03-06T10:01:00Z\"}\n"
            ),
        )
        .expect("write session");
        std::fs::write(
            sessions_dir.join("sessions.json"),
            serde_json::to_string(&serde_json::json!({
                "agent:main:main": {
                    "sessionId": "session-123",
                    "sessionFile": session_path.to_string_lossy(),
                },
                "agent:main:other": {
                    "sessionId": "session-456",
                    "sessionFile": sessions_dir.join("session-456.jsonl").to_string_lossy(),
                },
            }))
            .expect("serialize index"),
        )
        .expect("write index");

        delete_session(temp.path(), &session_path, "session-123").expect("delete session");

        assert!(!session_path.exists());
        let updated: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(sessions_dir.join("sessions.json")).expect("read index"),
        )
        .expect("parse index");
        assert!(updated.get("agent:main:main").is_none());
        assert!(updated.get("agent:main:other").is_some());
    }
}
