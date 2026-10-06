//! Grok (xAI) 官方订阅额度查询
//!
//! 读取 Grok CLI 的 OAuth 凭据（~/.grok/auth.json），调用 Grok Build CLI
//! `/usage` 所用的 JSON 账单接口查询 SuperGrok 订阅的 credit 用量。
//!
//! - 凭据：auth.json 是以 OIDC scope URL 为 key 的 map，优先 SuperGrok 的
//!   `https://auth.x.ai::<client-id>` 条目，回退 legacy session 条目；`key`
//!   字段即 Bearer token，`user_id` 随请求作为 `x-userid` 发送。
//! - 查询：与 xai-org/grok-build `xai-grok-shell/src/extensions/billing.rs`
//!   一致，`GET https://cli-chat-proxy.grok.com/v1/billing?format=credits`，
//!   响应是 `GetGrokCreditsConfig` 的 proto3 JSON（camelCase，零值字段省略）。
//! - token 刷新由 Grok CLI 自己负责（访问令牌约 6 小时过期，CLI 下次运行时
//!   用 refresh_token 换新），本模块只读不刷新；过期但仍有 refresh_token 时
//!   报“待刷新”，刷新令牌也没了才引导用户重新 `grok login`。

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::services::subscription::{
    CredentialStatus, QuotaTier, SubscriptionQuota, TIER_CREDITS, TIER_MONTHLY, TIER_WEEKLY_LIMIT,
};

const GROK_BILLING_URL: &str = "https://cli-chat-proxy.grok.com/v1/billing?format=credits";

/// Grok CLI 访问 cli-chat-proxy 时携带的 `X-XAI-Token-Auth` 值
/// （`xai_grok_login::GrokComConfig::default().token_header`）
const GROK_TOKEN_AUTH: &str = "xai-grok-cli";

/// SuperGrok（OIDC）条目的 scope 前缀
const OIDC_SCOPE_PREFIX: &str = "https://auth.x.ai::";
/// 旧版 `grok login` 的 session scope
const LEGACY_SESSION_SCOPE: &str = "https://accounts.x.ai/sign-in";

const RELOGIN_HINT: &str = "Please re-login with `grok login`.";

// ── 凭据读取 ──────────────────────────────────────────────

/// auth.json 中选中条目的凭据
struct GrokAuth {
    access_token: String,
    /// 旧条目可能没有
    user_id: Option<String>,
}

/// (auth, status, message)
type GrokCredentials = (Option<GrokAuth>, CredentialStatus, Option<String>);

/// 读取 Grok CLI 的 OAuth 凭据（~/.grok/auth.json，目录可被设置覆盖）
fn read_grok_credentials() -> GrokCredentials {
    let auth_path = crate::grok_config::get_grok_config_dir().join("auth.json");

    if !auth_path.exists() {
        return (None, CredentialStatus::NotFound, None);
    }

    let content = match std::fs::read_to_string(&auth_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to read Grok auth file: {e}")),
            );
        }
    };

    parse_grok_auth_json(&content)
}

/// 解析 auth.json：顶层是 scope → 条目的 map，选出首选条目并检查过期
fn parse_grok_auth_json(content: &str) -> GrokCredentials {
    let parsed: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            return (
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to parse Grok auth JSON: {e}")),
            );
        }
    };

    let root = match parsed.as_object() {
        Some(o) => o,
        None => {
            return (
                None,
                CredentialStatus::ParseError,
                Some("Grok auth.json root is not an object".to_string()),
            );
        }
    };

    let entry = match select_preferred_entry(root) {
        Some(e) => e,
        None => {
            return (
                None,
                CredentialStatus::ParseError,
                Some("Grok auth.json contains no usable access token".to_string()),
            );
        }
    };

    let auth = GrokAuth {
        // select_preferred_entry 已保证 key 非空
        access_token: entry
            .get("key")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        user_id: entry
            .get("user_id")
            .and_then(|v| v.as_str())
            .filter(|id| !id.is_empty())
            .map(str::to_string),
    };

    if let Some(expires_at) = entry.get("expires_at").and_then(|v| v.as_str()) {
        if is_iso_expired(expires_at) {
            // 访问令牌几小时一换，Grok 下次运行时用刷新令牌换新的并写回 auth.json；
            // 刷新令牌被永久拒绝时 Grok 会把这条记录删掉，所以还在就不算登录过期。
            let refreshable = entry
                .get("refresh_token")
                .and_then(|v| v.as_str())
                .is_some_and(|t| !t.is_empty());
            return if refreshable {
                (
                    Some(auth),
                    CredentialStatus::RefreshPending,
                    Some(
                        "Access token has expired; Grok refreshes it the next time it runs"
                            .to_string(),
                    ),
                )
            } else {
                (
                    Some(auth),
                    CredentialStatus::Expired,
                    Some("Grok OAuth token has expired".to_string()),
                )
            };
        }
    }

    (Some(auth), CredentialStatus::Valid, None)
}

