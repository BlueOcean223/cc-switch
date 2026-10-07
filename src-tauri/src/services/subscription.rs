//! 官方订阅额度查询服务
//!
//! 读取 CLI 工具的已有 OAuth 凭据，查询官方订阅额度。
//! 第一层：仅读取凭据，不实现登录/刷新。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use std::collections::{HashMap, HashSet};

use crate::config;
use crate::http_client::read_json;

// ── 数据类型 ──────────────────────────────────────────────

/// 凭据状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialStatus {
    Valid,
    Expired,
    /// 访问令牌过期，但刷新令牌还能用：客户端下次运行时自己会换新的，不用重新登录。
    RefreshPending,
    NotFound,
    ParseError,
}

/// 单个限速窗口（如 5小时会话、7天周期）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaTier {
    /// 窗口标识：five_hour, seven_day, seven_day_fable, seven_day_opus 等
    pub name: String,
    /// 使用百分比 0–100
    pub utilization: f64,
    /// ISO 8601 重置时间
    pub resets_at: Option<String>,
    /// ZenMux: 已用额度（USD）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_value_usd: Option<f64>,
    /// ZenMux: 窗口上限（USD）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value_usd: Option<f64>,
}

/// 超额使用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraUsage {
    pub is_enabled: bool,
    pub monthly_limit: Option<f64>,
    pub used_credits: Option<f64>,
    pub utilization: Option<f64>,
    pub currency: Option<String>,
}

/// ChatGPT 订阅存下的限额重置次数（Codex「存下重置、需要时再用」）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetCredits {
    /// 每一次可用重置的到期时间（ISO 8601），先到期的在前，不过期的是 None 排最后；
    /// 长度就是可用次数（只收 available 且查询时还没过期的）
    pub expires_at: Vec<Option<String>>,
}

/// 订阅额度查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionQuota {
    pub tool: String,
    pub credential_status: CredentialStatus,
    pub credential_message: Option<String>,
    pub success: bool,
    pub tiers: Vec<QuotaTier>,
    pub extra_usage: Option<ExtraUsage>,
    /// 只有 ChatGPT 订阅（codex / codex_oauth）有；没查到时为 None，不影响额度本身
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_credits: Option<ResetCredits>,
    /// ChatGPT 订阅买的 Codex Credits 余额（额度用完后才扣）；没有、不限量或为 0 时为 None
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits_balance: Option<f64>,
    pub error: Option<String>,
    pub queried_at: Option<i64>,
}

impl SubscriptionQuota {
    pub(crate) fn not_found(tool: &str) -> Self {
        Self {
            tool: tool.to_string(),
            credential_status: CredentialStatus::NotFound,
            credential_message: None,
            success: false,
            tiers: vec![],
            extra_usage: None,
            reset_credits: None,
            credits_balance: None,
            error: None,
            queried_at: None,
        }
    }

    pub(crate) fn error(tool: &str, status: CredentialStatus, message: String) -> Self {
        Self {
            tool: tool.to_string(),
            credential_status: status,
            credential_message: Some(message.clone()),
            success: false,
            tiers: vec![],
            extra_usage: None,
            reset_credits: None,
            credits_balance: None,
            error: Some(message),
            queried_at: Some(now_millis()),
        }
    }
}

// ── Claude 凭据读取 ──────────────────────────────────────

/// Claude OAuth 凭据文件中的嵌套结构
#[derive(Deserialize)]
struct ClaudeOAuthEntry {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: Option<serde_json::Value>,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(rename = "refreshTokenExpiresAt")]
    refresh_token_expires_at: Option<serde_json::Value>,
}

/// 读取 Claude OAuth 凭据
///
/// 按优先级尝试以下来源：
/// 1. macOS Keychain (service: "Claude Code-credentials")
/// 2. 凭据文件 ~/.claude/.credentials.json
///
/// JSON 格式（两种 key 都兼容）：
/// {"claudeAiOauth": {"accessToken": "...", "expiresAt": ...}}
/// {"claude.ai_oauth": {"accessToken": "...", "expiresAt": ...}}
fn read_claude_credentials() -> (Option<String>, CredentialStatus, Option<String>) {
    // 来源 1: macOS Keychain
    #[cfg(target_os = "macos")]
    {
        if let Some(result) = read_claude_credentials_from_keychain() {
            return result;
        }
    }

    // 来源 2: 凭据文件
    read_claude_credentials_from_file()
}

const CLAUDE_KEYCHAIN_SERVICE: &str = "Claude Code-credentials";

/// Claude Code 存 OAuth 凭据的 Keychain 服务名候选，按优先级排列。
///
/// Claude Code（2.1.284 `wN("-credentials")`）在设了 `CLAUDE_CONFIG_DIR` 时给服务名加
/// `-` + sha256(该环境变量原文) 的前 8 位十六进制，没设时不加后缀。cc-switch 拿不到
/// Claude 进程的环境变量，只能按覆盖目录推：shell 展开 `~` 后的路径，以及带末尾
/// `/` 的写法；覆盖目录就是默认的 `~/.claude` 时环境变量多半没设，再试无后缀的名字。
/// 没设覆盖目录时反过来，先试无后缀，再试显式设成默认目录的情况。
fn claude_keychain_services(override_dir: Option<&Path>, default_dir: &Path) -> Vec<String> {
    let hashed = |dir: &str| {
        let digest = format!("{:x}", Sha256::digest(dir.as_bytes()));
        format!("{CLAUDE_KEYCHAIN_SERVICE}-{}", &digest[..8])
    };
    let default = default_dir.to_string_lossy();
    match override_dir {
        None => vec![CLAUDE_KEYCHAIN_SERVICE.to_string(), hashed(&default)],
        Some(dir) => {
            let dir = dir.to_string_lossy();
            let mut services = vec![hashed(&dir), hashed(&format!("{dir}/"))];
            if dir == default {
                services.push(CLAUDE_KEYCHAIN_SERVICE.to_string());
            }
            services
        }
    }
}

/// 用 `security find-generic-password -w` 读 macOS Keychain 里的一条密码。
/// 没有这一条、内容为空、读不出来（访问被拒、`security` 跑不起来）都返回 None，
/// 调用方回退到凭据文件。
#[cfg(target_os = "macos")]
fn read_keychain_password(service: &str, account: Option<&str>) -> Option<String> {
    let mut command = std::process::Command::new("security");
    command.args(["find-generic-password", "-s", service]);
    if let Some(account) = account {
        command.args(["-a", account]);
    }
    let output = command.arg("-w").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let secret = String::from_utf8(output.stdout).ok()?;
    let secret = secret.trim();
    (!secret.is_empty()).then(|| secret.to_string())
}

/// 从 macOS Keychain 读取 Claude 凭据
#[cfg(target_os = "macos")]
fn read_claude_credentials_from_keychain(
) -> Option<(Option<String>, CredentialStatus, Option<String>)> {
    let override_dir = crate::settings::get_claude_override_dir();
    let default_dir = config::get_home_dir().join(".claude");
    claude_keychain_services(override_dir.as_deref(), &default_dir)
        .iter()
        .find_map(|service| {
            read_keychain_password(service, None)
                .map(|json_str| parse_claude_credentials_json(&json_str))
        })
    // 全部没有时回退到文件
}

/// 从文件读取 Claude 凭据
fn read_claude_credentials_from_file() -> (Option<String>, CredentialStatus, Option<String>) {
    let cred_path = config::get_claude_config_dir().join(".credentials.json");

    if !cred_path.exists() {
        return (None, CredentialStatus::NotFound, None);
    }

    let content = match std::fs::read_to_string(&cred_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to read credentials file: {e}")),
            );
        }
    };

    parse_claude_credentials_json(&content)
}

/// 解析 Claude 凭据 JSON（Keychain 和文件共用）
fn parse_claude_credentials_json(
    content: &str,
) -> (Option<String>, CredentialStatus, Option<String>) {
    let parsed: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            return (
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to parse credentials JSON: {e}")),
            );
        }
    };

    // 兼容两种 key 名
    let entry_value = parsed
        .get("claudeAiOauth")
        .or_else(|| parsed.get("claude.ai_oauth"));

    let entry_value = match entry_value {
        Some(v) => v,
        None => {
            return (
                None,
                CredentialStatus::ParseError,
                Some("No OAuth entry found in credentials".to_string()),
            );
        }
    };

    let entry: ClaudeOAuthEntry = match serde_json::from_value(entry_value.clone()) {
        Ok(e) => e,
        Err(e) => {
            return (
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to parse OAuth entry: {e}")),
            );
        }
    };

    let access_token = match entry.access_token {
        Some(t) if !t.is_empty() => t,
        _ => {
            return (
                None,
                CredentialStatus::ParseError,
                Some("accessToken is empty or missing".to_string()),
            );
        }
    };

    // 检查 token 是否过期
    if let Some(expires_at) = entry.expires_at {
        if is_token_expired(&expires_at) {
            // 访问令牌几小时一换，Claude Code 下次运行时用刷新令牌换新的；
            // 刷新令牌还在就不算登录过期。
            let refreshable = entry.refresh_token.is_some_and(|t| !t.is_empty())
                && !entry
                    .refresh_token_expires_at
                    .as_ref()
                    .is_some_and(is_token_expired);
            return if refreshable {
                (
                    Some(access_token),
                    CredentialStatus::RefreshPending,
                    Some(
                        "Access token has expired; Claude Code refreshes it the next time it runs"
                            .to_string(),
                    ),
                )
            } else {
                (
                    Some(access_token),
                    CredentialStatus::Expired,
                    Some("OAuth token has expired".to_string()),
                )
            };
        }
    }

    (Some(access_token), CredentialStatus::Valid, None)
}

