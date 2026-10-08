//! 按应用的切换锁。
//!
//! 切换供应商、保存当前供应商、同步 live 都会改客户端文件和指针。同一应用的这些操作
//! 串行执行，不同应用之间（如 Claude 和 Codex）可以并行。写引擎的应用写锁在更里面拿，
//! 两把锁不反向嵌套。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard, RwLock};

use crate::app_config::AppType;
use crate::error::AppError;
use crate::store::AppState;

use super::operation;

/// 每个应用一把互斥锁。
#[derive(Clone, Default)]
pub struct SwitchLocks {
    locks: Arc<RwLock<HashMap<String, Arc<Mutex<()>>>>>,
}

impl SwitchLocks {
    pub fn new() -> Self {
        Self::default()
    }

    /// 拿这个应用的切换锁。持有期间同一应用的其他操作排队等待。
    pub async fn lock_for_app(&self, app_type: &str) -> OwnedMutexGuard<()> {
        let lock = {
            let locks = self.locks.read().await;
            if let Some(lock) = locks.get(app_type) {
                lock.clone()
            } else {
                drop(locks);
                let mut locks = self.locks.write().await;
                locks
                    .entry(app_type.to_string())
                    .or_insert_with(|| Arc::new(Mutex::new(())))
                    .clone()
            }
        };
        lock.lock_owned().await
    }
}

/// 拿这个应用的切换锁，再补完它上一次没做完的写入。之后读到的指针都是落定过的。写入
/// 函数在写锁里发现还有没补完的操作会补完后拒绝这次写入（见
/// `operation::recover_before_write`），入口先补完，用户就不用重试一次。
///
/// 补不完（比如本机设置文件写不进去、改不了指针）就拒绝这次操作：这时读到的还是补完前的
/// 指针，照着做下去（比如只存了一行、以为它不是当前供应商），等那次操作补完就和刚做的
/// 对不上了。
///
/// 累加式应用不经写引擎（`live-state.json` 里没有它们要补完的操作），不拿锁。
///
/// 切换锁是 tokio 的锁（异步命令里也要拿），这里用 `block_on` 等它；tokio 的
/// `blocking_lock` 在异步上下文里会 panic，而调用方不全在 `spawn_blocking` 里。
pub(crate) fn lock_settled_blocking(
    state: &AppState,
    app: &AppType,
) -> Result<Option<OwnedMutexGuard<()>>, AppError> {
    if app.is_additive_mode() {
        return Ok(None);
    }
    let guard = futures::executor::block_on(state.switch_locks.lock_for_app(app.as_str()));
    operation::settle(&state.db, app.as_str()).map_err(|error| {
        AppError::localized(
            "mode.unsettled",
            format!(
                "{} 上一次写配置文件的操作没做完，现在也补不完：{error}。本次什么都没做，请排查后重试",
                app.as_str()
            ),
            format!(
                "The previous write to {}'s config files is unfinished and cannot be completed now: {error}. Nothing was done; fix the cause and retry",
                app.as_str()
            ),
        )
    })?;
    Ok(Some(guard))
}
