//! 上游 CC Switch 的本地路由留在客户端配置里的痕迹。
//!
//! 上游接管某个应用时，把客户端配置改成指向本地代理（默认 `http://127.0.0.1:15721`），
//! Key 写成占位符 `PROXY_MANAGED`；退出时再写回直连。上游崩溃、被强杀或关机前来不及
//! 还原时，这些值留在客户端配置里，客户端会连不上。这里只做检测和「上游代理是否还在
//! 监听」的判断，修复由 `ProviderService::reapply_current` 重新写入当前供应商完成。

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::app_config::AppType;
use crate::provider::Provider;

/// 上游接管时写进客户端配置的 Key 占位符。
pub const PROXY_PLACEHOLDER: &str = "PROXY_MANAGED";
/// 上游给 Codex 官方账号写的代理路由表 id。
pub const OFFICIAL_PROXY_ROUTE_ID: &str = "cc-switch-official";
/// 上游代理的默认监听端口。
pub const DEFAULT_PROXY_PORT: u16 = 15721;
/// 客户端配置里没有地址时按这个地址判断上游代理是否还在。
pub const DEFAULT_PROXY_URL: &str = "http://127.0.0.1:15721";

const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);

/// 上游 Stack 模式写进 Codex 模型目录的行：`ccs-<key>/<model>`（格式不全的也算）。
/// 这些模型属于别的供应商、要经上游路由才能用，不能当成当前供应商的模型。
pub fn is_codex_stack_model(slug: &str) -> bool {
    slug.strip_prefix("ccs-")
        .is_some_and(|rest| rest.contains('/'))
}

const CLAUDE_KEY_FIELDS: &[&str] = &[
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "OPENROUTER_API_KEY",
    "OPENAI_API_KEY",
];

/// 客户端配置处于上游路由状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyRouting {
    /// 配置里指向的代理地址（读不到时为 `None`，按 [`DEFAULT_PROXY_URL`] 判断）。
    pub base_url: Option<String>,
}

/// 只检测这四个应用：上游只接管它们。
pub fn is_supported(app: &AppType) -> bool {
    matches!(
        app,
        AppType::Claude | AppType::Codex | AppType::Gemini | AppType::GrokBuild
    )
}

/// 一份配置（客户端文件或供应商行，形状同 `read_live_settings`）里有没有占位符或上游的
/// Codex 官方代理路由。
pub fn config_has_proxy_placeholder(app: &AppType, config: &Value) -> bool {
    match app {
        AppType::Claude => claude_has_placeholder(config),
        AppType::Codex => codex_has_placeholder(config) || codex_routes_to_proxy(config),
        AppType::Gemini => str_at(config, &["env", "GEMINI_API_KEY"]) == Some(PROXY_PLACEHOLDER),
        AppType::GrokBuild => {
            config_text(config)
                .and_then(crate::grok_config::extract_model_config)
                .and_then(|model| model.api_key)
                .as_deref()
                == Some(PROXY_PLACEHOLDER)
        }
        _ => false,
    }
}

/// 按 [`config_has_proxy_placeholder`] 判断，并取出配置里的代理地址。
pub fn detect(app: &AppType, live: &Value) -> Option<LegacyRouting> {
    if !config_has_proxy_placeholder(app, live) {
        return None;
    }
    let base_url = match app {
        AppType::Claude => str_at(live, &["env", "ANTHROPIC_BASE_URL"]).map(str::to_string),
        AppType::Codex => config_text(live).and_then(codex_proxy_base_url),
        AppType::Gemini => str_at(live, &["env", "GOOGLE_GEMINI_BASE_URL"]).map(str::to_string),
        AppType::GrokBuild => config_text(live).and_then(crate::grok_config::extract_base_url),
        _ => None,
    };
    Some(LegacyRouting {
        base_url: base_url
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty()),
    })
}

/// 读客户端配置并检测。配置不存在或读不了时按没有路由状态处理。
pub fn live_routing_state(app: &AppType) -> Option<LegacyRouting> {
    if !is_supported(app) {
        return None;
    }
    let live = crate::services::provider::ProviderService::read_live_settings(app.clone()).ok()?;
    detect(app, &live)
}