/// 判断 token 是否过期，兼容 Unix 时间戳（秒/毫秒）和 ISO 字符串
fn is_token_expired(expires_at: &serde_json::Value) -> bool {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match expires_at {
        serde_json::Value::Number(n) => {
            if let Some(ts) = n.as_u64() {
                // 区分秒和毫秒（毫秒级时间戳大于 1e12）
                let ts_secs = if ts > 1_000_000_000_000 {
                    ts / 1000
                } else {
                    ts
                };
                ts_secs < now_secs
            } else {
                false
            }
        }
        serde_json::Value::String(s) => {
            // 尝试解析 ISO 8601 格式
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                (dt.timestamp() as u64) < now_secs
            } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
            {
                (dt.and_utc().timestamp() as u64) < now_secs
            } else {
                false // 无法解析时不视为过期
            }
        }
        _ => false,
    }
}

// ── Claude API 查询 ──────────────────────────────────────

/// Claude OAuth 用量 API 响应中的单个窗口
#[derive(Deserialize)]
struct ApiUsageWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

/// `limits[]` 中的窗口使用 `percent`，而非旧顶层窗口的 `utilization`。
#[derive(Deserialize)]
struct ApiScopedUsageWindow {
    percent: f64,
    resets_at: Option<String>,
}

/// Claude OAuth 用量 API 响应中的超额用量
#[derive(Deserialize)]
struct ApiExtraUsage {
    is_enabled: Option<bool>,
    monthly_limit: Option<f64>,
    used_credits: Option<f64>,
    utilization: Option<f64>,
    currency: Option<String>,
}

/// 已知的 Claude 用量窗口名称；未知的旧格式窗口仍保留原名称。
pub const TIER_FIVE_HOUR: &str = "five_hour";
pub const TIER_SEVEN_DAY: &str = "seven_day";
/// 内部统一名称：Fable 实际由 `limits[].scope.model` 标识。
pub const TIER_SEVEN_DAY_FABLE: &str = "seven_day_fable";
pub const TIER_SEVEN_DAY_OPUS: &str = "seven_day_opus";
pub const TIER_SEVEN_DAY_SONNET: &str = "seven_day_sonnet";

/// Coding Plan（Kimi / MiniMax）的周窗口 tier 名。与 `coding_plan::query_*`
/// 写入、tray 渲染、commands::provider 扁平化三处共用同一标识。
pub const TIER_WEEKLY_LIMIT: &str = "weekly_limit";

/// 月窗口 tier 名。火山方舟 Agent Plan / Coding Plan 有 5h / 周 / 月 三个展示
/// 窗口（Kimi / MiniMax 只有 5h + 周），月窗口共用此标识；前端 `TIER_I18N_KEYS`
/// 映射到 `subscription.monthly`。
pub const TIER_MONTHLY: &str = "monthly";

/// Codex 免费方案的 30 天（月）滚动窗口 tier 名。付费方案的次要窗口是 7 天
/// (`seven_day`)，免费方案则是 30 天。由 `window_seconds_to_tier_name` 产出、
/// tray 的月分组渲染、前端 `TIER_I18N_KEYS` 映射到 `subscription.thirtyDay`
/// 三处共用同一标识。见 #3651。
pub const TIER_THIRTY_DAY: &str = "30_day";

/// Grok credit 额度窗口的兜底 tier 名。Grok 账单接口只返回一个 credit 用量
/// 窗口，`subscription_grok::tier_name_for_period` 按账单周期类型映射到
/// `weekly_limit` / `monthly`，类型未知或缺省时用此标识；前端 `TIER_I18N_KEYS`
/// 映射到 `subscription.credits`，tray 归入 "c" 分组。
pub const TIER_CREDITS: &str = "credits";

/// Gemini 用量分组名称（按模型而非时间窗口）。`classify_gemini_model` 输出。
pub const TIER_GEMINI_PRO: &str = "gemini_pro";
pub const TIER_GEMINI_FLASH: &str = "gemini_flash";
pub const TIER_GEMINI_FLASH_LITE: &str = "gemini_flash_lite";

const KNOWN_TIERS: &[&str] = &[
    TIER_FIVE_HOUR,
    TIER_SEVEN_DAY,
    TIER_SEVEN_DAY_FABLE,
    TIER_SEVEN_DAY_OPUS,
    TIER_SEVEN_DAY_SONNET,
];

/// 查询 Claude 官方订阅额度
///
/// 瞬时传输失败（网络/超时/读体中断）返回 `Err`（前端 reject → retry + 保留上次
/// 成功值）；确定性失败（鉴权/非 2xx/响应体非法 JSON）返回 `Ok(success:false)`。
/// codex/gemini 两个查询函数遵守同一约定。
async fn query_claude_quota(access_token: &str) -> Result<SubscriptionQuota, String> {
    let client = crate::http_client::get();

    let resp = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let body = resp.bytes().await.unwrap_or_default();
        return Ok(claude_http_error(status, retry_after.as_deref(), &body));
    }

    let body: serde_json::Value = match read_json(resp).await? {
        Ok(body) => body,
        Err(error) => {
            return Ok(SubscriptionQuota::error(
                "claude",
                CredentialStatus::Valid,
                error,
            ))
        }
    };

    Ok(parse_claude_quota(&body))
}

