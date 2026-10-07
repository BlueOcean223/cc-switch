//! 按会话日志重建用量。
//!
//! 用量明细是从各 CLI 的会话日志导入的，导入逻辑或计价规则改了以后，已入库的
//! 数字不会自己变。重建会清掉会话导入的明细和全部导入游标，从头重新导入，再把
//! 30 天前的明细重新汇总。日志里已经没有的日期，原有的按天汇总原样保留。
//!
//! 平时的增量同步不导入 30 天前的事件（见 [`is_below_import_floor`]）：那些日期的
//! 明细已经汇总后删除，再导一遍会在下次汇总时把同一批请求加进同一天第二次。

use crate::database::dao::usage_rollup::{
    compute_local_midnight_cutoff, USAGE_DETAIL_RETENTION_DAYS,
};
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::session_usage::{sync_all_unlocked, SessionSyncResult};
use rusqlite::OptionalExtension;
use std::sync::atomic::{AtomicBool, Ordering};

/// `settings` 表里的标记：下次会话同步改为整体重建。数据库迁移在导入或计价
/// 规则变化时写入。
pub(crate) const USAGE_REBUILD_PENDING_KEY: &str = "usage_rebuild_pending";

/// 会话导入写入的 `data_source`。重建只清这些行，旧版本地路由记录的行不动。
const SESSION_DATA_SOURCES: &[&str] = &[
    "session_log",
    "codex_session",
    "gemini_session",
    "grok_session",
    "opencode_session",
    "pi_session",
    "mcode_session",
];

/// 重建期间导入不设下限：30 天前的事件也要导入，之后再重新汇总。
static REBUILDING: AtomicBool = AtomicBool::new(false);

struct RebuildingGuard;

impl RebuildingGuard {
    fn enter() -> Self {
        REBUILDING.store(true, Ordering::SeqCst);
        Self
    }
}

