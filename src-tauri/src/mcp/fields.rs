//! 各客户端 MCP 条目认识的字段。
//!
//! 数据库里的服务器规范是各客户端共用的一份：从哪个客户端导入，就带着那个客户端自己的
//! 字段（Gemini 的 `trust`、Codex 的 `bearer_token_env_var`、Pi 的 `exposure` 等）。写给
//! 某个客户端时只写连接字段、这个客户端自己的字段和谁都不认识的字段；其他客户端的已知
//! 字段丢掉，免得到了别的客户端里不起作用（鉴权配置）或者改变行为（Gemini 的 `trust`）。

use serde_json::{Map, Value};

/// 各客户端共用的连接字段。
pub const TRANSPORT: &[&str] = &["type", "command", "args", "env", "cwd", "url", "headers"];

/// CC Switch 旧格式规范里的元数据，哪个客户端都不写。
const CC_SWITCH_META: &[&str] = &["source", "id", "name", "tags", "homepage", "docs"];

const CLAUDE: &[&str] = &["headersHelper", "oauth"];

const CODEX: &[&str] = &[
    "enabled",
    "required",
    "startup_timeout_sec",
    "startup_timeout_ms",
    "tool_timeout_sec",
    "tool_timeout_ms",
    "enabled_tools",
    "disabled_tools",
    "env_vars",
    "http_headers",
    "env_http_headers",
    "http_headers_helper",
    "bearer_token_env_var",
    "oauth_resource",
    "scopes",
    "startup_readiness",
];

/// Gemini 自己的字段（不含连接字段 `httpUrl`、`tcp`，见 `gemini_mcp`）。
pub const GEMINI: &[&str] = &[
    "timeout",
    "trust",
    "description",
    "includeTools",
    "excludeTools",
    "authProviderType",
    "targetAudience",
    "targetServiceAccount",
    "oauth",
];

/// Gemini 的连接字段里 [`TRANSPORT`] 之外的两个。
pub const GEMINI_TRANSPORT: &[&str] = &["httpUrl", "tcp"];

/// Pi 自己的字段（`mcp::pi` 重建条目时从保存的连接定义里带回来，`enabled` 除外）。
pub const PI: &[&str] = &[
    "enabled",
    "timeout",
    "description",
    "exposure",
    "toolExposure",
    "oauth",
    "auth",
];

const MCODE: &[&str] = &["enabled", "timeout", "description"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    Claude,
    Codex,
    Gemini,
    Pi,
    Mcode,
}

impl Client {
    const ALL: [Client; 5] = [
        Client::Claude,
        Client::Codex,
        Client::Gemini,
        Client::Pi,
        Client::Mcode,
    ];

    fn own(self) -> &'static [&'static str] {
        match self {
            Client::Claude => CLAUDE,
            Client::Codex => CODEX,
            Client::Gemini => GEMINI,
            Client::Pi => PI,
            Client::Mcode => MCODE,
        }
    }
}

/// `key` 是别的客户端（或 CC Switch 元数据）的已知字段，写给 `client` 时要丢掉。
pub fn is_foreign(client: Client, key: &str) -> bool {
    if TRANSPORT.contains(&key) || client.own().contains(&key) {
        return false;
    }
    if client == Client::Gemini && GEMINI_TRANSPORT.contains(&key) {
        return false;
    }
    CC_SWITCH_META.contains(&key)
        || GEMINI_TRANSPORT.contains(&key)
        || Client::ALL
            .iter()
            .any(|other| *other != client && other.own().contains(&key))
}

/// 去掉别的客户端的已知字段。
pub fn retain_for(client: Client, spec: &mut Map<String, Value>) {
    spec.retain(|key, _| !is_foreign(client, key));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_client_keeps_its_own_fields_transport_and_unknown_ones() {
        let all: Vec<&str> = TRANSPORT
            .iter()
            .chain(CC_SWITCH_META)
            .chain(GEMINI_TRANSPORT)
            .chain(Client::ALL.iter().flat_map(|client| client.own()))
            .copied()
            .chain(["somethingNew"])
            .collect();
        for client in Client::ALL {
            let kept: Vec<&str> = all
                .iter()
                .copied()
                .filter(|key| !is_foreign(client, key))
                .collect();
            for key in &kept {
                assert!(
                    TRANSPORT.contains(key)
                        || client.own().contains(key)
                        || *key == "somethingNew"
                        || (client == Client::Gemini && GEMINI_TRANSPORT.contains(key)),
                    "{client:?} keeps {key}"
                );
            }
            assert!(kept.contains(&"somethingNew"));
            assert!(TRANSPORT.iter().all(|key| kept.contains(key)));
            assert!(client.own().iter().all(|key| kept.contains(key)));
        }
    }
}