/// 选择首选凭据条目：OIDC（SuperGrok）优先，legacy session 兜底。
///
/// 只接受 `key` 非空的条目——残缺的 OIDC 记录不能遮蔽健康的 legacy 条目。
fn select_preferred_entry(
    root: &serde_json::Map<String, serde_json::Value>,
) -> Option<&serde_json::Map<String, serde_json::Value>> {
    let mut oidc_candidate = None;
    let mut legacy_candidate = None;

    for (scope, value) in root {
        let entry = match value.as_object() {
            Some(e) => e,
            None => continue,
        };
        let has_key = entry
            .get("key")
            .and_then(|v| v.as_str())
            .is_some_and(|k| !k.is_empty());
        if !has_key {
            continue;
        }
        if scope.starts_with(OIDC_SCOPE_PREFIX) {
            oidc_candidate = Some(entry);
        } else if scope == LEGACY_SESSION_SCOPE || scope.contains("/sign-in") {
            legacy_candidate = Some(entry);
        }
    }

    oidc_candidate.or(legacy_candidate)
}

/// 判断 ISO 8601 时间串是否已过期；无法解析时不视为过期
fn is_iso_expired(iso: &str) -> bool {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) {
        dt.timestamp() < now_secs
    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S%.f") {
        dt.and_utc().timestamp() < now_secs
    } else {
        false
    }
}

// ── 账单响应 ──────────────────────────────────────────────

/// `GET /v1/billing?format=credits` 的响应
#[derive(Deserialize)]
struct BillingResponse {
    config: Option<CreditsConfig>,
}

/// `GetGrokCreditsConfig`。只取额度展示所需字段；`monthlyLimit` / `used` /
/// `billingPeriod*` 是旧版字段，官方 CLI 也优先读下面两个。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreditsConfig {
    /// 已用百分比（0–100）。proto3 JSON 省略零值，用量为 0 时字段缺席
    credit_usage_percent: Option<f64>,
    current_period: Option<UsagePeriod>,
}

#[derive(Deserialize)]
struct UsagePeriod {
    /// proto 枚举名，如 `USAGE_PERIOD_TYPE_WEEKLY`
    #[serde(rename = "type")]
    period_type: Option<String>,
    /// 周期结束（即额度重置）时间，RFC 3339
    end: Option<String>,
}

/// 按账单周期类型选 tier 名；未知/缺省类型用通用 credit 额度
fn tier_name_for_period(period_type: Option<&str>) -> &'static str {
    match period_type {
        Some("USAGE_PERIOD_TYPE_WEEKLY") => TIER_WEEKLY_LIMIT,
        Some("USAGE_PERIOD_TYPE_MONTHLY") => TIER_MONTHLY,
        _ => TIER_CREDITS,
    }
}

/// 把账单响应映射为单个 credit 窗口
fn billing_tier(response: BillingResponse) -> Result<QuotaTier, String> {
    let config = response
        .config
        .ok_or("Grok billing response contained no credits config")?;
    let period = config.current_period;

    Ok(QuotaTier {
        name: tier_name_for_period(period.as_ref().and_then(|p| p.period_type.as_deref()))
            .to_string(),
        utilization: config.credit_usage_percent.unwrap_or(0.0).clamp(0.0, 100.0),
        resets_at: period.and_then(|p| p.end),
        used_value_usd: None,
        max_value_usd: None,
    })
}

/// 非 2xx 响应体形如 `{"error": "..."}`；取不到时截取原文
fn error_detail(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("error")?.as_str().map(str::to_string))
        .unwrap_or_else(|| body.chars().take(400).collect())
}

// ── API 查询 ──────────────────────────────────────────────

