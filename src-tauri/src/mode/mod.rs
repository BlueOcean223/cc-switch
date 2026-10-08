//! 客户端文件的设备本地状态（`live-state.json`）、按应用的切换锁，以及多文件操作的写前
//! 意图和崩溃恢复。

#[cfg(test)]
mod direct_tests;
mod lock;
pub mod operation;
pub mod state;

pub(crate) use lock::{lock_settled_blocking, SwitchLocks};