/// 上游代理是否还在监听：只对回环地址尝试 TCP 连接（超时 300 ms）。不是回环地址、
/// 地址解析不了或连不上，都按不在监听处理。
pub fn upstream_is_listening(routing: &LegacyRouting) -> bool {
    let url = routing.base_url.as_deref().unwrap_or(DEFAULT_PROXY_URL);
    loopback_addrs(url)
        .iter()
        .any(|addr| TcpStream::connect_timeout(addr, CONNECT_TIMEOUT).is_ok())
}

/// Claude 卡的接口格式要不要上游代理转换。格式的优先级照上游代理（d35726e2
/// `proxy::providers::claude::get_claude_api_format`）：`meta.apiFormat` > 旧版写在
/// settings 里的 `api_format` > 旧版的 `openrouter_compat_mode`（开着就是 openai_chat），
/// 认不出的值按 anthropic。
fn claude_needs_transform(meta_format: Option<&str>, settings: &Value) -> bool {
    const TRANSFORMED: [&str; 3] = ["openai_chat", "openai_responses", "gemini_native"];
    if let Some(format) = meta_format {
        return TRANSFORMED.contains(&format);
    }
    if let Some(format) = settings.get("api_format").and_then(Value::as_str) {
        return TRANSFORMED.contains(&format);
    }
    match settings.get("openrouter_compat_mode") {
        Some(Value::Bool(enabled)) => *enabled,
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0) != 0,
        Some(Value::String(value)) => {
            matches!(value.trim().to_lowercase().as_str(), "true" | "1")
        }
        _ => false,
    }
}

/// 上游这几种托管登录的凭据由本地代理按请求注入，没有路由就用不了。
const MANAGED_OAUTH_PROVIDER_TYPES: &[&str] = &["github_copilot", "codex_oauth", "xai_oauth"];

/// 这个供应商只能经过上游 CC Switch 已移除的本地路由使用：直接写进客户端配置也用不了
/// （托管登录、需要格式转换的接口、完整 URL）。和前端 `requiresRemovedRouting`
/// （`src/utils/providerCapabilities.ts`）是同一条规则，改一边要改另一边。
pub fn requires_removed_routing(app: &AppType, provider: &Provider) -> bool {
    if is_official_account(app, provider) {
        return false;
    }
    let meta = provider.meta.as_ref();
    let managed_oauth = meta
        .and_then(|meta| meta.provider_type.as_deref())
        .is_some_and(|kind| MANAGED_OAUTH_PROVIDER_TYPES.contains(&kind));
    let extra = |key: &str| meta.and_then(|meta| meta.extra.get(key));
    let full_url = extra("isFullUrl").and_then(Value::as_bool) == Some(true);
    let api_format = extra("apiFormat")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|format| !format.is_empty());
    let config = provider
        .settings_config
        .get("config")
        .and_then(Value::as_str);
    match app {
        AppType::Claude => {
            managed_oauth
                || full_url
                || claude_needs_transform(api_format, &provider.settings_config)
        }
        AppType::Codex => {
            managed_oauth
                || full_url
                || matches!(api_format, Some("openai_chat" | "anthropic"))
                || config
                    .and_then(codex_wire_api)
                    .is_some_and(|wire_api| is_chat_or_anthropic_wire_api(&wire_api))
        }
        // Grok Build 自己能接 Chat Completions 和 Anthropic Messages（`api_backend`）：上游
        // 按格式转换的卡，表里的 `api_backend` 改成对应的接口后就能直连。
        AppType::GrokBuild => {
            managed_oauth
                || full_url
                || api_format.is_some_and(|format| {
                    let wanted = match format {
                        "openai_chat" => "chat_completions",
                        "anthropic" => "messages",
                        _ => return false,
                    };
                    config
                        .and_then(crate::grok_config::extract_model_config)
                        .map(|model| model.api_backend)
                        .as_deref()
                        != Some(wanted)
                })
        }
        _ => false,
    }
}

