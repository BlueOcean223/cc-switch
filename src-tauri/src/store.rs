use crate::codex_oauth_auth::CodexOAuthManager;
use crate::database::Database;
use crate::mode::SwitchLocks;
use crate::services::UsageCache;
use std::sync::Arc;

/// 全局应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    /// 按应用的切换锁（见 `mode::lock_settled_blocking`）。
    pub switch_locks: SwitchLocks,
    pub usage_cache: Arc<UsageCache>,
    // 内部已使用细粒度锁（accounts/access_tokens/refresh_locks），所有方法均为
    // `&self`，无需外层 RwLock；避免持有粗粒度锁跨网络刷新导致的连锁阻塞。
    pub codex_oauth_manager: Arc<CodexOAuthManager>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(db: Arc<Database>) -> Self {
        let codex_oauth_manager =
            Arc::new(CodexOAuthManager::new(crate::config::get_app_config_dir()));

        Self {
            db,
            switch_locks: SwitchLocks::new(),
            usage_cache: Arc::new(UsageCache::new()),
            codex_oauth_manager,
        }
    }
}