/// 非 2xx 的分类，与 Claude Code 2.1.284 的 `h6()` 一致：401 和带 Anthropic 错误体
/// （`{"error":{"type":"…"}}`）的 403 是鉴权被拒；429 和其余 403 是限流。前端按
/// "Rate limited" 把限流当瞬时失败，沿用上次成功的读数。
fn claude_http_error(
    status: reqwest::StatusCode,
    retry_after: Option<&str>,
    body: &[u8],
) -> SubscriptionQuota {
    let anthropic_error = serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/type").map(serde_json::Value::is_string))
        .unwrap_or(false);
    if status == reqwest::StatusCode::UNAUTHORIZED
        || (status == reqwest::StatusCode::FORBIDDEN && anthropic_error)
    {
        return SubscriptionQuota::error(
            "claude",
            CredentialStatus::Expired,
            format!("Authentication failed (HTTP {status}). Please re-login with Claude CLI."),
        );
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status == reqwest::StatusCode::FORBIDDEN
    {
        let retry = retry_after
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(|secs| format!(", retry after {secs}s"))
            .unwrap_or_default();
        return SubscriptionQuota::error(
            "claude",
            CredentialStatus::Valid,
            format!("Rate limited (HTTP {status}{retry})"),
        );
    }
    SubscriptionQuota::error(
        "claude",
        CredentialStatus::Valid,
        format!(
            "API error (HTTP {status}): {}",
            String::from_utf8_lossy(body)
        ),
    )
}

/// 200 响应里至少要有其中一个键，否则 Claude Code（`ult()`）当作带内错误。
const CLAUDE_USAGE_KEYS: [&str; 8] = [
    "five_hour",
    "seven_day",
    "seven_day_oauth_apps",
    "seven_day_opus",
    "seven_day_sonnet",
    "cinder_cove",
    "extra_usage",
    "limits",
];

/// 兼容旧顶层窗口与新版模型专属周限额，保持查询、缓存和 UI 共用 QuotaTier。
fn parse_claude_quota(body: &serde_json::Value) -> SubscriptionQuota {
    let recognized = body
        .as_object()
        .is_some_and(|o| CLAUDE_USAGE_KEYS.iter().any(|k| o.contains_key(*k)));
    if !recognized {
        return SubscriptionQuota::error(
            "claude",
            CredentialStatus::Valid,
            "Unrecognized usage response".to_string(),
        );
    }
    // 解析已知的 tier 窗口
    let mut tiers = Vec::new();
    for &tier_name in KNOWN_TIERS {
        if let Some(window) = body.get(tier_name) {
            if let Ok(w) = serde_json::from_value::<ApiUsageWindow>(window.clone()) {
                if let Some(util) = w.utilization {
                    tiers.push(QuotaTier {
                        name: tier_name.to_string(),
                        utilization: util,
                        resets_at: w.resets_at,
                        used_value_usd: None,
                        max_value_usd: None,
                    });
                }
            }
        }
    }

    // 也解析未知窗口（API 可能返回新的窗口类型）
    if let Some(obj) = body.as_object() {
        for (key, value) in obj {
            if key == "extra_usage" || key == "limits" || KNOWN_TIERS.contains(&key.as_str()) {
                continue;
            }
            if let Ok(w) = serde_json::from_value::<ApiUsageWindow>(value.clone()) {
                if let Some(util) = w.utilization {
                    tiers.push(QuotaTier {
                        name: key.clone(),
                        utilization: util,
                        resets_at: w.resets_at,
                        used_value_usd: None,
                        max_value_usd: None,
                    });
                }
            }
        }
    }

    // 新版模型专属额度覆盖同名旧窗口。逐条解析，单个异常项目不影响其余额度。
    let mut scoped_tiers = HashSet::new();
    if let Some(limits) = body.get("limits").and_then(serde_json::Value::as_array) {
        for limit in limits {
            if limit.get("kind").and_then(serde_json::Value::as_str) != Some("weekly_scoped")
                || limit.get("group").and_then(serde_json::Value::as_str) != Some("weekly")
                // 不把特定使用场景的子限额合并进整个模型的周限额。
                || limit.pointer("/scope/surface").is_some_and(|v| !v.is_null())
            {
                continue;
            }
            let Some(model) = limit
                .pointer("/scope/model/display_name")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            let tier_name = match model.trim().to_ascii_lowercase().as_str() {
                "fable" => TIER_SEVEN_DAY_FABLE,
                "opus" => TIER_SEVEN_DAY_OPUS,
                "sonnet" => TIER_SEVEN_DAY_SONNET,
                _ => continue,
            };
            let Ok(window) = serde_json::from_value::<ApiScopedUsageWindow>(limit.clone()) else {
                continue;
            };
            if !window.percent.is_finite()
                || window.percent < 0.0
                || !scoped_tiers.insert(tier_name)
            {
                continue;
            }
            // 与 Claude Code 一致：不按 is_active 过滤。0% / resets_at:null
            // 也可能是有效的模型额度；不存在的额度由接口省略。
            let tier = QuotaTier {
                name: tier_name.to_string(),
                utilization: window.percent,
                resets_at: window.resets_at,
                used_value_usd: None,
                max_value_usd: None,
            };
            if let Some(existing) = tiers.iter_mut().find(|t| t.name == tier_name) {
                *existing = tier;
            } else {
                tiers.push(tier);
            }
        }
    }
    tiers.sort_by_key(|tier| {
        KNOWN_TIERS
            .iter()
            .position(|&name| name == tier.name)
            .unwrap_or(KNOWN_TIERS.len())
    });

    // 解析超额使用
    let extra_usage = body.get("extra_usage").and_then(|v| {
        serde_json::from_value::<ApiExtraUsage>(v.clone())
            .ok()
            .map(|e| ExtraUsage {
                is_enabled: e.is_enabled.unwrap_or(false),
                monthly_limit: e.monthly_limit,
                used_credits: e.used_credits,
                utilization: e.utilization,
                currency: e.currency,
            })
    });

    SubscriptionQuota {
        tool: "claude".to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: None,
        success: true,
        tiers,
        extra_usage,
        reset_credits: None,
        credits_balance: None,
        error: None,
        queried_at: Some(now_millis()),
    }
}

// ── Codex 凭据读取 ──────────────────────────────────────

#[derive(Deserialize)]
struct CodexAuthJson {
    auth_mode: Option<String>,
    tokens: Option<CodexTokens>,
    last_refresh: Option<String>,
}

#[derive(Deserialize)]
struct CodexTokens {
    access_token: Option<String>,
    account_id: Option<String>,
    refresh_token: Option<String>,
}

/// (access_token, account_id, status, message)
pub(crate) type CodexCredentials = (
    Option<String>,
    Option<String>,
    CredentialStatus,
    Option<String>,
);

/// 读取 Codex OAuth 凭据
///
/// 按优先级尝试以下来源：
/// 1. macOS Keychain (service: "Codex Auth"，账户按 Codex 配置目录区分)
/// 2. 凭据文件 ~/.codex/auth.json
///
/// 仅 auth_mode == "chatgpt" (OAuth) 时有效，API key 模式不支持用量查询。
fn read_codex_credentials() -> CodexCredentials {
    #[cfg(target_os = "macos")]
    {
        if let Some(result) = read_codex_credentials_from_keychain() {
            return result;
        }
    }

    read_codex_credentials_from_file()
}

/// 从 macOS Keychain 读取 Codex 凭据。服务名所有配置目录共用，必须带上账户名：
/// 只按服务名查，本机有别的配置目录的登录时 `security` 返回第一条匹配的。
#[cfg(target_os = "macos")]
fn read_codex_credentials_from_keychain() -> Option<CodexCredentials> {
    let account = codex_keychain_account(&crate::codex_config::get_codex_config_dir());
    read_keychain_password("Codex Auth", Some(&account))
        .map(|json_str| parse_codex_credentials_json(&json_str))
}

/// Codex 在 Keychain 里存登录用的账户名：`cli|` 加规范化后的 Codex 配置目录路径的
/// SHA-256 十六进制前 16 位（codex-rs `login/src/auth/storage.rs` 的 `compute_store_key`）。
/// 规范化失败时用原路径，和上游一致。
#[cfg(target_os = "macos")]
fn codex_keychain_account(codex_home: &std::path::Path) -> String {
    let canonical = codex_home
        .canonicalize()
        .unwrap_or_else(|_| codex_home.to_path_buf());
    let hex = crate::live::engine::sha256_hex(canonical.to_string_lossy().as_bytes());
    format!("cli|{}", &hex[..16])
}

/// 从文件读取 Codex 凭据
fn read_codex_credentials_from_file() -> CodexCredentials {
    let auth_path = crate::codex_config::get_codex_auth_path();

    if !auth_path.exists() {
        return (None, None, CredentialStatus::NotFound, None);
    }

    let content = match std::fs::read_to_string(&auth_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                None,
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to read Codex auth file: {e}")),
            );
        }
    };

    parse_codex_credentials_json(&content)
}

/// 解析 Codex 凭据 JSON（Keychain 和文件共用）
pub(crate) fn parse_codex_credentials_json(content: &str) -> CodexCredentials {
    let auth: CodexAuthJson = match serde_json::from_str(content) {
        Ok(a) => a,
        Err(e) => {
            return (
                None,
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to parse Codex auth JSON: {e}")),
            );
        }
    };

    // 仅 OAuth 模式有用量数据
    if auth.auth_mode.as_deref() != Some("chatgpt") {
        return (
            None,
            None,
            CredentialStatus::NotFound,
            Some("Codex not using OAuth mode".to_string()),
        );
    }

    let tokens = match auth.tokens {
        Some(t) => t,
        None => {
            return (
                None,
                None,
                CredentialStatus::ParseError,
                Some("No tokens in Codex auth".to_string()),
            );
        }
    };

    let access_token = match tokens.access_token {
        Some(t) if !t.is_empty() => t,
        _ => {
            return (
                None,
                None,
                CredentialStatus::ParseError,
                Some("access_token is empty or missing".to_string()),
            );
        }
    };

    // 与 codex-rs `should_refresh_proactively` 一致：access_token 带 `exp` 就只看
    // `exp`，解析不出来才退回"距上次刷新超过 8 天"。
    let expired = match jwt_exp(&access_token) {
        Some(exp) => exp <= now_millis() / 1000,
        None => auth
            .last_refresh
            .as_deref()
            .is_some_and(is_codex_token_stale),
    };
    if !expired {
        return (
            Some(access_token),
            tokens.account_id,
            CredentialStatus::Valid,
            None,
        );
    }
    // 有刷新令牌时 Codex CLI 下次运行会自己换新的，不用重新登录
    if tokens.refresh_token.is_some_and(|t| !t.is_empty()) {
        (
            Some(access_token),
            tokens.account_id,
            CredentialStatus::RefreshPending,
            Some(
                "Access token has expired; Codex CLI refreshes it the next time it runs"
                    .to_string(),
            ),
        )
    } else {
        (
            Some(access_token),
            tokens.account_id,
            CredentialStatus::Expired,
            Some("Codex OAuth token has expired. Please re-login with Codex CLI.".to_string()),
        )
    }
}