/// 官方账号卡（同前端 `isOfficialAccount`）：Codex 早期绑定托管账号的官方卡没有
/// category，按身份认。
fn is_official_account(app: &AppType, provider: &Provider) -> bool {
    provider.category.as_deref() == Some("official")
        || (*app == AppType::Codex && crate::codex_provider::is_codex_official_provider(provider))
}

/// Codex 配置里生效 provider 的 `wire_api`，没有时取顶层的。
fn codex_wire_api(text: &str) -> Option<String> {
    let doc = text.parse::<toml::Table>().ok()?;
    let active = doc
        .get("model_provider")
        .and_then(toml::Value::as_str)
        .and_then(|id| doc.get("model_providers")?.get(id))
        .and_then(|table| table.get("wire_api"))
        .and_then(toml::Value::as_str);
    active
        .or_else(|| doc.get("wire_api").and_then(toml::Value::as_str))
        .map(str::to_string)
}

fn is_chat_or_anthropic_wire_api(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "chat"
            | "chat_completions"
            | "chat-completions"
            | "openai_chat"
            | "openai-chat"
            | "openai_chat_completions"
            | "anthropic"
            | "anthropic_messages"
            | "anthropic-messages"
            | "messages"
            | "claude"
    )
}

fn loopback_addrs(url: &str) -> Vec<SocketAddr> {
    let Ok(parsed) = url::Url::parse(url) else {
        return Vec::new();
    };
    let Some(port) = parsed.port_or_known_default() else {
        return Vec::new();
    };
    let host = match parsed.host() {
        Some(url::Host::Ipv4(ip)) if ip.is_loopback() => return vec![(ip, port).into()],
        Some(url::Host::Ipv6(ip)) if ip.is_loopback() => return vec![(ip, port).into()],
        Some(url::Host::Domain(domain)) if domain.eq_ignore_ascii_case("localhost") => domain,
        _ => return Vec::new(),
    };
    (host, port)
        .to_socket_addrs()
        .map(|addrs| addrs.filter(|addr| addr.ip().is_loopback()).collect())
        .unwrap_or_default()
}

fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |value, key| value.get(key))
        .and_then(Value::as_str)
}

fn config_text(config: &Value) -> Option<&str> {
    config.get("config").and_then(Value::as_str)
}

fn claude_has_placeholder(config: &Value) -> bool {
    CLAUDE_KEY_FIELDS
        .iter()
        .any(|key| str_at(config, &["env", key]) == Some(PROXY_PLACEHOLDER))
}

fn codex_has_placeholder(config: &Value) -> bool {
    str_at(config, &["auth", "OPENAI_API_KEY"]) == Some(PROXY_PLACEHOLDER)
        || config_text(config)
            .and_then(crate::codex_config::extract_codex_experimental_bearer_token)
            .as_deref()
            == Some(PROXY_PLACEHOLDER)
}

/// 上游给官方账号写的代理路由没有占位符，只能按选路和地址认：选了 `cc-switch-official`
/// 表；或者不选别的 provider、顶层 `openai_base_url` 指向代理；或者选 `custom`、表里
/// `requires_openai_auth = true` 且地址指向代理。代理地址按回环主机加 15721 端口认，
/// 免得把用户自己的本地模型服务（如 `127.0.0.1:1234/v1`）当成上游代理。
fn codex_routes_to_proxy(config: &Value) -> bool {
    let Some(doc) = config_text(config).and_then(|text| text.parse::<toml::Table>().ok()) else {
        return false;
    };
    let table = |id: &str| {
        doc.get("model_providers")
            .and_then(|providers| providers.get(id))
            .and_then(toml::Value::as_table)
    };
    let url_of = |value: Option<&toml::Value>| {
        value
            .and_then(toml::Value::as_str)
            .is_some_and(is_proxy_url)
    };
    match doc.get("model_provider").and_then(toml::Value::as_str) {
        Some(OFFICIAL_PROXY_ROUTE_ID) => true,
        None | Some("openai") => url_of(doc.get("openai_base_url")),
        Some(crate::live::project::codex::ROUTE_ID) => table(crate::live::project::codex::ROUTE_ID)
            .is_some_and(|table| {
                table
                    .get("requires_openai_auth")
                    .and_then(toml::Value::as_bool)
                    == Some(true)
                    && url_of(table.get("base_url"))
            }),
        Some(_) => false,
    }
}

