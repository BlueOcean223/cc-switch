//! Codex 供应商的判定：是不是官方卡、该用哪种模型目录工具配置。
//! 写 Codex 客户端文件（`config.toml`、模型目录）时用。

use crate::provider::Provider;
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

fn has_explicit_codex_third_party_upstream(provider: &Provider) -> bool {
    let non_empty_setting = |key: &str| {
        provider
            .settings_config
            .get(key)
            .and_then(JsonValue::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    };
    let config = provider
        .settings_config
        .get("config")
        .and_then(JsonValue::as_str)
        .map(|text| {
            crate::codex_config::strip_codex_unified_session_bucket(text)
                .unwrap_or_else(|_| text.to_string())
        });
    let config = config.as_deref();

    ["baseUrl", "baseURL", "base_url"]
        .into_iter()
        .any(non_empty_setting)
        || config
            .and_then(crate::codex_config::extract_codex_experimental_bearer_token)
            .is_some()
        || config
            .and_then(crate::codex_config::extract_codex_base_url)
            .is_some()
        || config
            .and_then(|text| text.parse::<TomlValue>().ok())
            .and_then(|doc| {
                doc.get("model_provider")
                    .and_then(TomlValue::as_str)
                    .map(str::trim)
                    .filter(|provider_id| !provider_id.is_empty())
                    .map(str::to_string)
            })
            // Exact match, mirroring upstream: the built-in lookup is
            // case-sensitive, so `OpenAI` routes to a custom table — a
            // third-party upstream, not the official provider.
            .is_some_and(|provider_id| provider_id != "openai")
}

/// Codex Official ChatGPT cards receive authentication from the calling Codex
/// client (`requires_openai_auth = true`). Unbound cards with a stored API key
/// stay on the direct OpenAI API path instead of being sent to the ChatGPT
/// backend. The fixed legacy card keeps its existing behavior.
pub fn is_codex_official_provider(provider: &Provider) -> bool {
    let is_fixed_official_id = provider.id == crate::database::CODEX_OFFICIAL_PROVIDER_ID;
    if is_fixed_official_id && provider.category.as_deref() == Some("official") {
        return true;
    }

    let has_auth_object = provider
        .settings_config
        .get("auth")
        .is_some_and(JsonValue::is_object);
    let has_valid_config_shape = provider
        .settings_config
        .get("config")
        .is_none_or(|config| config.is_null() || config.is_string());
    if !has_auth_object || !has_valid_config_shape {
        return false;
    }

    if has_explicit_codex_third_party_upstream(provider) {
        return false;
    }

    let has_managed_account = provider
        .meta
        .as_ref()
        .and_then(|meta| meta.managed_account_id_for("codex_oauth"))
        .is_some_and(|account_id| !account_id.trim().is_empty());
    if has_managed_account {
        return true;
    }

    let has_stored_api_key = provider
        .settings_config
        .get("auth")
        .and_then(|auth| auth.get("OPENAI_API_KEY"))
        .and_then(JsonValue::as_str)
        .is_some_and(|key| !key.trim().is_empty());
    if has_stored_api_key {
        return false;
    }

    is_fixed_official_id || provider.category.as_deref() == Some("official")
}

/// Vendors whose OFFICIAL Codex integration is a native `/responses` gateway that
/// rejects Codex's freeform custom tools (`apply_patch` with `type: "custom"`,
/// #6944). This is intentionally separate from `CODEX_WEB_SEARCH_REJECT_HOSTS`:
/// web-search compatibility alone must not change a stored Chat provider's
/// protocol or catalog. Matched on
/// host labels via `codex_url_host_matches_any`, never by substring.
const CODEX_NATIVE_RESPONSES_HOSTS: &[&str] = &[
    "bigmodel.cn",
    "z.ai",
    "xiaomimimo.com",
    "minimaxi.com",
    "minimax.cn",
    "minimax.io",
    "longcat.chat",
];

/// Path markers of a listed vendor's OpenAI *Chat Completions* endpoint, which is
/// NOT its Responses endpoint. Zhipu documents three separate base URLs per site
/// (Anthropic `/api/anthropic`, Chat `/api/coding/paas/v4` + pay-as-you-go
/// `/api/paas/v4`, Responses `/api/v1`) and warns that the wrong one cannot use
/// Coding Plan quota. A stored provider still pointing at a Chat path is a
/// pre-2026-09 Chat-route record: it keeps its `ProxyChat` catalog (direct
/// connect fails loudly with the #6944 400 until the preset is re-imported)
/// instead of being silently steered onto the wrong endpoint with a native
/// catalog.
const CODEX_NATIVE_RESPONSES_CHAT_PATH_MARKERS: &[&str] = &["/paas/v4"];

/// Whether `base_url` points at a listed vendor's native Responses gateway, so a
/// provider whose stored `apiFormat` predates the preset's switch to
/// `openai_responses` still gets the `NativeResponses` catalog without a re-save.
pub fn is_codex_native_responses_url(base_url: &str) -> bool {
    if !crate::codex_config::codex_url_host_matches_any(base_url, CODEX_NATIVE_RESPONSES_HOSTS) {
        return false;
    }
    if is_chat_completions_url(base_url) {
        return false;
    }
    let lower = base_url.to_ascii_lowercase();
    !CODEX_NATIVE_RESPONSES_CHAT_PATH_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Resolve the model-catalog tool profile for a Codex provider: official cards and
/// known native Responses hosts are `NativeResponses`; other providers follow their
/// `apiFormat` ([`CodexCatalogToolProfile::from_api_format`]).
///
/// [`CodexCatalogToolProfile::from_api_format`]: crate::codex_config::CodexCatalogToolProfile::from_api_format
pub fn resolve_codex_catalog_tool_profile(
    provider: &Provider,
) -> crate::codex_config::CodexCatalogToolProfile {
    use crate::codex_config::CodexCatalogToolProfile;
    if is_codex_official_provider(provider) {
        return CodexCatalogToolProfile::NativeResponses;
    }

    // Defensive fallback for providers saved in SQLite before their preset
    // switched to `openai_responses` (the #6944 reporter reinstalled to no
    // effect precisely because the stale `apiFormat` lives in the DB row): a
    // base_url on a listed vendor's native Responses gateway forces the
    // NativeResponses catalog. Chat-endpoint paths are deliberately excluded —
    // see `CODEX_NATIVE_RESPONSES_CHAT_PATH_MARKERS`.
    if let Some(base_url) = provider
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .and_then(extract_codex_base_url_from_toml)
        .or_else(|| {
            provider
                .settings_config
                .get("base_url")
                .or_else(|| provider.settings_config.get("baseURL"))
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
    {
        if is_codex_native_responses_url(&base_url) {
            return CodexCatalogToolProfile::NativeResponses;
        }
    }

    let api_format = provider
        .meta
        .as_ref()
        .and_then(|m| m.api_format.as_deref())
        .or_else(|| {
            provider
                .settings_config
                .get("api_format")
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            provider
                .settings_config
                .get("apiFormat")
                .and_then(|v| v.as_str())
        });
    CodexCatalogToolProfile::from_api_format(api_format)
}

fn is_chat_completions_url(value: &str) -> bool {
    value
        .trim_end_matches('/')
        .to_ascii_lowercase()
        .ends_with("/chat/completions")
}

fn extract_codex_base_url_from_toml(config_text: &str) -> Option<String> {
    // Canonical parser lives in codex_config.
    crate::codex_config::extract_codex_base_url(config_text)
}