/// 读 JWT payload 里的 `exp`（秒）。格式同 codex-rs `decode_jwt_payload`：三段
/// 都非空，payload 是 base64url（无填充）编码的 JSON。
fn jwt_exp(token: &str) -> Option<i64> {
    use base64::Engine;
    let mut parts = token.split('.');
    let (Some(header), Some(payload), Some(signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    if header.is_empty() || payload.is_empty() || signature.is_empty() {
        return None;
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()?
        .get("exp")?
        .as_i64()
}

/// 判断 Codex token 是否可能过期（Codex CLI 在 >8 天时自动刷新）
fn is_codex_token_stale(last_refresh: &str) -> bool {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(last_refresh) {
        let age_secs = now_secs.saturating_sub(dt.timestamp() as u64);
        age_secs > 8 * 24 * 3600
    } else {
        false
    }
}

// ── Codex API 查询 ──────────────────────────────────────

#[derive(Deserialize)]
struct CodexRateLimitWindow {
    used_percent: Option<f64>,
    limit_window_seconds: Option<i64>,
    reset_at: Option<i64>,
}

#[derive(Deserialize)]
struct CodexRateLimit {
    primary_window: Option<CodexRateLimitWindow>,
    secondary_window: Option<CodexRateLimitWindow>,
}

#[derive(Deserialize)]
struct CodexUsageResponse {
    rate_limit: Option<CodexRateLimit>,
    /// 原样收下再挑字段：形状不对时只是不显示余额，不能让整份额度解析失败
    credits: Option<serde_json::Value>,
}

/// `wham/usage` 的 `credits`：`{has_credits, unlimited, balance}`，balance 实测是字符串
/// （"62500"），也收数字（同 CodexBar）。不限量、没有或为 0 时不显示
fn parse_codex_credits_balance(credits: &serde_json::Value) -> Option<f64> {
    if credits.get("has_credits").and_then(|v| v.as_bool()) != Some(true)
        || credits.get("unlimited").and_then(|v| v.as_bool()) == Some(true)
    {
        return None;
    }
    let balance = match credits.get("balance")? {
        serde_json::Value::Number(n) => n.as_f64()?,
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok()?,
        _ => return None,
    };
    (balance.is_finite() && balance > 0.0).then_some(balance)
}

/// 根据窗口秒数映射到 tier 名称（与 Claude 的命名兼容以复用前端 i18n）
fn window_seconds_to_tier_name(secs: i64) -> String {
    match secs {
        18000 => TIER_FIVE_HOUR.to_string(),
        604800 => TIER_SEVEN_DAY.to_string(),
        // Codex 免费方案的 30 天窗口。显式映射到常量，与 tray 月分组、前端
        // TIER_I18N_KEYS 保持同一标识（否则动态回退虽也得到 "30_day"，但字符串
        // 分散在多处、易和托盘/前端白名单脱节）。见 #3651。
        2_592_000 => TIER_THIRTY_DAY.to_string(),
        s => {
            let hours = s / 3600;
            if hours >= 24 {
                format!("{}_day", hours / 24)
            } else {
                format!("{}_hour", hours)
            }
        }
    }
}

/// Unix 时间戳（秒）转 ISO 8601 字符串
fn unix_ts_to_iso(ts: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.to_rfc3339())
}

#[derive(Deserialize)]
struct CodexResetCreditsResponse {
    #[serde(default)]
    credits: Vec<CodexResetCreditEntry>,
}

#[derive(Deserialize)]
struct CodexResetCreditEntry {
    status: Option<String>,
    expires_at: Option<String>,
}

/// 解析 `wham/rate-limit-reset-credits`：不信 `available_count`，自己按
/// status == "available" 且没过期来数（同 CodexBar），先到期的排前面
fn parse_codex_reset_credits(
    raw: &[u8],
    now: chrono::DateTime<chrono::Utc>,
) -> Option<ResetCredits> {
    let body: CodexResetCreditsResponse = serde_json::from_slice(raw).ok()?;
    let mut expiries: Vec<Option<chrono::DateTime<chrono::Utc>>> = body
        .credits
        .into_iter()
        .filter(|credit| credit.status.as_deref() == Some("available"))
        .filter_map(|credit| match credit.expires_at {
            None => Some(None),
            // 认不出的到期时间当作不过期，宁可多显示一次也不吞掉
            Some(raw) => match chrono::DateTime::parse_from_rfc3339(&raw) {
                Ok(at) => {
                    let at = at.with_timezone(&chrono::Utc);
                    (at > now).then_some(Some(at))
                }
                Err(_) => Some(None),
            },
        })
        .collect();
    // None（不过期）排最后
    expiries.sort_by_key(|at| (at.is_none(), *at));
    Some(ResetCredits {
        expires_at: expiries
            .into_iter()
            .map(|at| at.map(|at| at.to_rfc3339()))
            .collect(),
    })
}

/// 查存下的限额重置次数。附带查询：任何失败都只返回 None，不连累额度本身
async fn query_codex_reset_credits(
    access_token: &str,
    account_id: Option<&str>,
) -> Option<ResetCredits> {
    let mut req = crate::http_client::get()
        .get("https://chatgpt.com/backend-api/wham/rate-limit-reset-credits")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-Agent", "codex-cli")
        .header("Accept", "application/json")
        .header("OpenAI-Beta", "codex-1");
    if let Some(id) = account_id {
        req = req.header("ChatGPT-Account-Id", id);
    }
    let resp = req
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        log::debug!("Codex reset credits query failed: HTTP {}", resp.status());
        return None;
    }
    let raw = resp.bytes().await.ok()?;
    parse_codex_reset_credits(&raw, chrono::Utc::now())
}

/// 查询 Codex / ChatGPT 反代订阅额度（连同存下的重置次数，两个请求并行）
///
/// 参数化 `tool_label` 和 `expired_message` 让该函数可被两个调用点共用：
/// - `"codex"` + "Please re-login with Codex CLI."（CLI 凭据路径）
/// - `"codex_oauth"` + "Please re-login via cc-switch."（cc-switch 自管 OAuth 路径）
pub(crate) async fn query_codex_quota(
    access_token: &str,
    account_id: Option<&str>,
    tool_label: &str,
    expired_message: &str,
) -> Result<SubscriptionQuota, String> {
    let (quota, reset_credits) = tokio::join!(
        query_codex_usage(access_token, account_id, tool_label, expired_message),
        query_codex_reset_credits(access_token, account_id),
    );
    let mut quota = quota?;
    if quota.success {
        quota.reset_credits = reset_credits;
    }
    Ok(quota)
}

async fn query_codex_usage(
    access_token: &str,
    account_id: Option<&str>,
    tool_label: &str,
    expired_message: &str,
) -> Result<SubscriptionQuota, String> {
    let client = crate::http_client::get();

    let mut req = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-Agent", "codex-cli")
        .header("Accept", "application/json");

    if let Some(id) = account_id {
        req = req.header("ChatGPT-Account-Id", id);
    }

    let resp = req
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(SubscriptionQuota::error(
            tool_label,
            CredentialStatus::Expired,
            format!("{expired_message} (HTTP {status})"),
        ));
    }

    let body: CodexUsageResponse = match read_json(resp).await? {
        Ok(body) => body,
        Err(error) => {
            return Ok(SubscriptionQuota::error(
                tool_label,
                CredentialStatus::Valid,
                error,
            ))
        }
    };

    let mut tiers = Vec::new();

    if let Some(rate_limit) = body.rate_limit {
        for window in [rate_limit.primary_window, rate_limit.secondary_window]
            .into_iter()
            .flatten()
        {
            if let Some(used) = window.used_percent {
                tiers.push(QuotaTier {
                    name: window
                        .limit_window_seconds
                        .map(window_seconds_to_tier_name)
                        .unwrap_or_else(|| "unknown".to_string()),
                    utilization: used,
                    resets_at: window.reset_at.and_then(unix_ts_to_iso),
                    used_value_usd: None,
                    max_value_usd: None,
                });
            }
        }
    }

    Ok(SubscriptionQuota {
        tool: tool_label.to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: None,
        success: true,
        tiers,
        extra_usage: None,
        reset_credits: None,
        credits_balance: body.credits.as_ref().and_then(parse_codex_credits_balance),
        error: None,
        queried_at: Some(now_millis()),
    })
}

// ── Gemini 凭据读取 ──────────────────────────────────────

/// Gemini OAuth 凭据文件格式（~/.gemini/oauth_creds.json）
#[derive(Deserialize)]
struct GeminiOAuthCredsFile {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expiry_date: Option<i64>, // 毫秒时间戳
}

/// (access_token, refresh_token, status, message)
type GeminiCredentials = (
    Option<String>,
    Option<String>,
    CredentialStatus,
    Option<String>,
);

/// 读取 Gemini OAuth 凭据
///
/// 按优先级尝试以下来源：
/// 1. macOS Keychain (service: "gemini-cli-oauth", account: "main-account")
/// 2. 凭据文件 ~/.gemini/oauth_creds.json（遗留格式）
///
/// 仅 OAuth 认证模式（`oauth-personal`）有效；API key 模式无法查询官方用量。
fn read_gemini_credentials() -> GeminiCredentials {
    #[cfg(target_os = "macos")]
    {
        if let Some(result) = read_gemini_credentials_from_keychain() {
            return result;
        }
    }

    read_gemini_credentials_from_file()
}

/// 从 macOS Keychain 读取 Gemini 凭据
#[cfg(target_os = "macos")]
fn read_gemini_credentials_from_keychain() -> Option<GeminiCredentials> {
    read_keychain_password("gemini-cli-oauth", Some("main-account"))
        .map(|json_str| parse_gemini_keychain_json(&json_str))
}

/// 解析 Keychain 格式的 Gemini 凭据
///
/// Keychain 格式（keytar）：
/// ```json
/// { "token": { "accessToken": "...", "refreshToken": "...", "expiresAt": 1234 }, "updatedAt": ... }
/// ```
#[cfg(target_os = "macos")]
fn parse_gemini_keychain_json(content: &str) -> GeminiCredentials {
    let parsed: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            return (
                None,
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to parse Gemini keychain JSON: {e}")),
            )
        }
    };

    let token = match parsed.get("token") {
        Some(t) => t,
        None => {
            // Keychain 中可能是扁平格式，尝试文件格式解析
            return parse_gemini_file_json(content);
        }
    };

    let access_token = token
        .get("accessToken")
        .and_then(|v| v.as_str())
        .map(String::from);
    let refresh_token = token
        .get("refreshToken")
        .and_then(|v| v.as_str())
        .map(String::from);
    let expires_at = token.get("expiresAt").and_then(|v| v.as_i64());

    match access_token {
        Some(at) if !at.is_empty() => {
            // expiresAt 是毫秒时间戳
            if let Some(exp_ms) = expires_at {
                if exp_ms < now_millis() {
                    return (
                        Some(at),
                        refresh_token,
                        CredentialStatus::Expired,
                        Some("Gemini access token has expired".to_string()),
                    );
                }
            }
            (Some(at), refresh_token, CredentialStatus::Valid, None)
        }
        _ => (
            None,
            refresh_token,
            CredentialStatus::ParseError,
            Some("accessToken is empty or missing".to_string()),
        ),
    }
}

