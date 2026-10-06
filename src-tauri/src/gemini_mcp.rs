use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::atomic_write;
use crate::error::AppError;
use crate::gemini_config::get_gemini_settings_path;

/// 获取 Gemini MCP 配置文件路径（~/.gemini/settings.json）
fn user_config_path() -> PathBuf {
    get_gemini_settings_path()
}

fn read_json_value(path: &Path) -> Result<Value, AppError> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;
    let value: Value = serde_json::from_str(&content).map_err(|e| AppError::json(path, e))?;
    Ok(value)
}

fn write_json_value(path: &Path, value: &Value) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let json =
        serde_json::to_string_pretty(value).map_err(|e| AppError::JsonSerialize { source: e })?;
    atomic_write(path, json.as_bytes())
}

/// Gemini 条目里描述连接方式的字段。写入时先从旧条目里清掉这些字段再放入新规范，
/// 免得从 stdio 改成 http 后残留 `command`；`timeout`、`trust`、`includeTools`
/// 这类 Gemini 自己的设置原样保留。
const TRANSPORT_FIELDS: [&str; 9] = [
    "command", "args", "env", "cwd", "url", "httpUrl", "headers", "tcp", "type",
];

/// 统一规范里只供 CC Switch 或其它客户端使用、Gemini 不认识的字段。
const SKIPPED_FIELDS: [&str; 12] = [
    "type",
    "enabled",
    "source",
    "id",
    "name",
    "tags",
    "homepage",
    "docs",
    "startup_timeout_sec",
    "startup_timeout_ms",
    "tool_timeout_sec",
    "tool_timeout_ms",
];

/// Gemini 条目 → 统一规范。
///
/// Gemini CLI 的传输判定：`httpUrl`（已弃用）是 streamable HTTP；`url` 按 `type`
/// 选 `http` / `sse`，没写 `type` 时先试 HTTP、失败再回退 SSE；只有 `command` 是 stdio。
fn unified_spec(native: &Value) -> Value {
    let mut spec = native.clone();
    let Some(obj) = spec.as_object_mut() else {
        return spec;
    };
    if let Some(http_url) = obj.remove("httpUrl") {
        obj.insert("url".into(), http_url);
        obj.insert("type".into(), Value::String("http".into()));
    }
    if obj.get("type").is_none() {
        if obj.contains_key("command") {
            obj.insert("type".into(), Value::String("stdio".into()));
        } else if obj.contains_key("url") {
            obj.insert("type".into(), Value::String("http".into()));
        }
    }
    spec
}

/// 读取 Gemini settings.json 中的 mcpServers 映射，转换成统一规范（见 [`unified_spec`]）
pub fn read_mcp_servers_map() -> Result<std::collections::HashMap<String, Value>, AppError> {
    let path = user_config_path();
    if !path.exists() {
        return Ok(std::collections::HashMap::new());
    }

    let root = read_json_value(&path)?;
    Ok(root
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), unified_spec(v)))
                .collect()
        })
        .unwrap_or_default())
}

/// 统一规范 → Gemini 条目，合并到同名的现有条目上。
///
/// - http 写成 `url` + `type: "http"`（Gemini 对 `httpUrl` 的弃用提示要求的写法）；
///   现有条目用不带 `type` 的同一个 `url`（HTTP 失败回退 SSE）时保持原写法。
/// - sse 写成 `url` + `type: "sse"`；stdio 不写 `type`。
/// - Claude/Codex 的 `startup_timeout_*` / `tool_timeout_*` 换算成 Gemini 的
///   `timeout`（毫秒，取较大者）；规范自带 `timeout` 时以它为准；都没有就不写，
///   沿用现有条目的值或 Gemini 默认的 10 分钟。
fn native_entry(id: &str, spec: &Value, existing: Option<&Value>) -> Result<Value, AppError> {
    let spec = spec
        .as_object()
        .ok_or_else(|| AppError::McpValidation(format!("MCP 服务器 '{id}' 不是对象")))?;
    let existing = existing.and_then(Value::as_object);
    let mut out = existing.cloned().unwrap_or_default();
    for field in TRANSPORT_FIELDS {
        out.remove(field);
    }

    for (key, value) in spec {
        if key == "type" {
            let auto_detect = existing.is_some_and(|e| {
                e.get("type").is_none()
                    && e.get("httpUrl").is_none()
                    && e.get("url") == spec.get("url")
            });
            match value.as_str() {
                Some("http") if !auto_detect => {
                    out.insert("type".into(), value.clone());
                }
                Some("sse") => {
                    out.insert("type".into(), value.clone());
                }
                _ => {}
            }
        } else if !SKIPPED_FIELDS.contains(&key.as_str()) {
            out.insert(key.clone(), value.clone());
        }
    }

    if !spec.contains_key("timeout") {
        let millis = |key: &str, multiplier: f64| {
            spec.get(key)
                .and_then(Value::as_f64)
                .map(|n| (n * multiplier) as u64)
        };
        let startup =
            millis("startup_timeout_sec", 1000.0).or_else(|| millis("startup_timeout_ms", 1.0));
        let tool = millis("tool_timeout_sec", 1000.0).or_else(|| millis("tool_timeout_ms", 1.0));
        if let Some(timeout) = startup.max(tool) {
            out.insert("timeout".into(), Value::Number(timeout.into()));
        }
    }

    Ok(Value::Object(out))
}

