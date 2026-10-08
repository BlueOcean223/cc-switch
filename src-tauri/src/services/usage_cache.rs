//! 托盘展示用的用量缓存（进程内、写穿式）。
//!
//! 各 usage 查询命令成功时写入；系统托盘构建菜单时读取。不持久化，
//! 进程重启即空，由下一次自动查询或托盘悬停触发的刷新重新填充。
//!
//! 瞬时失败（见 [`is_transient_usage_error`]）不覆盖已缓存的成功读数，`put_*`
//! 返回 `false`，调用方不通知托盘和前端。

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{LazyLock, RwLock};

use crate::app_config::AppType;
use crate::provider::UsageResult;
use crate::services::subscription::SubscriptionQuota;

/// 失败结果是否是瞬时的（限流、网络、5xx），规则与前端
/// `src/lib/query/queries.ts` 的 `isTransientUsageError` 相同，改一边要同步另一边。
pub fn is_transient_usage_error(success: bool, error: Option<&str>) -> bool {
    static HTTP_STATUS: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"http\s+(\d{3})").expect("valid regex"));
    if success {
        return false;
    }
    let error = error.unwrap_or_default().to_lowercase();
    if error.is_empty() {
        return false;
    }
    const NETWORK: [&str; 5] = [
        "network error",
        "request failed",
        "请求失败",
        "failed to read response",
        "读取响应失败",
    ];
    if NETWORK.iter().any(|text| error.contains(text)) || error.contains("rate limited") {
        return true;
    }
    HTTP_STATUS
        .captures(&error)
        .and_then(|captures| captures[1].parse::<u16>().ok())
        .is_some_and(|status| (500..=599).contains(&status) || status == 429 || status == 408)
}

trait UsageOutcome {
    fn succeeded(&self) -> bool;
    fn error(&self) -> Option<&str>;
}