/// 从文件读取 Gemini 凭据
fn read_gemini_credentials_from_file() -> GeminiCredentials {
    let cred_path = crate::gemini_config::get_gemini_dir().join("oauth_creds.json");
    if !cred_path.exists() {
        return (None, None, CredentialStatus::NotFound, None);
    }

    let content = match std::fs::read_to_string(&cred_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                None,
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to read Gemini credentials: {e}")),
            )
        }
    };

    parse_gemini_file_json(&content)
}

/// 解析文件格式的 Gemini 凭据
///
/// 文件格式（oauth_creds.json）：
/// ```json
/// { "access_token": "...", "refresh_token": "...", "expiry_date": 1234 }
/// ```
fn parse_gemini_file_json(content: &str) -> GeminiCredentials {
    let creds: GeminiOAuthCredsFile = match serde_json::from_str(content) {
        Ok(c) => c,
        Err(e) => {
            return (
                None,
                None,
                CredentialStatus::ParseError,
                Some(format!("Failed to parse Gemini credentials: {e}")),
            )
        }
    };

    let access_token = match creds.access_token {
        Some(t) if !t.is_empty() => t,
        _ => {
            return (
                None,
                creds.refresh_token,
                CredentialStatus::ParseError,
                Some("access_token is empty or missing".to_string()),
            )
        }
    };

    // expiry_date 是毫秒时间戳
    if let Some(exp_ms) = creds.expiry_date {
        if exp_ms < now_millis() {
            return (
                Some(access_token),
                creds.refresh_token,
                CredentialStatus::Expired,
                Some("Gemini access token has expired".to_string()),
            );
        }
    }

    (
        Some(access_token),
        creds.refresh_token,
        CredentialStatus::Valid,
        None,
    )
}

// ── Gemini Token 刷新 ──────────────────────────────────────

/// Gemini OAuth Client 凭据（公开值，来自 Gemini CLI 源码 google-gemini/gemini-cli）
const GEMINI_OAUTH_CLIENT_ID: &str =
    "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
const GEMINI_OAUTH_CLIENT_SECRET: &str = "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";

/// 使用 refresh_token 刷新 Gemini access token
///
/// Google OAuth access_token 仅有 ~1h 有效期，需要定期用 refresh_token 刷新。
/// refresh_token 本身不过期（除非用户撤销授权）。
async fn refresh_gemini_token(refresh_token: &str) -> Option<String> {
    let client = crate::http_client::get();

    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", GEMINI_OAUTH_CLIENT_ID),
            ("client_secret", GEMINI_OAUTH_CLIENT_SECRET),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;
    body.get("access_token")?.as_str().map(String::from)
}

// ── Gemini API 查询 ──────────────────────────────────────

/// loadCodeAssist 响应
#[derive(Deserialize)]
struct GeminiLoadCodeAssistResponse {
    #[serde(rename = "cloudaicompanionProject")]
    cloudaicompanion_project: Option<serde_json::Value>,
    /// 没有时账号还没在 gemini-cli 里完成初始化（onboard）
    #[serde(rename = "currentTier")]
    current_tier: Option<serde_json::Value>,
    #[serde(rename = "ineligibleTiers", default)]
    ineligible_tiers: Vec<GeminiIneligibleTier>,
}

#[derive(Deserialize)]
struct GeminiIneligibleTier {
    #[serde(rename = "reasonMessage")]
    reason_message: Option<String>,
}

/// 配额 bucket
#[derive(Deserialize)]
struct GeminiBucketInfo {
    #[serde(rename = "remainingFraction")]
    remaining_fraction: Option<f64>,
    #[serde(rename = "resetTime")]
    reset_time: Option<String>,
    #[serde(rename = "modelId")]
    model_id: Option<String>,
}

/// retrieveUserQuota 响应
#[derive(Deserialize)]
struct GeminiQuotaResponse {
    buckets: Option<Vec<GeminiBucketInfo>>,
}

/// 从 loadCodeAssist 响应中提取项目 ID
fn extract_project_id(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(obj) => obj
            .get("id")
            .or_else(|| obj.get("projectId"))
            .and_then(|v| v.as_str())
            .map(String::from),
        _ => None,
    }
    .filter(|id| !id.is_empty())
}

const GEMINI_PROJECT_ENV_KEYS: [&str; 2] = ["GOOGLE_CLOUD_PROJECT", "GOOGLE_CLOUD_PROJECT_ID"];

/// 用户给 gemini-cli 配的 Google Cloud 项目（`code_assist/setup.ts`）：`GOOGLE_CLOUD_PROJECT`，其次
/// `GOOGLE_CLOUD_PROJECT_ID`。gemini-cli 启动时把找到的第一个 `.env` 并入进程环境（不覆盖已有变量）；
/// cc-switch 没有工作区目录，只查 `~/.gemini/.env` 和 `~/.env`。macOS 上 GUI 进程拿不到 shell 里
/// export 的变量，多数情况靠的是这两个文件。
fn configured_gemini_project() -> Option<String> {
    let env_file = [
        crate::gemini_config::get_gemini_dir().join(".env"),
        crate::config::get_home_dir().join(".env"),
    ]
    .into_iter()
    .find_map(|path| std::fs::read_to_string(path).ok())
    .unwrap_or_default();
    pick_gemini_project(|key| std::env::var(key).ok(), &env_file)
}

fn pick_gemini_project(env: impl Fn(&str) -> Option<String>, env_file: &str) -> Option<String> {
    GEMINI_PROJECT_ENV_KEYS.iter().find_map(|key| {
        env(key)
            .or_else(|| dotenv_value(env_file, key))
            .filter(|value| !value.is_empty())
    })
}

/// `.env` 里某个键的值（同名键取最后一个），按 dotenv 的规则处理 `export ` 前缀、引号和行尾注释
fn dotenv_value(content: &str, key: &str) -> Option<String> {
    let mut found = None;
    for line in content.lines() {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        let value = value.trim();
        let value = match value.chars().next() {
            Some(quote @ ('"' | '\'' | '`')) => value[1..].split(quote).next().unwrap_or_default(),
            _ => value.split('#').next().unwrap_or_default().trim(),
        };
        found = Some(value.to_string());
    }
    found
}

/// 拿不到项目时的提示，对应 gemini-cli 的几种报错：还没初始化账号、账号不符合任何档位
/// （`IneligibleTierError`）、需要自己配置项目（`ProjectIdRequiredError`）。
fn gemini_missing_project_message(load: &GeminiLoadCodeAssistResponse) -> String {
    if load.current_tier.is_none() {
        return "Gemini CLI has not finished setting up this account. Run gemini once, then refresh."
            .to_string();
    }
    let reasons: Vec<&str> = load
        .ineligible_tiers
        .iter()
        .filter_map(|tier| tier.reason_message.as_deref())
        .filter(|reason| !reason.is_empty())
        .collect();
    if !reasons.is_empty() {
        return reasons.join(", ");
    }
    "This account requires setting GOOGLE_CLOUD_PROJECT or GOOGLE_CLOUD_PROJECT_ID; \
     cc-switch reads it from ~/.gemini/.env or ~/.env."
        .to_string()
}

/// 将 Gemini 模型 ID 分类为 Pro / Flash / Flash Lite
fn classify_gemini_model(model_id: &str) -> &str {
    if model_id.contains("flash-lite") {
        TIER_GEMINI_FLASH_LITE
    } else if model_id.contains("flash") {
        TIER_GEMINI_FLASH
    } else if model_id.contains("pro") {
        TIER_GEMINI_PRO
    } else {
        model_id
    }
}