/// Codex 配置里生效的代理地址：选中的 provider 表的 `base_url`，没有时取顶层
/// `openai_base_url`。
fn codex_proxy_base_url(text: &str) -> Option<String> {
    crate::codex_config::extract_codex_base_url(text).or_else(|| {
        text.parse::<toml::Table>()
            .ok()?
            .get("openai_base_url")?
            .as_str()
            .map(str::to_string)
    })
}

fn is_proxy_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url.trim()) else {
        return false;
    };
    let loopback = match parsed.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        Some(url::Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        None => false,
    };
    loopback && parsed.port() == Some(DEFAULT_PROXY_PORT)
}

#[cfg(test)]
mod tests {
    #[test]
    fn codex_stack_models_follow_the_upstream_id_format() {
        use super::is_codex_stack_model;
        assert!(is_codex_stack_model(
            "ccs-deepseek/deepseek/deepseek-v4-pro"
        ));
        assert!(is_codex_stack_model("ccs-/m"));
        assert!(!is_codex_stack_model("ccs-without-separator"));
        assert!(!is_codex_stack_model("gpt-5.5"));
        assert!(!is_codex_stack_model("openai/gpt-5.5"));
    }

    use super::*;
    use serde_json::json;

    #[test]
    fn claude_placeholder_in_any_key_field_is_routing() {
        for key in CLAUDE_KEY_FIELDS {
            let live = json!({ "env": {
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:15721",
                *key: PROXY_PLACEHOLDER,
            }});
            assert_eq!(
                detect(&AppType::Claude, &live),
                Some(LegacyRouting {
                    base_url: Some("http://127.0.0.1:15721".into())
                }),
                "{key}"
            );
        }
        let direct = json!({ "env": {
            "ANTHROPIC_BASE_URL": "https://api.example.com",
            "ANTHROPIC_AUTH_TOKEN": "sk-real",
        }});
        assert_eq!(detect(&AppType::Claude, &direct), None);
    }

    #[test]
    fn codex_placeholder_in_auth_or_active_table_is_routing() {
        let auth = json!({
            "auth": { "OPENAI_API_KEY": PROXY_PLACEHOLDER },
            "config": "model = \"gpt-5\"\n",
        });
        assert!(detect(&AppType::Codex, &auth).is_some());

        let table = json!({
            "auth": {},
            "config": "model_provider = \"custom\"\n\n[model_providers.custom]\nbase_url = \"http://127.0.0.1:15721/v1\"\nexperimental_bearer_token = \"PROXY_MANAGED\"\n",
        });
        assert_eq!(
            detect(&AppType::Codex, &table),
            Some(LegacyRouting {
                base_url: Some("http://127.0.0.1:15721/v1".into())
            })
        );

        let direct = json!({
            "auth": {},
            "config": "model_provider = \"custom\"\n\n[model_providers.custom]\nbase_url = \"https://api.example.com/v1\"\nexperimental_bearer_token = \"sk-real\"\n",
        });
        assert_eq!(detect(&AppType::Codex, &direct), None);
    }

    #[test]
    fn codex_official_proxy_routes_are_routing() {
        let selected = "model_provider = \"cc-switch-official\"\n\n[model_providers.cc-switch-official]\nbase_url = \"http://127.0.0.1:15721/v1\"\nrequires_openai_auth = true\n";
        let top_level = "openai_base_url = \"http://127.0.0.1:15721/v1\"\n";
        let mirror = "model_provider = \"custom\"\n\n[model_providers.custom]\nname = \"OpenAI\"\nbase_url = \"http://localhost:15721/v1\"\nrequires_openai_auth = true\n";
        for text in [selected, top_level, mirror] {
            let live = json!({ "auth": {}, "config": text });
            assert!(detect(&AppType::Codex, &live).is_some(), "{text}");
        }
        assert_eq!(
            detect(&AppType::Codex, &json!({ "auth": {}, "config": top_level }))
                .and_then(|routing| routing.base_url),
            Some("http://127.0.0.1:15721/v1".into())
        );
    }