/// 查询 Grok 官方订阅额度
///
/// 与 claude/codex/gemini 同一约定：瞬时传输失败返回 `Err`（前端 retry +
/// 保留上次成功值），确定性失败返回 `Ok(success:false)`。
async fn query_grok_quota(auth: &GrokAuth) -> Result<SubscriptionQuota, String> {
    let client = crate::http_client::get();

    let mut request = client
        .get(GROK_BILLING_URL)
        .header("Authorization", format!("Bearer {}", auth.access_token))
        .header("X-XAI-Token-Auth", GROK_TOKEN_AUTH)
        .header("x-grok-client-mode", "headless")
        .header("Accept", "application/json")
        .header("User-Agent", "cc-switch")
        .timeout(std::time::Duration::from_secs(15));
    if let Some(user_id) = &auth.user_id {
        request = request.header("x-userid", user_id);
    }

    let resp = match request.send().await {
        Ok(r) => r,
        Err(e) => return Err(format!("Network error: {e}")),
    };

    let status = resp.status();

    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(SubscriptionQuota::error(
            "grokbuild",
            CredentialStatus::Expired,
            format!("Authentication failed (HTTP {status}). {RELOGIN_HINT}"),
        ));
    }

    // 408 是服务端超时，以 Err 传播（前端 retry + keep-last-good）；折叠进下方
    // 通用分支会因前端 isTransientUsageError 只认 5xx/429 为瞬时而清掉 lastGood。
    if status == reqwest::StatusCode::REQUEST_TIMEOUT {
        return Err(format!("Transient HTTP failure (HTTP {status})"));
    }

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Ok(SubscriptionQuota::error(
            "grokbuild",
            CredentialStatus::Valid,
            format!("API error (HTTP {status}): {}", error_detail(&body)),
        ));
    }

    // 先 bytes() 再解析：读体失败是瞬时 → Err；拿到完整响应体后解析失败才是确定性
    let raw = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => return Err(format!("Failed to read API response: {e}")),
    };

    let tier = match serde_json::from_slice::<BillingResponse>(&raw)
        .map_err(|e| e.to_string())
        .and_then(billing_tier)
    {
        Ok(t) => t,
        Err(e) => {
            return Ok(SubscriptionQuota::error(
                "grokbuild",
                CredentialStatus::Valid,
                format!("Failed to parse API response: {e}"),
            ));
        }
    };

    Ok(SubscriptionQuota {
        tool: "grokbuild".to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: None,
        success: true,
        tiers: vec![tier],
        extra_usage: None,
        reset_credits: None,
        credits_balance: None,
        error: None,
        queried_at: Some(now_millis()),
    })
}

/// grokbuild 的订阅额度入口（由 `subscription::get_subscription_quota` 分发）
pub(crate) async fn get_grok_subscription_quota() -> Result<SubscriptionQuota, String> {
    let (auth, status, message) = read_grok_credentials();

    match status {
        CredentialStatus::NotFound => Ok(SubscriptionQuota::not_found("grokbuild")),
        CredentialStatus::ParseError => Ok(SubscriptionQuota::error(
            "grokbuild",
            CredentialStatus::ParseError,
            message.unwrap_or_else(|| "Failed to parse Grok credentials".to_string()),
        )),
        CredentialStatus::Expired | CredentialStatus::RefreshPending => {
            // 即使过期也尝试调用 API（时钟偏差时 token 可能仍有效）
            if let Some(ref auth) = auth {
                let result = query_grok_quota(auth).await?;
                if result.success {
                    return Ok(result);
                }
            }
            let message = message.unwrap_or_else(|| "Grok OAuth token has expired.".to_string());
            let message = if matches!(status, CredentialStatus::Expired) {
                format!("{message} {RELOGIN_HINT}")
            } else {
                message
            };
            Ok(SubscriptionQuota::error("grokbuild", status, message))
        }
        CredentialStatus::Valid => {
            let auth = auth.expect("auth must be Some when status is Valid");
            query_grok_quota(&auth).await
        }
    }
}