impl Drop for RebuildingGuard {
    fn drop(&mut self) {
        REBUILDING.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
thread_local! {
    /// 测试夹具的日期大多早于 30 天前；只有专门测下限的用例打开它。
    static TEST_IMPORT_FLOOR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn set_test_import_floor(enabled: bool) {
    TEST_IMPORT_FLOOR.with(|floor| floor.set(enabled));
}

/// 事件时间早于明细保留期（已汇总删除的日期）时不导入，重建期间除外。
pub(crate) fn is_below_import_floor(created_at: i64) -> bool {
    #[cfg(test)]
    if !TEST_IMPORT_FLOOR.with(|floor| floor.get()) {
        return false;
    }
    if REBUILDING.load(Ordering::SeqCst) {
        return false;
    }
    compute_local_midnight_cutoff(chrono::Local::now(), USAGE_DETAIL_RETENTION_DAYS)
        .is_ok_and(|cutoff| created_at < cutoff)
}

pub(crate) fn is_rebuild_pending(db: &Database) -> Result<bool, AppError> {
    let conn = lock_conn!(db.conn);
    let value = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [USAGE_REBUILD_PENDING_KEY],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| AppError::Database(format!("读取用量重建标记失败: {e}")))?;
    Ok(value.flatten().is_some_and(|value| value == "true"))
}

fn data_source_list() -> String {
    SESSION_DATA_SOURCES
        .iter()
        .map(|source| format!("'{source}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// 清掉会话导入的明细和导入游标，从头导入全部会话日志，再重新汇总。
///
/// 调用方持有 `session_sync_mutex`，保证后台同步不会插在清理和重导之间。
pub fn rebuild_session_usage(db: &Database) -> Result<SessionSyncResult, AppError> {
    db.backup_database_file()?;
    let sources = data_source_list();
    {
        let conn = lock_conn!(db.conn);
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(format!("开启用量重建事务失败: {e}")))?;
        tx.execute_batch(&format!(
            "DELETE FROM proxy_request_logs WHERE data_source IN ({sources});
             DELETE FROM session_log_sync;
             DELETE FROM session_usage_dedup;"
        ))
        .map_err(|e| AppError::Database(format!("清理会话用量失败: {e}")))?;
        tx.commit()
            .map_err(|e| AppError::Database(format!("提交用量重建清理失败: {e}")))?;
    }
    crate::services::session_usage_codex::clear_codex_replay_caches();

    let result = {
        let _rebuilding = RebuildingGuard::enter();
        sync_all_unlocked(db)
    };

    {
        // 日志覆盖到的日期，旧汇总作废，由下面的 rollup 用重新导入的明细生成
        let conn = lock_conn!(db.conn);
        conn.execute(
            &format!(
                "DELETE FROM usage_daily_rollups
                 WHERE EXISTS (
                     SELECT 1 FROM proxy_request_logs l
                     WHERE l.data_source IN ({sources})
                       AND l.app_type = usage_daily_rollups.app_type
                       AND date(l.created_at, 'unixepoch', 'localtime') = usage_daily_rollups.date
                 )"
            ),
            [],
        )
        .map_err(|e| AppError::Database(format!("清理被重建日期的用量汇总失败: {e}")))?;
        conn.execute(
            "DELETE FROM settings WHERE key = ?1",
            [USAGE_REBUILD_PENDING_KEY],
        )
        .map_err(|e| AppError::Database(format!("清除用量重建标记失败: {e}")))?;
    }
    db.rollup_and_prune(USAGE_DETAIL_RETENTION_DAYS)?;
    crate::usage_events::notify_log_recorded();
    log::info!(
        "[USAGE-REBUILD] 按会话日志重建用量完成：导入 {} 条，跳过 {} 条，{} 个错误",
        result.imported,
        result.skipped,
        result.errors.len()
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, NaiveDate, TimeZone};

    fn local_midnight(date: &str) -> i64 {
        let day = NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("date");
        Local
            .from_local_datetime(&day.and_hms_opt(0, 0, 0).expect("midnight"))
            .earliest()
            .expect("local midnight")
            .timestamp()
    }

    #[test]
    fn import_floor_skips_pruned_days_except_during_rebuild() {
        set_test_import_floor(true);
        let now = Local::now().timestamp();
        let pruned_day = now - 40 * 86_400;
        assert!(is_below_import_floor(pruned_day));
        assert!(!is_below_import_floor(now - 86_400));
        {
            let _rebuilding = RebuildingGuard::enter();
            assert!(!is_below_import_floor(pruned_day));
        }
        assert!(is_below_import_floor(pruned_day));
        set_test_import_floor(false);
    }

    /// 在数据库副本上走一遍升级和重建，按月打印各应用的费用。`CC_SWITCH_TEST_HOME`
    /// 指向一个临时目录，里面放 `.cc-switch/cc-switch.db` 的副本，各 CLI 的目录
    /// 链接到真实目录：
    /// `CC_SWITCH_TEST_HOME=/tmp/home cargo test --release --lib upgrade_database_copy -- --ignored --nocapture`
    #[test]
    #[ignore = "needs CC_SWITCH_TEST_HOME with a copy of a real database"]
    fn upgrade_database_copy() {
        assert!(std::env::var("CC_SWITCH_TEST_HOME").is_ok());
        let db = Database::init().expect("init");
        println!(
            "rebuild pending: {}",
            is_rebuild_pending(&db).expect("flag")
        );
        let result = rebuild_session_usage(&db).expect("rebuild");
        println!(
            "imported={} skipped={} deferred={} errors={:?}",
            result.imported, result.skipped, result.deferred_files, result.errors
        );
        let conn = db.conn.lock().expect("lock");
        let mut stmt = conn
            .prepare(
                "SELECT substr(day, 1, 7), app_type, ROUND(SUM(cost), 2) FROM (
                     SELECT date(created_at, 'unixepoch', 'localtime') AS day, app_type,
                            CAST(total_cost_usd AS REAL) AS cost
                     FROM proxy_request_logs
                     UNION ALL
                     SELECT date, app_type, CAST(total_cost_usd AS REAL) FROM usage_daily_rollups
                 ) GROUP BY 1, 2 ORDER BY 1, 2",
            )
            .expect("prepare");
        let rows = stmt
            .query_map([], |row| {
                Ok(format!(
                    "{}\t{}\t{}",
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?
                ))
            })
            .expect("query");
        for row in rows {
            println!("{}", row.expect("row"));
        }
    }

    /// 把本机的全部会话日志导入内存数据库，按应用和模型打印合计，用来和 ccusage 对照。
    /// 只读日志，不碰 `~/.cc-switch`：
    /// `CC_USAGE_RANGES=2026-09-07..2026-10-07 cargo test --release --lib real_session_logs_totals -- --ignored --nocapture`
    #[test]
    #[ignore = "reads the real session logs under $HOME"]
    fn real_session_logs_totals() {
        let db = Database::memory().expect("memory db");
        let result = sync_all_unlocked(&db);
        println!(
            "imported={} skipped={} deferred={} errors={:?}",
            result.imported, result.skipped, result.deferred_files, result.errors
        );
        let ranges = std::env::var("CC_USAGE_RANGES").unwrap_or_default();
        let conn = db.conn.lock().expect("lock");
        // CC_USAGE_DB_OUT=/path/to.db：把导入结果另存一份，方便逐条对照
        if let Ok(out) = std::env::var("CC_USAGE_DB_OUT") {
            conn.execute("VACUUM INTO ?1", [&out]).expect("vacuum into");
        }
        for range in ranges.split(',').filter(|r| !r.is_empty()) {
            let (from, to) = range.split_once("..").expect("FROM..TO");
            println!("== {from} .. {to} (to exclusive)");
            let mut stmt = conn
                .prepare(
                    "SELECT app_type, model, COUNT(*), SUM(input_tokens), SUM(output_tokens),
                            SUM(cache_read_tokens), SUM(cache_creation_tokens),
                            SUM(cache_creation_1h_tokens), SUM(CAST(total_cost_usd AS REAL)),
                            SUM(CASE WHEN service_tier <> '' THEN 1 ELSE 0 END)
                     FROM proxy_request_logs
                     WHERE created_at >= ?1 AND created_at < ?2
                     GROUP BY 1, 2 ORDER BY 1, 9 DESC",
                )
                .expect("prepare");
            let rows = stmt
                .query_map([local_midnight(from), local_midnight(to)], |row| {
                    Ok(format!(
                        "{}\t{}\treq={}\tin={}\tout={}\tcr={}\tcw={}\tcw1h={}\tcost={:.2}\tpriority={}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, f64>(8)?,
                        row.get::<_, i64>(9)?,
                    ))
                })
                .expect("query");
            for row in rows {
                println!("{}", row.expect("row"));
            }
        }
    }
}