    #[test]
    fn codex_local_model_servers_and_dormant_tables_are_not_routing() {
        let lm_studio = "openai_base_url = \"http://127.0.0.1:1234/v1\"\n";
        let mirror_elsewhere = "model_provider = \"custom\"\n\n[model_providers.custom]\nbase_url = \"http://127.0.0.1:8080/v1\"\nrequires_openai_auth = true\n";
        // 选官方时留下的休眠表（新旧两种形态）没有被选中，不算路由状态。
        let legacy_dormant = "model = \"gpt-5.5\"\n\n[model_providers.custom]\nname = \"custom\"\nbase_url = \"http://127.0.0.1:15721/v1\"\nwire_api = \"responses\"\nexperimental_bearer_token = \"PROXY_MANAGED\"\n";
        let dormant = "model = \"gpt-5.5\"\n\n[model_providers.custom]\nname = \"custom\"\nbase_url = \"https://dormant.invalid/v1\"\nwire_api = \"responses\"\nexperimental_bearer_token = \"CCS_LITE_DORMANT\"\n";
        for text in [lm_studio, mirror_elsewhere, legacy_dormant, dormant] {
            let live = json!({ "auth": {}, "config": text });
            assert_eq!(detect(&AppType::Codex, &live), None, "{text}");
        }
    }

    #[test]
    fn gemini_placeholder_key_is_routing() {
        let live = json!({ "env": {
            "GEMINI_API_KEY": PROXY_PLACEHOLDER,
            "GOOGLE_GEMINI_BASE_URL": "http://127.0.0.1:15721/gemini",
        }, "config": {} });
        assert_eq!(
            detect(&AppType::Gemini, &live),
            Some(LegacyRouting {
                base_url: Some("http://127.0.0.1:15721/gemini".into())
            })
        );
        let direct = json!({ "env": { "GEMINI_API_KEY": "real" }, "config": {} });
        assert_eq!(detect(&AppType::Gemini, &direct), None);
    }

    #[test]
    fn grok_placeholder_in_selected_model_is_routing() {
        let routed = "[models]\ndefault = \"grok-4.5\"\n\n[model.\"grok-4.5\"]\nmodel = \"a\"\nbase_url = \"http://127.0.0.1:15721/grokbuild/v1\"\napi_key = \"PROXY_MANAGED\"\n";
        assert_eq!(
            detect(&AppType::GrokBuild, &json!({ "config": routed })),
            Some(LegacyRouting {
                base_url: Some("http://127.0.0.1:15721/grokbuild/v1".into())
            })
        );
        let direct = routed.replace("PROXY_MANAGED", "xai-real");
        assert_eq!(
            detect(&AppType::GrokBuild, &json!({ "config": direct })),
            None
        );
    }

    fn card(app_config: Value, meta: Value) -> Provider {
        let mut provider = Provider::with_id("p".to_string(), "P".to_string(), app_config, None);
        provider.meta = Some(serde_json::from_value(meta).unwrap());
        provider
    }