impl UsageOutcome for SubscriptionQuota {
    fn succeeded(&self) -> bool {
        self.success
    }
    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl UsageOutcome for UsageResult {
    fn succeeded(&self) -> bool {
        self.success
    }
    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// 写入一条结果；新结果是瞬时失败且已缓存的是成功读数时保留旧值，返回 `false`。
fn put_unless_transient<K: Eq + Hash, V: UsageOutcome>(
    map: &RwLock<HashMap<K, V>>,
    key: K,
    value: V,
) -> bool {
    let Ok(mut w) = map.write() else {
        return false;
    };
    let keep_cached = is_transient_usage_error(value.succeeded(), value.error())
        && w.get(&key).is_some_and(UsageOutcome::succeeded);
    if keep_cached {
        return false;
    }
    w.insert(key, value);
    true
}

#[derive(Default)]
pub struct UsageCache {
    subscription: RwLock<HashMap<AppType, SubscriptionQuota>>,
    codex_oauth: RwLock<HashMap<String, SubscriptionQuota>>,
    script: RwLock<HashMap<(AppType, String), UsageResult>>,
}

impl UsageCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回是否写入（瞬时失败不覆盖成功读数，见模块说明）。
    pub fn put_subscription(&self, app_type: AppType, quota: SubscriptionQuota) -> bool {
        put_unless_transient(&self.subscription, app_type, quota)
    }

    /// 返回是否写入（瞬时失败不覆盖成功读数，见模块说明）。
    pub fn put_codex_oauth(&self, account_id: String, quota: SubscriptionQuota) -> bool {
        put_unless_transient(&self.codex_oauth, account_id, quota)
    }

    /// Managed accounts must never share the CLI's app-wide subscription snapshot.
    pub fn with_codex_oauth<R>(
        &self,
        account_id: &str,
        f: impl FnOnce(&SubscriptionQuota) -> R,
    ) -> Option<R> {
        self.codex_oauth
            .read()
            .ok()
            .and_then(|r| r.get(account_id).map(f))
    }

    /// 返回是否写入（瞬时失败不覆盖成功读数，见模块说明）。
    pub fn put_script(&self, app_type: AppType, provider_id: String, result: UsageResult) -> bool {
        put_unless_transient(&self.script, (app_type, provider_id), result)
    }

    /// 以借用形式暴露订阅快照，避免托盘每次重建时深拷贝整个 `SubscriptionQuota`。
    pub fn with_subscription<R>(
        &self,
        app_type: &AppType,
        f: impl FnOnce(&SubscriptionQuota) -> R,
    ) -> Option<R> {
        self.subscription
            .read()
            .ok()
            .and_then(|r| r.get(app_type).map(f))
    }

    /// 以借用形式暴露脚本型用量结果，同上。
    pub fn with_script<R>(
        &self,
        app_type: &AppType,
        provider_id: &str,
        f: impl FnOnce(&UsageResult) -> R,
    ) -> Option<R> {
        self.script
            .read()
            .ok()
            .and_then(|r| r.get(&(app_type.clone(), provider_id.to_string())).map(f))
    }

    pub fn invalidate_script(&self, app_type: &AppType, provider_id: &str) {
        // 热路径会对每个禁用脚本的 provider 在托盘重建时调用一次：先走读锁
        // `contains_key` 快速放行"本来就不在缓存里"的常见情况，避免无谓的写锁升级。
        let key = (app_type.clone(), provider_id.to_string());
        if !self.script.read().is_ok_and(|r| r.contains_key(&key)) {
            return;
        }
        if let Ok(mut w) = self.script.write() {
            w.remove(&key);
        }
    }

    pub fn invalidate_subscription(&self, app_type: &AppType) {
        if !self
            .subscription
            .read()
            .is_ok_and(|r| r.contains_key(app_type))
        {
            return;
        }
        if let Ok(mut w) = self.subscription.write() {
            w.remove(app_type);
        }
    }

    /// Drop all process-local usage snapshots after a database/provider restore.
    pub fn invalidate_all(&self) {
        if let Ok(mut subscriptions) = self.subscription.write() {
            subscriptions.clear();
        }
        if let Ok(mut scripts) = self.script.write() {
            scripts.clear();
        }
        if let Ok(mut accounts) = self.codex_oauth.write() {
            accounts.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::subscription::CredentialStatus;

    fn fake_quota() -> SubscriptionQuota {
        SubscriptionQuota {
            tool: "claude".to_string(),
            credential_status: CredentialStatus::Valid,
            credential_message: None,
            success: true,
            tiers: vec![],
            extra_usage: None,
            reset_credits: None,
            credits_balance: None,
            error: None,
            queried_at: Some(0),
        }
    }

    fn fake_result() -> UsageResult {
        UsageResult {
            success: true,
            data: None,
            error: None,
        }
    }

    fn failed_quota(error: &str) -> SubscriptionQuota {
        SubscriptionQuota::error("claude", CredentialStatus::Valid, error.to_string())
    }

    #[test]
    fn transient_errors_match_the_frontend_rules() {
        for error in [
            "Rate limited (HTTP 429)",
            "Rate limited (HTTP 403 Forbidden)",
            "Network error: timed out",
            "API error (HTTP 502 Bad Gateway): oops",
            "HTTP 503 Service Unavailable",
            "请求失败: timeout",
            "Transient HTTP failure (HTTP 408 Request Timeout)",
        ] {
            assert!(is_transient_usage_error(false, Some(error)), "{error}");
        }
        for error in [
            "Authentication failed (HTTP 401 Unauthorized)",
            "API error (HTTP 404 Not Found): x",
            "Unrecognized Kimi usage response",
            "",
        ] {
            assert!(!is_transient_usage_error(false, Some(error)), "{error}");
        }
        assert!(!is_transient_usage_error(false, None));
        assert!(!is_transient_usage_error(true, Some("Rate limited")));
    }

    #[test]
    fn transient_failure_keeps_the_cached_reading() {
        let cache = UsageCache::new();
        let success = |q: &SubscriptionQuota| q.success;
        // 没有成功读数时照常写入
        assert!(cache.put_subscription(AppType::Claude, failed_quota("Rate limited (HTTP 429)")));
        assert!(cache.put_subscription(AppType::Claude, fake_quota()));
        assert!(!cache.put_subscription(AppType::Claude, failed_quota("Rate limited (HTTP 429)")));
        assert_eq!(
            cache.with_subscription(&AppType::Claude, success),
            Some(true)
        );

        // 确定性失败照常覆盖
        assert!(cache.put_subscription(
            AppType::Claude,
            failed_quota("Authentication failed (HTTP 401 Unauthorized)")
        ));
        assert_eq!(
            cache.with_subscription(&AppType::Claude, success),
            Some(false)
        );

        cache.put_script(AppType::Codex, "p".to_string(), fake_result());
        let rate_limited = UsageResult {
            success: false,
            data: None,
            error: Some("HTTP 429 Too Many Requests".to_string()),
        };
        assert!(!cache.put_script(AppType::Codex, "p".to_string(), rate_limited));
        assert_eq!(
            cache.with_script(&AppType::Codex, "p", |r| r.success),
            Some(true)
        );
    }

    #[test]
    fn subscription_round_trip() {
        let cache = UsageCache::new();
        assert!(cache
            .with_subscription(&AppType::Claude, |q| q.success)
            .is_none());
        cache.put_subscription(AppType::Claude, fake_quota());
        let got = cache
            .with_subscription(&AppType::Claude, |q| q.success)
            .unwrap();
        assert!(got);
        assert!(cache
            .with_subscription(&AppType::Codex, |q| q.success)
            .is_none());
    }

    #[test]
    fn script_round_trip_and_invalidate() {
        let cache = UsageCache::new();
        assert!(cache
            .with_script(&AppType::Codex, "pid", |r| r.success)
            .is_none());
        cache.put_script(AppType::Codex, "pid".to_string(), fake_result());
        assert!(cache
            .with_script(&AppType::Codex, "pid", |r| r.success)
            .is_some());
        cache.invalidate_script(&AppType::Codex, "pid");
        assert!(cache
            .with_script(&AppType::Codex, "pid", |r| r.success)
            .is_none());
    }

    #[test]
    fn script_keys_isolated_by_app_type() {
        let cache = UsageCache::new();
        cache.put_script(AppType::Claude, "same".to_string(), fake_result());
        assert!(cache
            .with_script(&AppType::Claude, "same", |r| r.success)
            .is_some());
        assert!(cache
            .with_script(&AppType::Codex, "same", |r| r.success)
            .is_none());
    }

    #[test]
    fn invalidate_all_clears_all_usage_snapshots() {
        let cache = UsageCache::new();
        cache.put_subscription(AppType::Claude, fake_quota());
        cache.put_codex_oauth("account-1".to_string(), fake_quota());
        cache.put_script(AppType::Codex, "provider".to_string(), fake_result());

        cache.invalidate_all();

        assert!(cache
            .with_subscription(&AppType::Claude, |quota| quota.success)
            .is_none());
        assert!(cache
            .with_script(&AppType::Codex, "provider", |usage| usage.success)
            .is_none());
        assert!(cache
            .with_codex_oauth("account-1", |quota| quota.success)
            .is_none());
    }
}