/// 写入或删除 Gemini settings.json 里的单个 MCP 条目，其它条目和其它设置保持不变。
/// 结果与现有内容相同时不写文件。
pub fn sync_server(id: &str, spec: Option<&Value>) -> Result<(), AppError> {
    let path = user_config_path();
    let mut root = read_json_value(&path)?;
    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| AppError::Config("~/.gemini/settings.json 根必须是对象".into()))?;

    let current = root_obj.get("mcpServers").and_then(|v| v.get(id));
    let next = spec
        .map(|spec| native_entry(id, spec, current))
        .transpose()?;
    if current == next.as_ref() {
        return Ok(());
    }

    let servers = root_obj
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            AppError::Config("~/.gemini/settings.json 的 mcpServers 必须是对象".into())
        })?;
    match next {
        Some(entry) => {
            servers.insert(id.to_string(), entry);
        }
        None => {
            servers.remove(id);
        }
    }

    write_json_value(&path, &root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unified_spec_follows_gemini_transport_rules() {
        let cases = [
            (
                json!({"httpUrl": "https://a/mcp"}),
                json!({"url": "https://a/mcp", "type": "http"}),
            ),
            (
                json!({"url": "https://a/mcp"}),
                json!({"url": "https://a/mcp", "type": "http"}),
            ),
            (
                json!({"url": "https://a/sse", "type": "sse"}),
                json!({"url": "https://a/sse", "type": "sse"}),
            ),
            (
                json!({"command": "node"}),
                json!({"command": "node", "type": "stdio"}),
            ),
        ];
        for (native, unified) in cases {
            assert_eq!(unified_spec(&native), unified);
        }
    }

    #[test]
    fn native_entry_writes_url_with_type_and_drops_foreign_fields() {
        let spec = json!({
            "type": "http",
            "url": "https://a/mcp",
            "headers": {"Authorization": "Bearer t"},
            "enabled": true,
            "description": "docs search",
        });
        assert_eq!(
            native_entry("a", &spec, None).unwrap(),
            json!({
                "type": "http",
                "url": "https://a/mcp",
                "headers": {"Authorization": "Bearer t"},
                "description": "docs search",
            })
        );
        let sse = json!({"type": "sse", "url": "https://a/sse"});
        assert_eq!(native_entry("a", &sse, None).unwrap(), sse);
        assert_eq!(
            native_entry("a", &json!({"type": "stdio", "command": "node"}), None).unwrap(),
            json!({"command": "node"})
        );
    }

    #[test]
    fn native_entry_keeps_gemini_settings_and_replaces_transport() {
        let existing = json!({"command": "old", "args": ["x"], "timeout": 5000, "trust": true});
        let spec = json!({"type": "http", "url": "https://a/mcp"});
        assert_eq!(
            native_entry("a", &spec, Some(&existing)).unwrap(),
            json!({"timeout": 5000, "trust": true, "type": "http", "url": "https://a/mcp"})
        );

        // 现有条目靠自动探测（HTTP 失败回退 SSE）时不补 `type`。
        let auto = json!({"url": "https://a/mcp", "timeout": 5000});
        assert_eq!(native_entry("a", &spec, Some(&auto)).unwrap(), auto);
    }

    #[test]
    fn native_entry_converts_timeouts_only_when_configured() {
        let spec = |extra: Value| {
            let mut spec = json!({"command": "node"});
            spec.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            spec
        };
        let timeout = |extra: Value, existing: Option<Value>| {
            native_entry("a", &spec(extra), existing.as_ref()).unwrap()["timeout"].clone()
        };
        assert_eq!(timeout(json!({}), None), Value::Null);
        assert_eq!(
            timeout(json!({}), Some(json!({"timeout": 5000}))),
            json!(5000)
        );
        assert_eq!(timeout(json!({"tool_timeout_sec": 30}), None), json!(30000));
        assert_eq!(
            timeout(
                json!({"startup_timeout_ms": 90000, "tool_timeout_sec": 30}),
                None
            ),
            json!(90000)
        );
        assert_eq!(
            timeout(json!({"timeout": 1000, "tool_timeout_sec": 30}), None),
            json!(1000)
        );
        let entry = native_entry("a", &spec(json!({"tool_timeout_sec": 30})), None).unwrap();
        assert!(entry.get("tool_timeout_sec").is_none());
    }
}