    #[test]
    fn cards_that_only_worked_through_routing_are_recognized() {
        let claude = json!({ "env": { "ANTHROPIC_BASE_URL": "https://x.example" } });
        let codex_responses = json!({ "auth": {}, "config": "model_provider = \"x\"\n[model_providers.x]\nbase_url = \"https://x.example/v1\"\nwire_api = \"responses\"\n" });
        let codex_chat = json!({ "auth": {}, "config": "model_provider = \"x\"\n[model_providers.x]\nbase_url = \"https://x.example/v1\"\nwire_api = \"chat\"\n" });
        let grok = |backend: &str| json!({ "config": format!("[models]\ndefault = \"g\"\n\n[model.g]\nmodel = \"m\"\nbase_url = \"https://x.example/v1\"\napi_key = \"k\"\napi_backend = \"{backend}\"\n") });

        let cases = [
            (AppType::Claude, card(claude.clone(), json!({})), false),
            (
                AppType::Claude,
                card(claude.clone(), json!({ "apiFormat": "anthropic" })),
                false,
            ),
            (
                AppType::Claude,
                card(claude.clone(), json!({ "apiFormat": "openai_chat" })),
                true,
            ),
            // 旧版写在 settings 里的格式；meta 里的优先
            (
                AppType::Claude,
                card(
                    json!({ "env": {}, "api_format": "openai_responses" }),
                    json!({}),
                ),
                true,
            ),
            (
                AppType::Claude,
                card(
                    json!({ "env": {}, "api_format": "openai_chat" }),
                    json!({ "apiFormat": "anthropic" }),
                ),
                false,
            ),
            (
                AppType::Claude,
                card(
                    json!({ "env": {}, "openrouter_compat_mode": "1" }),
                    json!({}),
                ),
                true,
            ),
            (
                AppType::Claude,
                card(
                    json!({ "env": {}, "openrouter_compat_mode": false }),
                    json!({}),
                ),
                false,
            ),
            (
                AppType::Claude,
                card(claude.clone(), json!({ "isFullUrl": true })),
                true,
            ),
            (
                AppType::Claude,
                card(claude.clone(), json!({ "providerType": "github_copilot" })),
                true,
            ),
            (
                AppType::Claude,
                card(claude.clone(), json!({ "providerType": "codex_oauth" })),
                true,
            ),
            (
                AppType::Codex,
                card(codex_responses.clone(), json!({})),
                false,
            ),
            (AppType::Codex, card(codex_chat, json!({})), true),
            (
                AppType::Codex,
                card(
                    codex_responses.clone(),
                    json!({ "apiFormat": "openai_chat" }),
                ),
                true,
            ),
            (
                AppType::Codex,
                card(
                    codex_responses.clone(),
                    json!({ "apiFormat": "openai_responses" }),
                ),
                false,
            ),
            (
                AppType::GrokBuild,
                card(grok("responses"), json!({ "apiFormat": "openai_chat" })),
                true,
            ),
            (
                AppType::GrokBuild,
                card(
                    grok("chat_completions"),
                    json!({ "apiFormat": "openai_chat" }),
                ),
                false,
            ),
            (
                AppType::GrokBuild,
                card(grok("messages"), json!({ "apiFormat": "anthropic" })),
                false,
            ),
            (
                AppType::GrokBuild,
                card(grok("responses"), json!({ "providerType": "xai_oauth" })),
                true,
            ),
            (
                AppType::Gemini,
                card(json!({ "env": {} }), json!({ "isFullUrl": true })),
                false,
            ),
        ];
        for (app, provider, expected) in cases {
            assert_eq!(
                requires_removed_routing(&app, &provider),
                expected,
                "{app:?} {:?}",
                provider.meta
            );
        }

        // 官方账号卡（含绑定了 ChatGPT 账号的 Codex 官方卡）不算。
        let mut official = card(codex_responses, json!({ "providerType": "codex_oauth" }));
        official.category = Some("official".to_string());
        assert!(!requires_removed_routing(&AppType::Codex, &official));
    }

    #[test]
    fn listening_check_only_connects_to_loopback() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let live = |url: String| LegacyRouting {
            base_url: Some(url),
        };
        assert!(upstream_is_listening(&live(format!(
            "http://127.0.0.1:{port}/v1"
        ))));
        assert!(upstream_is_listening(&live(format!(
            "http://localhost:{port}"
        ))));
        drop(listener);
        assert!(!upstream_is_listening(&live(format!(
            "http://127.0.0.1:{port}/v1"
        ))));
        assert!(!upstream_is_listening(&live(
            "https://api.example.com/v1".into()
        )));
        assert!(!upstream_is_listening(&live("not a url".into())));
    }
}