/// 查询 Gemini 官方订阅额度
///
/// 两步 API 调用，项目的取法同 gemini-cli（`setupUser` 与 `refreshUserQuota`）：
/// 1. loadCodeAssist（带上用户配置的项目）→ 获取 cloudaicompanionProject，没有时用配置的项目
/// 2. retrieveUserQuota → 获取按模型分桶的配额数据；没有项目时 gemini-cli 不查，这里也不查
async fn query_gemini_quota(access_token: &str) -> Result<SubscriptionQuota, String> {
    let client = crate::http_client::get();

    let configured_project = configured_gemini_project();
    if let Some(project) = configured_project
        .as_deref()
        .filter(|project| project.bytes().all(|b| b.is_ascii_digit()))
    {
        return Ok(SubscriptionQuota::error(
            "gemini",
            CredentialStatus::Valid,
            format!(
                "Invalid Google Cloud Project ID: \"{project}\". GOOGLE_CLOUD_PROJECT (or \
                 GOOGLE_CLOUD_PROJECT_ID) must be the string Project ID (e.g. \"my-project-123\"), \
                 not the numeric Project Number."
            ),
        ));
    }
    let mut load_request = serde_json::json!({
        "metadata": {
            "ideType": "GEMINI_CLI",
            "pluginType": "GEMINI"
        }
    });
    if let Some(project) = &configured_project {
        load_request["cloudaicompanionProject"] = project.as_str().into();
        load_request["metadata"]["duetProject"] = project.as_str().into();
    }

    // ── Step 1: loadCodeAssist 获取项目 ID ──
    let load_resp = client
        .post("https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Content-Type", "application/json")
        .json(&load_request)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error (loadCodeAssist): {e}"))?;

    let load_status = load_resp.status();
    if load_status == reqwest::StatusCode::UNAUTHORIZED
        || load_status == reqwest::StatusCode::FORBIDDEN
    {
        return Ok(SubscriptionQuota::error(
            "gemini",
            CredentialStatus::Expired,
            format!("Authentication failed (HTTP {load_status}). Please re-login with Gemini CLI."),
        ));
    }
    let load_body: GeminiLoadCodeAssistResponse = match read_json(load_resp)
        .await
        .map_err(|e| format!("loadCodeAssist: {e}"))?
    {
        Ok(body) => body,
        Err(error) => {
            return Ok(SubscriptionQuota::error(
                "gemini",
                CredentialStatus::Valid,
                format!("loadCodeAssist: {error}"),
            ))
        }
    };

    let project_id = load_body
        .cloudaicompanion_project
        .as_ref()
        .and_then(extract_project_id)
        .or(configured_project);
    let Some(project_id) = project_id else {
        return Ok(SubscriptionQuota::error(
            "gemini",
            CredentialStatus::Valid,
            gemini_missing_project_message(&load_body),
        ));
    };

    // ── Step 2: retrieveUserQuota 获取配额 ──
    let quota_body = serde_json::json!({ "project": project_id });

    let quota_resp = client
        .post("https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Content-Type", "application/json")
        .json(&quota_body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error (retrieveUserQuota): {e}"))?;

    let quota_status = quota_resp.status();
    if quota_status == reqwest::StatusCode::UNAUTHORIZED
        || quota_status == reqwest::StatusCode::FORBIDDEN
    {
        return Ok(SubscriptionQuota::error(
            "gemini",
            CredentialStatus::Expired,
            format!("Authentication failed (HTTP {quota_status})."),
        ));
    }
    let quota_data: GeminiQuotaResponse = match read_json(quota_resp)
        .await
        .map_err(|e| format!("retrieveUserQuota: {e}"))?
    {
        Ok(body) => body,
        Err(error) => {
            return Ok(SubscriptionQuota::error(
                "gemini",
                CredentialStatus::Valid,
                format!("retrieveUserQuota: {error}"),
            ))
        }
    };

    // ── 按模型分类汇总，每类取最低 remainingFraction ──
    let mut category_map: HashMap<String, (f64, Option<String>)> = HashMap::new();

    if let Some(buckets) = quota_data.buckets {
        for bucket in buckets {
            let model_id = bucket.model_id.as_deref().unwrap_or("unknown");
            let category = classify_gemini_model(model_id).to_string();
            let remaining = bucket.remaining_fraction.unwrap_or(1.0).clamp(0.0, 1.0);

            let entry = category_map
                .entry(category)
                .or_insert((remaining, bucket.reset_time.clone()));
            if remaining < entry.0 {
                entry.0 = remaining;
                if bucket.reset_time.is_some() {
                    entry.1.clone_from(&bucket.reset_time);
                }
            }
        }
    }

    // 转换为 tiers（remainingFraction → utilization: 已用百分比）
    let sort_order = |name: &str| -> usize {
        match name {
            TIER_GEMINI_PRO => 0,
            TIER_GEMINI_FLASH => 1,
            TIER_GEMINI_FLASH_LITE => 2,
            _ => 3,
        }
    };

    let mut tiers: Vec<QuotaTier> = category_map
        .into_iter()
        .map(|(name, (remaining, reset_time))| QuotaTier {
            name,
            utilization: (1.0 - remaining) * 100.0,
            resets_at: reset_time,
            used_value_usd: None,
            max_value_usd: None,
        })
        .collect();

    tiers.sort_by_key(|t| sort_order(&t.name));

    Ok(SubscriptionQuota {
        tool: "gemini".to_string(),
        credential_status: CredentialStatus::Valid,
        credential_message: None,
        success: true,
        tiers,
        extra_usage: None,
        reset_credits: None,
        credits_balance: None,
        error: None,
        queried_at: Some(now_millis()),
    })
}

// ── 入口函数 ──────────────────────────────────────────────

/// 查询指定 CLI 工具的官方订阅额度
///
/// 瞬时传输失败以 `Err` 传播（前端 reject → retry + 保留上次成功值）。Expired
/// 分支的"过期也试一把"重试同样用 `?` 传播瞬时错误——不能折叠成"已过期"，
/// 否则一次网络抖动会被误报成确定性的凭据过期。
pub async fn get_subscription_quota(tool: &str) -> Result<SubscriptionQuota, String> {
    match tool {
        "claude" => {
            let (token, status, message) = read_claude_credentials();

            match status {
                CredentialStatus::NotFound => Ok(SubscriptionQuota::not_found("claude")),
                CredentialStatus::ParseError => Ok(SubscriptionQuota::error(
                    "claude",
                    CredentialStatus::ParseError,
                    message.unwrap_or_else(|| "Failed to parse credentials".to_string()),
                )),
                CredentialStatus::Expired | CredentialStatus::RefreshPending => {
                    // 即使过期也尝试调用 API（token 可能实际上仍有效）。只有接口也拒绝了
                    // 这个 token 才改报凭据过期；限流、5xx 说明 token 被接受了，原样返回，
                    // 前端才能按瞬时失败处理。
                    if let Some(token) = token {
                        let result = query_claude_quota(&token).await?;
                        if result.success
                            || !matches!(result.credential_status, CredentialStatus::Expired)
                        {
                            return Ok(result);
                        }
                    }
                    Ok(SubscriptionQuota::error(
                        "claude",
                        status,
                        message.unwrap_or_else(|| "OAuth token has expired".to_string()),
                    ))
                }
                CredentialStatus::Valid => {
                    let token = token.expect("token must be Some when status is Valid");
                    query_claude_quota(&token).await
                }
            }
        }
        "codex" => {
            let (token, account_id, status, message) = read_codex_credentials();

            match status {
                CredentialStatus::NotFound => Ok(SubscriptionQuota::not_found("codex")),
                CredentialStatus::ParseError => Ok(SubscriptionQuota::error(
                    "codex",
                    CredentialStatus::ParseError,
                    message.unwrap_or_else(|| "Failed to parse credentials".to_string()),
                )),
                CredentialStatus::Expired | CredentialStatus::RefreshPending => {
                    // 即使可能过期也尝试调用 API；只有接口也拒绝了 token 才改报凭据
                    // 状态，其余失败（限流、5xx）原样返回，同 Claude 分支。
                    if let Some(token) = token {
                        let result = query_codex_quota(
                            &token,
                            account_id.as_deref(),
                            "codex",
                            "Authentication failed. Please re-login with Codex CLI.",
                        )
                        .await?;
                        if result.success
                            || !matches!(result.credential_status, CredentialStatus::Expired)
                        {
                            return Ok(result);
                        }
                    }
                    Ok(SubscriptionQuota::error(
                        "codex",
                        status,
                        message.unwrap_or_else(|| "Codex OAuth token has expired".to_string()),
                    ))
                }
                CredentialStatus::Valid => {
                    let token = token.expect("token must be Some when status is Valid");
                    query_codex_quota(
                        &token,
                        account_id.as_deref(),
                        "codex",
                        "Authentication failed. Please re-login with Codex CLI.",
                    )
                    .await
                }
            }
        }
        "gemini" => {
            let (token, refresh_token, status, message) = read_gemini_credentials();

            match status {
                CredentialStatus::NotFound => Ok(SubscriptionQuota::not_found("gemini")),
                CredentialStatus::ParseError => Ok(SubscriptionQuota::error(
                    "gemini",
                    CredentialStatus::ParseError,
                    message.unwrap_or_else(|| "Failed to parse credentials".to_string()),
                )),
                CredentialStatus::Expired | CredentialStatus::RefreshPending => {
                    // Gemini access_token 仅 ~1h 有效，尝试用 refresh_token 刷新
                    if let Some(ref rt) = refresh_token {
                        if let Some(new_token) = refresh_gemini_token(rt).await {
                            return query_gemini_quota(&new_token).await;
                        }
                    }
                    // 刷新失败，尝试用旧 token
                    if let Some(ref token) = token {
                        let result = query_gemini_quota(token).await?;
                        if result.success {
                            return Ok(result);
                        }
                    }
                    Ok(SubscriptionQuota::error(
                        "gemini",
                        CredentialStatus::Expired,
                        message.unwrap_or_else(|| "Gemini OAuth token has expired".to_string()),
                    ))
                }
                CredentialStatus::Valid => {
                    let token = token.expect("token must be Some when status is Valid");
                    query_gemini_quota(&token).await
                }
            }
        }
        "grokbuild" => crate::services::subscription_grok::get_grok_subscription_quota().await,
        _ => Ok(SubscriptionQuota::not_found(tool)),
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

    #[test]
    fn claude_keychain_service_follows_config_dir_hash() {
        // 期望值按 Claude Code 的 sha256(dir).hex[..8] 用 Python 独立算出
        let default = Path::new("/Users/x/.claude");
        assert_eq!(
            claude_keychain_services(None, default),
            vec![
                "Claude Code-credentials",
                "Claude Code-credentials-c72cc1ce"
            ]
        );
        assert_eq!(
            claude_keychain_services(Some(Path::new("/Users/x/claude-work")), default),
            vec![
                "Claude Code-credentials-8e8c5344",
                "Claude Code-credentials-8ac017c5"
            ]
        );
        assert_eq!(
            claude_keychain_services(Some(default), default),
            vec![
                "Claude Code-credentials-c72cc1ce",
                "Claude Code-credentials-95d5ea82",
                "Claude Code-credentials"
            ]
        );
    }

    #[test]
    fn codex_reset_credits_count_only_unexpired_available() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-04T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let raw = br#"{
            "available_count": 9,
            "credits": [
                {"id":"a","reset_type":"weekly","status":"available","granted_at":"2026-09-20T00:00:00Z","expires_at":"2026-10-20T00:00:00Z"},
                {"id":"b","reset_type":"weekly","status":"available","granted_at":"2026-09-01T00:00:00Z","expires_at":"2026-10-03T00:00:00Z"},
                {"id":"c","reset_type":"weekly","status":"redeemed","granted_at":"2026-09-01T00:00:00Z","expires_at":"2026-10-30T00:00:00Z"},
                {"id":"d","reset_type":"weekly","status":"available","granted_at":"2026-09-01T00:00:00Z","expires_at":null},
                {"id":"e","reset_type":"weekly","status":"available","granted_at":"2026-09-25T00:00:00.123Z","expires_at":"2026-10-08T12:00:00.123Z"}
            ]
        }"#;
        let credits = parse_codex_reset_credits(raw, now).unwrap();
        // b 已过期、c 已用掉；剩下按到期先后，不过期的排最后
        assert_eq!(credits.expires_at.len(), 3);
        assert!(credits.expires_at[0]
            .as_deref()
            .unwrap()
            .starts_with("2026-10-08T12:00:00.123"));
        assert!(credits.expires_at[1]
            .as_deref()
            .unwrap()
            .starts_with("2026-10-20"));
        assert_eq!(credits.expires_at[2], None);
    }

    #[test]
    fn codex_reset_credits_empty_and_malformed() {
        let now = chrono::Utc::now();
        let empty = parse_codex_reset_credits(br#"{"available_count":0,"credits":[]}"#, now);
        assert_eq!(empty.unwrap().expires_at.len(), 0);
        assert!(parse_codex_reset_credits(b"<html>", now).is_none());
        // 新序列化的字段对旧缓存是可选的
        let quota: SubscriptionQuota = serde_json::from_str(
            r#"{"tool":"codex","credentialStatus":"valid","credentialMessage":null,
                "success":true,"tiers":[],"extraUsage":null,"error":null,"queriedAt":1}"#,
        )
        .unwrap();
        assert!(quota.reset_credits.is_none());
        assert!(quota.credits_balance.is_none());
    }

    #[test]
    fn codex_credits_balance_from_usage_response() {
        let parse = |raw: &str| {
            let body: CodexUsageResponse = serde_json::from_str(raw).unwrap();
            body.credits.as_ref().and_then(parse_codex_credits_balance)
        };
        // 实测形状：balance 是字符串
        assert_eq!(
            parse(
                r#"{"credits":{"has_credits":true,"unlimited":false,"overage_limit_reached":false,
                    "balance":"62500","approx_local_messages":[15625,81250]}}"#
            ),
            Some(62500.0)
        );
        assert_eq!(
            parse(r#"{"credits":{"has_credits":true,"unlimited":false,"balance":42.5}}"#),
            Some(42.5)
        );
        // 0、不限量、没有、缺字段、形状不对：都不显示，也不让额度解析失败
        assert_eq!(
            parse(r#"{"credits":{"has_credits":true,"unlimited":false,"balance":"0"}}"#),
            None
        );
        assert_eq!(
            parse(r#"{"credits":{"has_credits":true,"unlimited":true,"balance":"10"}}"#),
            None
        );
        assert_eq!(
            parse(r#"{"credits":{"has_credits":false,"unlimited":false,"balance":"10"}}"#),
            None
        );
        assert_eq!(parse(r#"{"credits":{"has_credits":true}}"#), None);
        assert_eq!(parse(r#"{"credits":"weird","rate_limit":null}"#), None);
        assert_eq!(parse(r#"{}"#), None);
    }

    /// 和 codex-rs `compute_store_key` 同一算法：路径不存在时按原样算，存在时先规范化
    /// （符号链接和它指向的目录是同一个账户）。
    #[cfg(target_os = "macos")]
    #[test]
    fn codex_keychain_account_matches_codex() {
        assert_eq!(
            codex_keychain_account(std::path::Path::new("/nonexistent/codex-home")),
            "cli|b5d85b424b15c4d9"
        );

        let dir = tempfile::TempDir::new().unwrap();
        let real = dir.path().join("codex-home");
        std::fs::create_dir(&real).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert_eq!(codex_keychain_account(&link), codex_keychain_account(&real));
    }

    #[test]
    fn claude_expired_access_token_with_live_refresh_token_is_refresh_pending() {
        let past = now_millis() - 60_000;
        let future = now_millis() + 3_600_000;
        let status = |entry: serde_json::Value| {
            parse_claude_credentials_json(
                &serde_json::json!({ "claudeAiOauth": entry }).to_string(),
            )
            .1
        };

        assert!(matches!(
            status(serde_json::json!({
                "accessToken": "a", "expiresAt": past,
                "refreshToken": "r", "refreshTokenExpiresAt": future
            })),
            CredentialStatus::RefreshPending
        ));
        // 旧版凭据没有刷新令牌的过期时间：有刷新令牌就按能刷新算。
        assert!(matches!(
            status(serde_json::json!({
                "accessToken": "a", "expiresAt": past, "refreshToken": "r"
            })),
            CredentialStatus::RefreshPending
        ));
        // 刷新令牌也过期了 / 根本没有：真的要重新登录。
        assert!(matches!(
            status(serde_json::json!({
                "accessToken": "a", "expiresAt": past,
                "refreshToken": "r", "refreshTokenExpiresAt": past
            })),
            CredentialStatus::Expired
        ));
        assert!(matches!(
            status(serde_json::json!({ "accessToken": "a", "expiresAt": past })),
            CredentialStatus::Expired
        ));
        assert!(matches!(
            status(serde_json::json!({
                "accessToken": "a", "expiresAt": future, "refreshToken": "r"
            })),
            CredentialStatus::Valid
        ));
    }

    fn scoped_limit(model: &str, percent: f64) -> serde_json::Value {
        serde_json::json!({
            "kind": "weekly_scoped",
            "group": "weekly",
            "percent": percent,
            "resets_at": "2026-09-12T00:00:00Z",
            "is_active": true,
            "scope": { "model": { "id": null, "display_name": model }, "surface": null }
        })
    }

    fn codex_auth_json(exp: Option<i64>, refresh_token: &str, last_refresh: &str) -> String {
        use base64::Engine;
        let access_token = match exp {
            Some(exp) => {
                let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(serde_json::json!({ "exp": exp }).to_string());
                format!("e30.{payload}.sig")
            }
            None => "opaque-token".to_string(),
        };
        serde_json::json!({
            "auth_mode": "chatgpt",
            "last_refresh": last_refresh,
            "tokens": {
                "access_token": access_token,
                "account_id": "acct",
                "refresh_token": refresh_token
            }
        })
        .to_string()
    }

    #[test]
    fn codex_token_status_follows_jwt_exp_first() {
        let now = now_millis() / 1000;
        let long_ago = "2020-01-01T00:00:00Z";

        // exp 在将来：last_refresh 再旧也有效
        let (_, _, status, _) =
            parse_codex_credentials_json(&codex_auth_json(Some(now + 3600), "rt", long_ago));
        assert!(matches!(status, CredentialStatus::Valid));

        // exp 已过：有刷新令牌是 RefreshPending，没有是 Expired
        let (_, _, status, _) =
            parse_codex_credentials_json(&codex_auth_json(Some(now - 60), "rt", long_ago));
        assert!(matches!(status, CredentialStatus::RefreshPending));
        let (_, _, status, _) =
            parse_codex_credentials_json(&codex_auth_json(Some(now - 60), "", long_ago));
        assert!(matches!(status, CredentialStatus::Expired));

        // 读不出 exp 才看 last_refresh
        let (_, _, status, _) =
            parse_codex_credentials_json(&codex_auth_json(None, "rt", long_ago));
        assert!(matches!(status, CredentialStatus::RefreshPending));
        let recent = chrono::Utc::now().to_rfc3339();
        let (_, _, status, _) = parse_codex_credentials_json(&codex_auth_json(None, "rt", &recent));
        assert!(matches!(status, CredentialStatus::Valid));
    }

    #[test]
    fn claude_http_errors_follow_claude_code_classification() {
        use reqwest::StatusCode;
        let anthropic_body =
            br#"{"type":"error","error":{"type":"permission_error","message":"nope"}}"#;

        let q = claude_http_error(StatusCode::UNAUTHORIZED, None, b"");
        assert!(matches!(q.credential_status, CredentialStatus::Expired));
        let q = claude_http_error(StatusCode::FORBIDDEN, None, anthropic_body);
        assert!(matches!(q.credential_status, CredentialStatus::Expired));

        // 不带 Anthropic 错误体的 403 和 429 都是限流
        let q = claude_http_error(StatusCode::FORBIDDEN, None, b"<html>blocked</html>");
        assert!(matches!(q.credential_status, CredentialStatus::Valid));
        assert!(q.error.unwrap().starts_with("Rate limited (HTTP 403"));
        let q = claude_http_error(StatusCode::TOO_MANY_REQUESTS, Some("30"), b"");
        assert_eq!(
            q.error.as_deref(),
            Some("Rate limited (HTTP 429 Too Many Requests, retry after 30s)")
        );

        let q = claude_http_error(StatusCode::BAD_GATEWAY, None, b"oops");
        assert_eq!(
            q.error.as_deref(),
            Some("API error (HTTP 502 Bad Gateway): oops")
        );
    }

    #[test]
    fn claude_quota_without_any_usage_key_is_an_error() {
        let quota = parse_claude_quota(&serde_json::json!({ "error": "in-band" }));
        assert!(!quota.success);
        assert_eq!(quota.error.as_deref(), Some("Unrecognized usage response"));
    }

    #[test]
    fn claude_quota_preserves_legacy_windows_and_extra_usage() {
        let quota = parse_claude_quota(&serde_json::json!({
            "five_hour": { "utilization": 12.0, "resets_at": "2026-09-09T15:00:00Z" },
            "seven_day": { "utilization": 25.0, "resets_at": null },
            "seven_day_opus": { "utilization": 8.0 },
            "seven_day_sonnet": null,
            "other_window": { "utilization": 4.0 },
            "extra_usage": { "is_enabled": true, "monthly_limit": 100.0,
                "used_credits": 9.0, "utilization": 9.0, "currency": "USD" }
        }));
        assert!(quota.success);
        assert_eq!(quota.tool, "claude");
        assert_eq!(
            quota
                .tiers
                .iter()
                .map(|t| (t.name.as_str(), t.utilization))
                .collect::<Vec<_>>(),
            vec![
                (TIER_FIVE_HOUR, 12.0),
                (TIER_SEVEN_DAY, 25.0),
                (TIER_SEVEN_DAY_OPUS, 8.0),
                ("other_window", 4.0)
            ]
        );
        assert_eq!(
            quota.tiers[0].resets_at.as_deref(),
            Some("2026-09-09T15:00:00Z")
        );
        let extra = quota.extra_usage.unwrap();
        assert!(extra.is_enabled);
        assert_eq!(extra.used_credits, Some(9.0));
        assert_eq!(extra.monthly_limit, Some(100.0));
        assert_eq!(extra.currency.as_deref(), Some("USD"));
    }

    #[test]
    fn claude_quota_adds_fable_from_limits_array() {
        let quota = parse_claude_quota(&serde_json::json!({
            "five_hour": { "utilization": 12.0 },
            "seven_day": { "utilization": 25.0 },
            "seven_day_opus": null,
            "seven_day_sonnet": null,
            "limits": [scoped_limit("Fable", 37.5)]
        }));
        assert_eq!(quota.tiers.len(), 3);
        let tier = &quota.tiers[2];
        assert_eq!(tier.name, TIER_SEVEN_DAY_FABLE);
        assert_eq!(tier.utilization, 37.5);
        assert_eq!(tier.resets_at.as_deref(), Some("2026-09-12T00:00:00Z"));
        // 前端与缓存使用同一份 camelCase 数据，无需额外字段。
        let serialized = serde_json::to_value(&quota).unwrap();
        assert_eq!(serialized["tiers"][2]["resetsAt"], "2026-09-12T00:00:00Z");
    }

    #[test]
    fn claude_quota_scoped_windows_override_legacy_and_deduplicate() {
        let mut fable = scoped_limit("  fAbLe  ", 0.0);
        fable["is_active"] = serde_json::json!(false);
        fable["resets_at"] = serde_json::Value::Null;
        let quota = parse_claude_quota(&serde_json::json!({
            "seven_day_fable": { "utilization": 80.0, "resets_at": "2026-09-11T00:00:00Z" },
            "seven_day_opus": { "utilization": 20.0 },
            "seven_day_sonnet": { "utilization": 30.0 },
            "limits": [scoped_limit("Sonnet", 5.0), fable, scoped_limit("Fable", 90.0), scoped_limit("Opus", 6.0)]
        }));
        assert_eq!(
            quota
                .tiers
                .iter()
                .map(|t| (t.name.as_str(), t.utilization))
                .collect::<Vec<_>>(),
            vec![
                (TIER_SEVEN_DAY_FABLE, 0.0),
                (TIER_SEVEN_DAY_OPUS, 6.0),
                (TIER_SEVEN_DAY_SONNET, 5.0)
            ]
        );
        assert_eq!(quota.tiers[0].resets_at, None);
    }

    #[test]
    fn claude_quota_skips_invalid_or_unrelated_scoped_rows() {
        let valid = scoped_limit("Fable", 37.0);
        let mut invalid = vec![serde_json::Value::Null, serde_json::json!("invalid")];
        for (pointer, value) in [
            ("/kind", serde_json::json!("spend")),
            ("/group", serde_json::json!("daily")),
            ("/percent", serde_json::Value::Null),
            ("/percent", serde_json::json!("37")),
            ("/percent", serde_json::json!(-1)),
            ("/resets_at", serde_json::json!(123)),
            ("/scope/model/display_name", serde_json::Value::Null),
            ("/scope/model/display_name", serde_json::json!("Unknown")),
            ("/scope/surface", serde_json::json!("claude_code")),
        ] {
            let mut row = valid.clone();
            *row.pointer_mut(pointer).unwrap() = value;
            invalid.push(row);
        }
        let mut body = serde_json::json!({
            "five_hour": { "utilization": 12.0 },
            "seven_day_fable": { "utilization": 8.0 },
            "limits": invalid
        });
        let fallback = parse_claude_quota(&body);
        assert_eq!(fallback.tiers.len(), 2);
        assert_eq!(fallback.tiers[1].utilization, 8.0);
        body["limits"].as_array_mut().unwrap().push(valid);
        let quota = parse_claude_quota(&body);
        assert_eq!(quota.tiers.len(), 2);
        assert_eq!(quota.tiers[0].utilization, 12.0);
        assert_eq!(quota.tiers[1].utilization, 37.0);
    }

    #[test]
    fn claude_quota_does_not_invent_missing_fable_usage() {
        for limits in [
            serde_json::Value::Null,
            serde_json::json!([]),
            serde_json::json!({}),
        ] {
            let quota = parse_claude_quota(&serde_json::json!({
                "five_hour": { "utilization": 12.0 },
                "limits": limits
            }));
            assert_eq!(quota.tiers.len(), 1);
            assert_eq!(quota.tiers[0].name, TIER_FIVE_HOUR);
        }
        let quota =
            parse_claude_quota(&serde_json::json!({ "limits": [scoped_limit("Fable", 100.0)] }));
        assert_eq!(quota.tiers.len(), 1);
        assert_eq!(quota.tiers[0].name, TIER_SEVEN_DAY_FABLE);
        assert_eq!(quota.tiers[0].utilization, 100.0);
    }

    #[test]
    fn window_seconds_map_to_expected_tier_names() {
        // 官方特例窗口
        assert_eq!(window_seconds_to_tier_name(18000), TIER_FIVE_HOUR);
        assert_eq!(window_seconds_to_tier_name(604800), TIER_SEVEN_DAY);
        // Codex 免费方案的次要窗口是 30 天（30 * 24 * 3600 = 2_592_000 秒）。
        // 前端 TIER_I18N_KEYS 与 tray 月分组都需要认得 "30_day"，见 #3651。
        assert_eq!(window_seconds_to_tier_name(2_592_000), TIER_THIRTY_DAY);
        // 其他窗口按小时/天回退命名
        assert_eq!(window_seconds_to_tier_name(3600), "1_hour");
        assert_eq!(window_seconds_to_tier_name(86400), "1_day");
    }

    #[test]
    fn gemini_project_follows_gemini_cli_lookup_order() {
        let env_file = "# project\nexport GOOGLE_CLOUD_PROJECT_ID='from-file-id'\nGOOGLE_CLOUD_PROJECT=\"from-file\" # main\n";
        let no_env = |_: &str| None;
        assert_eq!(
            pick_gemini_project(no_env, env_file).as_deref(),
            Some("from-file")
        );
        // 进程环境优先于 .env；GOOGLE_CLOUD_PROJECT 优先于 _ID，即使后者来自进程环境
        let env = |key: &str| (key == "GOOGLE_CLOUD_PROJECT_ID").then(|| "env-id".to_string());
        assert_eq!(
            pick_gemini_project(env, env_file).as_deref(),
            Some("from-file")
        );
        assert_eq!(pick_gemini_project(env, "").as_deref(), Some("env-id"));
        // 空值视为没有
        assert_eq!(
            pick_gemini_project(
                no_env,
                "GOOGLE_CLOUD_PROJECT=\nGOOGLE_CLOUD_PROJECT_ID=p-2 # x"
            )
            .as_deref(),
            Some("p-2")
        );
        assert_eq!(pick_gemini_project(no_env, "OTHER=1"), None);
    }

    #[test]
    fn gemini_missing_project_message_matches_gemini_cli_errors() {
        let load = |body: serde_json::Value| -> GeminiLoadCodeAssistResponse {
            serde_json::from_value(body).unwrap()
        };
        assert!(gemini_missing_project_message(&load(serde_json::json!({})))
            .contains("Run gemini once"));
        assert_eq!(
            gemini_missing_project_message(&load(serde_json::json!({
                "currentTier": {"id": "standard-tier"},
                "ineligibleTiers": [{"reasonMessage": "Not eligible in your region"}]
            }))),
            "Not eligible in your region"
        );
        assert!(gemini_missing_project_message(&load(serde_json::json!({
            "currentTier": {"id": "standard-tier"}
        })))
        .starts_with("This account requires setting GOOGLE_CLOUD_PROJECT"));
        assert_eq!(extract_project_id(&serde_json::json!("")), None);
        assert_eq!(
            extract_project_id(&serde_json::json!({"id": "managed-123"})).as_deref(),
            Some("managed-123")
        );
    }
}