// ── 辅助函数 ──────────────────────────────────────────────

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tier_from(json: serde_json::Value) -> Result<QuotaTier, String> {
        billing_tier(serde_json::from_value(json).expect("valid billing JSON"))
    }

    #[test]
    fn credits_config_maps_to_weekly_tier() {
        // 2026-10 实测的 `format=credits` 响应
        let tier = tier_from(serde_json::json!({
            "config": {
                "currentPeriod": {
                    "type": "USAGE_PERIOD_TYPE_WEEKLY",
                    "start": "2026-10-03T11:44:50.508758+00:00",
                    "end": "2026-10-10T11:44:50.508758+00:00"
                },
                "creditUsagePercent": 3.0,
                "onDemandCap": {"val": 0},
                "onDemandUsed": {"val": 0},
                "productUsage": [
                    {"product": "GrokBuild", "usagePercent": 3.0},
                    {"product": "GrokChat"}
                ],
                "isUnifiedBillingUser": true,
                "prepaidBalance": {"val": 0},
                "topUpMethod": "TOP_UP_METHOD_SAVED_PAYMENT_METHOD",
                "billingPeriodStart": "2026-10-03T11:44:50.508758+00:00",
                "billingPeriodEnd": "2026-10-10T11:44:50.508758+00:00"
            }
        }))
        .expect("tier");
        assert_eq!(tier.name, TIER_WEEKLY_LIMIT);
        assert_eq!(tier.utilization, 3.0);
        assert_eq!(
            tier.resets_at.as_deref(),
            Some("2026-10-10T11:44:50.508758+00:00")
        );
    }

    #[test]
    fn omitted_percent_reads_as_zero_usage() {
        // proto3 JSON 省略零值：没用过额度时没有 creditUsagePercent
        let tier = tier_from(serde_json::json!({
            "config": {
                "currentPeriod": {
                    "type": "USAGE_PERIOD_TYPE_MONTHLY",
                    "end": "2026-11-01T00:00:00Z"
                }
            }
        }))
        .expect("tier");
        assert_eq!(tier.name, TIER_MONTHLY);
        assert_eq!(tier.utilization, 0.0);
    }

    #[test]
    fn unknown_or_missing_period_falls_back_to_credits_tier() {
        let tier = tier_from(serde_json::json!({
            "config": {"creditUsagePercent": 140.0}
        }))
        .expect("tier");
        assert_eq!(tier.name, TIER_CREDITS);
        assert_eq!(tier.utilization, 100.0);
        assert_eq!(tier.resets_at, None);

        assert_eq!(
            tier_name_for_period(Some("USAGE_PERIOD_TYPE_UNSPECIFIED")),
            TIER_CREDITS
        );
    }

    #[test]
    fn missing_config_is_error() {
        assert!(tier_from(serde_json::json!({"config": null})).is_err());
        assert!(tier_from(serde_json::json!({})).is_err());
    }

    #[test]
    fn error_detail_prefers_json_error_field() {
        assert_eq!(
            error_detail(r#"{"error":"Invalid or expired credentials"}"#),
            "Invalid or expired credentials"
        );
        assert_eq!(error_detail("upstream down"), "upstream down");
        assert_eq!(error_detail(&"x".repeat(1000)).len(), 400);
    }

    fn access_token(credentials: &GrokCredentials) -> Option<&str> {
        credentials.0.as_ref().map(|a| a.access_token.as_str())
    }

    #[test]
    fn auth_json_prefers_oidc_entry_over_legacy() {
        let content = r#"{
            "https://accounts.x.ai/sign-in": {"key": "legacy-token"},
            "https://auth.x.ai::client-id": {"key": "oidc-token", "user_id": "user-1"}
        }"#;
        let credentials = parse_grok_auth_json(content);
        assert_eq!(access_token(&credentials), Some("oidc-token"));
        assert_eq!(
            credentials.0.as_ref().and_then(|a| a.user_id.as_deref()),
            Some("user-1")
        );
        assert!(matches!(credentials.1, CredentialStatus::Valid));
    }

    #[test]
    fn auth_json_empty_oidc_key_falls_back_to_legacy() {
        // 残缺 OIDC 记录不遮蔽健康的 legacy 条目
        let content = r#"{
            "https://auth.x.ai::client-id": {"key": ""},
            "https://accounts.x.ai/sign-in": {"key": "legacy-token"}
        }"#;
        let credentials = parse_grok_auth_json(content);
        assert_eq!(access_token(&credentials), Some("legacy-token"));
        assert!(credentials.0.as_ref().is_some_and(|a| a.user_id.is_none()));
        assert!(matches!(credentials.1, CredentialStatus::Valid));
    }

    #[test]
    fn auth_json_expired_entry_reports_expired() {
        let content = r#"{
            "https://auth.x.ai::client-id": {
                "key": "token",
                "expires_at": "2020-01-01T00:00:00.000Z"
            }
        }"#;
        let credentials = parse_grok_auth_json(content);
        assert_eq!(access_token(&credentials), Some("token"));
        assert!(matches!(credentials.1, CredentialStatus::Expired));
        assert!(credentials.2.is_some());
    }

    #[test]
    fn auth_json_expired_entry_with_refresh_token_is_refresh_pending() {
        let status = |refresh_token: &str| {
            let content = serde_json::json!({
                "https://auth.x.ai::client-id": {
                    "key": "token",
                    "expires_at": "2020-01-01T00:00:00.000Z",
                    "refresh_token": refresh_token
                }
            })
            .to_string();
            parse_grok_auth_json(&content).1
        };
        assert!(matches!(status("rt"), CredentialStatus::RefreshPending));
        // 空的刷新令牌换不了新令牌：真的要重新登录。
        assert!(matches!(status(""), CredentialStatus::Expired));
    }

    #[test]
    fn auth_json_without_usable_entry_is_parse_error() {
        let credentials = parse_grok_auth_json(r#"{"other-scope": {"key": "x"}}"#);
        assert!(credentials.0.is_none());
        assert!(matches!(credentials.1, CredentialStatus::ParseError));
    }
}
