//! 按会话日志重建用量。
//!
//! 用量明细是从各 CLI 的会话日志导入的，导入逻辑或计价规则改了以后，已入库的
//! 数字不会自己变。重建按日志重算每个来源的"重算范围"（见 [`RebuildHorizons`]）：
//! 删掉范围内的会话明细、导入账本条目和全部导入游标，从头重新导入，再删掉有重导
//! 用量的 (应用, 日期) 的旧汇总，用重导的明细重新汇总。
//!
//! 重算范围按工具自己的日志保留期定：会自动删日志的工具（Claude Code、Gemini CLI）
//! 只重算日志还在的日期，更早的日期保留原有汇总；不删日志的工具全部重算。范围内
//! 日志已经删除的会话不再计入；旧版本地路由按真实供应商记的汇总，在重算的日期里
//! 换成按会话日志算的数字。
//!
//! 平时的增量同步对 30 天前的事件按导入账本判断（见 [`import_gate`]）：导入过的
//! 已经汇总后删除，再导一遍会在下次汇总时把同一批请求加进同一天第二次；从没导入过
//! 的（例如很久没打开应用期间产生的）照常导入。

use crate::database::dao::usage_rollup::{
    compute_local_midnight_cutoff, USAGE_DETAIL_RETENTION_DAYS,
};
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::session_usage::{sync_all_unlocked, SessionSyncResult};
use rusqlite::OptionalExtension;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

/// `settings` 表里的标记：下次会话同步改为整体重建。数据库迁移在导入或计价
/// 规则变化时写入。
pub(crate) const USAGE_REBUILD_PENDING_KEY: &str = "usage_rebuild_pending";

/// `settings` 表里导入账本开始记账的时间（本地零点，秒）。早于它的事件导入时
/// 账本里查不到，要按那一天有没有用量来判断，见 [`import_gate`]。
pub(crate) const USAGE_IMPORT_LEDGER_SINCE_KEY: &str = "usage_import_ledger_since";

/// 会话导入写入的 `data_source`。重建只清这些来源在重算范围内的明细；旧版本地
/// 路由记录的明细行（30 天内）不动，它们的汇总在重算的日期里会被替换。
const SESSION_DATA_SOURCES: &[&str] = &[
    "session_log",
    "codex_session",
    "gemini_session",
    "grok_session",
    "opencode_session",
    "pi_session",
    "mcode_session",
];

/// 每个会话来源的重算起点（本地零点，秒），早于它的事件重建时不导入、日期的汇总
/// 不删除。`None` 表示不设限。
///
/// - Claude Code：设置里的 `cleanupPeriodDays`（默认 30 天）之前的日志会被删除；
/// - Gemini CLI：`general.sessionRetention`（默认开启、`maxAge` 30 天）；
/// - mcode：本地用量表没有说明保留规则，只重算明细保留期（30 天）以内；
/// - Codex、OpenCode、Pi、Grok Build 不自动删日志，不设限。
#[derive(Debug, Clone, Default)]
pub(crate) struct RebuildHorizons(Vec<(&'static str, i64)>);

impl RebuildHorizons {
    fn current() -> Result<Self, AppError> {
        let now = chrono::Local::now();
        let mut horizons = vec![(
            "session_log",
            compute_local_midnight_cutoff(now, claude_cleanup_days())?,
        )];
        if let Some(days) = gemini_retention_days() {
            horizons.push(("gemini_session", compute_local_midnight_cutoff(now, days)?));
        }
        horizons.push((
            "mcode_session",
            compute_local_midnight_cutoff(now, USAGE_DETAIL_RETENTION_DAYS)?,
        ));
        Ok(Self(horizons))
    }

    fn of(&self, data_source: &str) -> Option<i64> {
        self.0
            .iter()
            .find(|(source, _)| *source == data_source)
            .map(|(_, horizon)| *horizon)
    }

    /// 某个来源重算范围内的明细：`{alias}data_source = '…' AND {alias}created_at >= …`
    fn range_sql(&self, data_source: &str, alias: &str) -> String {
        match self.of(data_source) {
            Some(horizon) => {
                format!("({alias}data_source = '{data_source}' AND {alias}created_at >= {horizon})")
            }
            None => format!("({alias}data_source = '{data_source}')"),
        }
    }

    /// 全部会话来源重算范围内的明细
    fn all_ranges_sql(&self, alias: &str) -> String {
        SESSION_DATA_SOURCES
            .iter()
            .map(|source| self.range_sql(source, alias))
            .collect::<Vec<_>>()
            .join(" OR ")
    }
}

/// Claude Code 的 `cleanupPeriodDays`，读不到时用 Claude Code 的默认值 30。
fn claude_cleanup_days() -> i64 {
    let path = crate::config::get_claude_config_dir().join("settings.json");
    read_json(&path)
        .and_then(|settings| settings.get("cleanupPeriodDays")?.as_i64())
        .map_or(30, |days| days.max(0))
}

/// Gemini CLI 的会话保留天数；关掉了自动清理返回 `None`。
fn gemini_retention_days() -> Option<i64> {
    let settings = read_json(&crate::gemini_config::get_gemini_settings_path());
    let retention = settings
        .as_ref()
        .and_then(|settings| settings.get("general")?.get("sessionRetention"));
    let enabled = retention
        .and_then(|retention| retention.get("enabled")?.as_bool())
        .unwrap_or(true);
    if !enabled {
        return None;
    }
    let max_age = retention.and_then(|retention| retention.get("maxAge")?.as_str());
    Some(max_age.and_then(parse_retention_days).unwrap_or(30))
}

/// Gemini CLI 的时长写法（`24h`、`7d`、`4w`）换成天数，不足一天按一天算。
fn parse_retention_days(value: &str) -> Option<i64> {
    let value = value.trim();
    let (number, unit) = value.split_at(value.len().checked_sub(1)?);
    let number: i64 = number.trim().parse().ok().filter(|n| *n >= 0)?;
    match unit {
        "h" => Some((number + 23) / 24),
        "d" => Some(number),
        "w" => number.checked_mul(7),
        _ => None,
    }
}

fn read_json(path: &std::path::Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

thread_local! {
    /// 重建期间的重算范围。重建在调用线程上依次跑各导入器，所以按线程记。
    static REBUILD_HORIZONS: RefCell<Option<RebuildHorizons>> = const { RefCell::new(None) };
}

struct RebuildingGuard;

impl RebuildingGuard {
    fn enter(horizons: &RebuildHorizons) -> Self {
        REBUILD_HORIZONS.with(|cell| *cell.borrow_mut() = Some(horizons.clone()));
        Self
    }
}

impl Drop for RebuildingGuard {
    fn drop(&mut self) {
        REBUILD_HORIZONS.with(|cell| *cell.borrow_mut() = None);
    }
}

/// 重建期间按重算范围判断；不在重建中返回 `None`。
fn rebuild_gate(data_source: &str, created_at: i64) -> Option<bool> {
    REBUILD_HORIZONS.with(|cell| {
        cell.borrow().as_ref().map(|horizons| {
            horizons
                .of(data_source)
                .is_none_or(|horizon| created_at >= horizon)
        })
    })
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

/// 这个会话事件要不要导入。
///
/// 早于明细保留期（30 天）的日期已经汇总、明细已删除，同一事件再导入一次会在下次
/// 汇总时把它加进同一天第二次。导入账本 `usage_import_ledger` 记着每个导入过的事件
/// （由 `proxy_request_logs` 上的触发器写入），所以：
///
/// - 不早于保留期：导入（明细表的主键挡住重复）；
/// - 账本里有：导入过，跳过；
/// - 不早于账本开始记账的时间：没导入过，导入，下次汇总时累加进那一天；
/// - 更早：账本出现之前的事件查不到记录，那一天（按应用）已经有汇总就当作导入过，
///   整天都没有用量时才导入（例如升级前很久没打开应用）；
/// - 重建标记还在时（重建失败过或还没跑），账本里查不到的旧事件一律按上一条判断。
///
/// 重建期间按该来源的重算范围导入，见 [`RebuildHorizons`]。
pub(crate) fn import_gate(
    conn: &rusqlite::Connection,
    data_source: &str,
    request_id: &str,
    created_at: i64,
) -> bool {
    if let Some(allowed) = rebuild_gate(data_source, created_at) {
        return allowed;
    }
    #[cfg(test)]
    if !TEST_IMPORT_FLOOR.with(|floor| floor.get()) {
        return true;
    }
    let Ok(cutoff) =
        compute_local_midnight_cutoff(chrono::Local::now(), USAGE_DETAIL_RETENTION_DAYS)
    else {
        return true;
    };
    if created_at >= cutoff {
        return true;
    }
    match gate_old_event(conn, data_source, request_id, created_at) {
        Ok(allowed) => allowed,
        Err(e) => {
            log::warn!("[USAGE-IMPORT] 判断旧事件 {request_id} 是否导入过失败，跳过: {e}");
            false
        }
    }
}

fn gate_old_event(
    conn: &rusqlite::Connection,
    data_source: &str,
    request_id: &str,
    created_at: i64,
) -> Result<bool, AppError> {
    let in_ledger: bool = conn
        .prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM usage_import_ledger
                           WHERE data_source = ?1 AND request_id = ?2)",
        )
        .and_then(|mut stmt| stmt.query_row([data_source, request_id], |row| row.get(0)))
        .map_err(|e| AppError::Database(format!("查询导入账本失败: {e}")))?;
    if in_ledger {
        return Ok(false);
    }
    let since: Option<i64> = conn
        .prepare_cached("SELECT CAST(value AS INTEGER) FROM settings WHERE key = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([USAGE_IMPORT_LEDGER_SINCE_KEY], |row| row.get(0))
                .optional()
        })
        .map_err(|e| AppError::Database(format!("读取导入账本起点失败: {e}")))?;
    // 重建标记还在时账本可能缺了一部分：重建开头按范围删了账本，整个读不了的来源没能
    // 重新记账。按账本判断会把它导入过、已经汇总的事件在那些天上再算一次，所以这时和
    // 账本出现之前一样，按那一天有没有汇总判断。
    let ledger_complete = !rebuild_pending_on(conn)?;
    if ledger_complete && since.is_none_or(|since| created_at >= since) {
        return Ok(true);
    }
    // 早于保留期的明细在启动和每天的汇总里已经并进汇总表，这里只看汇总
    let app_types = app_types_for_source(data_source);
    if app_types.is_empty() {
        return Ok(false);
    }
    let placeholders = app_types
        .iter()
        .map(|app| format!("'{app}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let day_has_usage: bool = conn
        .prepare_cached(&format!(
            "SELECT EXISTS(SELECT 1 FROM usage_daily_rollups
                           WHERE date = date(?1, 'unixepoch', 'localtime')
                             AND app_type IN ({placeholders}))"
        ))
        .and_then(|mut stmt| stmt.query_row([created_at], |row| row.get(0)))
        .map_err(|e| AppError::Database(format!("查询当天用量汇总失败: {e}")))?;
    Ok(!day_has_usage)
}

/// 这个事件导入过、并且明细已经汇总删除了。
///
/// Codex 的 record 行取代按旧规则导入的 token_count 行（同一次响应）：旧行还在明细
/// 里就删掉换成新行；已经汇总进按天合计，再导入 record 就会把这次响应算两次。
pub(crate) fn already_rolled_up(
    conn: &rusqlite::Connection,
    data_source: &str,
    request_id: &str,
) -> bool {
    conn.prepare_cached(
        "SELECT EXISTS(SELECT 1 FROM usage_import_ledger
                       WHERE data_source = ?1 AND request_id = ?2)
            AND NOT EXISTS(SELECT 1 FROM proxy_request_logs WHERE request_id = ?2)",
    )
    .and_then(|mut stmt| stmt.query_row([data_source, request_id], |row| row.get(0)))
    .unwrap_or_else(|e| {
        log::warn!("[USAGE-IMPORT] 查询 {request_id} 是否已汇总失败: {e}");
        false
    })
}

/// 会话来源写入的 app_type。旧版本地路由把 Claude Desktop 的 Code 页面记作
/// `claude-desktop`，它写的是 Claude Code 的会话日志，所以算进 Claude。
fn app_types_for_source(data_source: &str) -> &'static [&'static str] {
    match data_source {
        "session_log" => &["claude", "claude-desktop"],
        "codex_session" => &["codex"],
        "gemini_session" => &["gemini"],
        "grok_session" => &["grokbuild"],
        "opencode_session" => &["opencode"],
        "pi_session" => &["pi"],
        "mcode_session" => &["mcode"],
        _ => &[],
    }
}

pub(crate) fn is_rebuild_pending(db: &Database) -> Result<bool, AppError> {
    let conn = lock_conn!(db.conn);
    rebuild_pending_on(&conn)
}

fn rebuild_pending_on(conn: &rusqlite::Connection) -> Result<bool, AppError> {
    let value = conn
        .prepare_cached("SELECT value FROM settings WHERE key = ?1")
        .and_then(|mut stmt| {
            stmt.query_row([USAGE_REBUILD_PENDING_KEY], |row| {
                row.get::<_, Option<String>>(0)
            })
            .optional()
        })
        .map_err(|e| AppError::Database(format!("读取用量重建标记失败: {e}")))?;
    Ok(value.flatten().is_some_and(|value| value == "true"))
}

/// 本次运行里重建失败过：自动同步的定时轮次不再重试（每分钟清一遍明细太重），
/// 等下次启动或手动同步。
static REBUILD_FAILED_THIS_RUN: AtomicBool = AtomicBool::new(false);

/// 一轮会话同步：有重建标记就重建，否则增量同步。
///
/// 手动同步（`retry_failed_rebuild = true`）总会处理标记；自动同步的定时轮次在本次
/// 运行重建失败过以后只做增量同步。调用方持有 `session_sync_mutex`。
pub fn sync_or_rebuild(
    db: &Database,
    retry_failed_rebuild: bool,
) -> Result<SessionSyncResult, AppError> {
    if should_rebuild(db, retry_failed_rebuild)? {
        return rebuild_session_usage(db);
    }
    Ok(sync_all_unlocked(db))
}

fn should_rebuild(db: &Database, retry_failed_rebuild: bool) -> Result<bool, AppError> {
    let may_rebuild = retry_failed_rebuild || !REBUILD_FAILED_THIS_RUN.load(Ordering::Relaxed);
    Ok(may_rebuild && is_rebuild_pending(db)?)
}

/// 按会话日志重建用量，见模块说明。
///
/// 调用方持有 `session_sync_mutex`，保证后台同步不会插在清理和重导之间。数据库
/// 出错或某个来源整个读不了时返回 Err，重建标记保留，下次再试；单个文件解析失败
/// 只记在结果里。
pub fn rebuild_session_usage(db: &Database) -> Result<SessionSyncResult, AppError> {
    let outcome = db
        .backup_database_file()
        .and_then(|_| RebuildHorizons::current())
        .and_then(|horizons| rebuild_with(db, &horizons, sync_all_unlocked));
    REBUILD_FAILED_THIS_RUN.store(outcome.is_err(), Ordering::Relaxed);
    outcome
}

fn rebuild_with(
    db: &Database,
    horizons: &RebuildHorizons,
    sync: impl FnOnce(&Database) -> SessionSyncResult,
) -> Result<SessionSyncResult, AppError> {
    clear_rebuilt_ranges(db, horizons)?;
    crate::services::session_usage_codex::clear_codex_replay_caches();
    let outcome = reimport_and_rollup(db, horizons, sync);
    // 明细已经清过，成败都要让打开的看板刷新
    crate::usage_events::notify_log_recorded();
    outcome
}

fn clear_rebuilt_ranges(db: &Database, horizons: &RebuildHorizons) -> Result<(), AppError> {
    let ranges = horizons.all_ranges_sql("");
    let conn = lock_conn!(db.conn);
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::Database(format!("开启用量重建事务失败: {e}")))?;
    tx.execute_batch(&format!(
        "DELETE FROM proxy_request_logs WHERE {ranges};
         DELETE FROM usage_import_ledger WHERE {ranges};
         DELETE FROM session_log_sync;
         DELETE FROM session_usage_dedup;"
    ))
    .map_err(|e| AppError::Database(format!("清理会话用量失败: {e}")))?;
    tx.commit()
        .map_err(|e| AppError::Database(format!("提交用量重建清理失败: {e}")))
}

fn reimport_and_rollup(
    db: &Database,
    horizons: &RebuildHorizons,
    sync: impl FnOnce(&Database) -> SessionSyncResult,
) -> Result<SessionSyncResult, AppError> {
    let result = {
        let _rebuilding = RebuildingGuard::enter(horizons);
        sync(db)
    };

    {
        let conn = lock_conn!(db.conn);
        // 有重导用量的 (应用, 日期)，旧汇总作废，由下面的 rollup 用重导的明细生成。
        // Claude Desktop 的 Code 页面写的也是 Claude Code 日志，旧路由按
        // claude-desktop 记过一份，同一天的也删掉
        let ranges = horizons.all_ranges_sql("");
        conn.execute(
            &format!(
                "DELETE FROM usage_daily_rollups WHERE (app_type, date) IN (
                     SELECT app_type, date(created_at, 'unixepoch', 'localtime')
                     FROM proxy_request_logs WHERE {ranges}
                     UNION
                     SELECT 'claude-desktop', date(created_at, 'unixepoch', 'localtime')
                     FROM proxy_request_logs WHERE app_type = 'claude' AND ({ranges})
                 )"
            ),
            [],
        )
        .map_err(|e| AppError::Database(format!("清理被重建日期的用量汇总失败: {e}")))?;
        let duplicates =
            crate::services::usage_proxy_dedup::delete_session_rows_duplicating_proxy(&conn)?;
        if duplicates > 0 {
            log::info!("[USAGE-REBUILD] 删除与旧路由记录重复的会话用量 {duplicates} 条");
        }
    }
    db.rollup_and_prune(USAGE_DETAIL_RETENTION_DAYS)?;

    if !result.failed_sources.is_empty() {
        return Err(AppError::Message(format!(
            "用量重建没有完成，下次启动时重试：{}",
            result.errors.join("; ")
        )));
    }
    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "DELETE FROM settings WHERE key = ?1",
            [USAGE_REBUILD_PENDING_KEY],
        )
        .map_err(|e| AppError::Database(format!("清除用量重建标记失败: {e}")))?;
    }
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

    const DAY: i64 = 86_400;

    fn set_ledger_since(db: &Database, since: i64) {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![USAGE_IMPORT_LEDGER_SINCE_KEY, since.to_string()],
        )
        .unwrap();
    }

    fn insert_session_row(db: &Database, source: &str, app: &str, request_id: &str, at: i64) {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO proxy_request_logs (
                 request_id, provider_id, app_type, model, input_tokens, output_tokens,
                 total_cost_usd, latency_ms, status_code, created_at, data_source)
             VALUES (?1, '_s', ?2, 'm', 10, 5, '0.5', 0, 200, ?3, ?4)",
            rusqlite::params![request_id, app, at, source],
        )
        .unwrap();
    }

    fn insert_rollup(db: &Database, app: &str, at: i64) {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                 date, app_type, provider_id, model, request_count, success_count,
                 input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                 total_cost_usd, avg_latency_ms)
             VALUES (date(?1, 'unixepoch', 'localtime'), ?2, 'p', 'm', 3, 3, 1, 1, 0, 0, '1', 0)",
            rusqlite::params![at, app],
        )
        .unwrap();
    }

    fn gate(db: &Database, source: &str, request_id: &str, at: i64) -> bool {
        let conn = db.conn.lock().unwrap();
        import_gate(&conn, source, request_id, at)
    }

    #[test]
    fn import_gate_uses_the_ledger_for_old_events() {
        set_test_import_floor(true);
        let db = Database::memory().unwrap();
        let now = Local::now().timestamp();
        set_ledger_since(&db, now - 60 * DAY);
        let old = now - 45 * DAY;

        assert!(gate(&db, "gemini_session", "recent", now - DAY));
        // 账本出现之后、从没导入过 → 导入
        assert!(gate(&db, "gemini_session", "never-seen", old));
        // 导入过（触发器记了账）→ 跳过
        insert_session_row(&db, "gemini_session", "gemini", "seen", old);
        assert!(!gate(&db, "gemini_session", "seen", old));
        {
            // 重建时按重算范围判断，不看账本
            let _rebuilding = RebuildingGuard::enter(&RebuildHorizons::default());
            assert!(gate(&db, "gemini_session", "seen", old));
        }
        {
            let horizons = RebuildHorizons(vec![("gemini_session", now - 30 * DAY)]);
            let _rebuilding = RebuildingGuard::enter(&horizons);
            assert!(!gate(&db, "gemini_session", "seen", old));
            assert!(gate(&db, "gemini_session", "recent", now - DAY));
        }
        set_test_import_floor(false);
    }

    #[test]
    fn import_gate_checks_whole_days_before_the_ledger_existed() {
        set_test_import_floor(true);
        let db = Database::memory().unwrap();
        let now = Local::now().timestamp();
        set_ledger_since(&db, now - 30 * DAY);
        let day_with_usage = now - 45 * DAY;
        let empty_day = now - 50 * DAY;
        insert_rollup(&db, "gemini", day_with_usage);
        insert_rollup(&db, "claude-desktop", day_with_usage);

        assert!(!gate(&db, "gemini_session", "a", day_with_usage));
        assert!(gate(&db, "gemini_session", "b", empty_day));
        // Claude Desktop 的旧汇总算 Claude 的
        assert!(!gate(&db, "session_log", "c", day_with_usage));
        // 别的应用那天有用量不影响
        assert!(gate(&db, "codex_session", "d", day_with_usage));
        set_test_import_floor(false);
    }

    #[test]
    fn import_gate_checks_whole_days_while_a_rebuild_is_pending() {
        set_test_import_floor(true);
        let db = Database::memory().unwrap();
        let now = Local::now().timestamp();
        set_ledger_since(&db, now - 60 * DAY);
        let day_with_usage = now - 45 * DAY;
        let empty_day = now - 50 * DAY;
        insert_rollup(&db, "codex", day_with_usage);

        // 账本完整时：账本出现之后、账本里没有的旧事件照常导入
        assert!(gate(&db, "codex_session", "a", day_with_usage));
        set_rebuild_pending(&db);
        // 重建标记还在：那天已有汇总就跳过，整天没有用量才导入
        assert!(!gate(&db, "codex_session", "a", day_with_usage));
        assert!(gate(&db, "codex_session", "b", empty_day));
        // 30 天内的事件不受影响
        assert!(gate(&db, "codex_session", "c", now - DAY));
        set_test_import_floor(false);
    }

    /// 重建时某个来源整个读不了：它的账本已被清掉，旧汇总还在。同一次运行里它恢复
    /// 可读后，定时同步（这次运行不再重建）不能把它导入过的旧事件再算一次。
    #[test]
    fn a_source_that_recovers_after_a_failed_rebuild_is_not_counted_twice() {
        set_test_import_floor(true);
        let db = Database::memory().unwrap();
        let now = Local::now().timestamp();
        set_ledger_since(&db, now - 60 * DAY);
        let day = now - 45 * DAY;
        // 这个事件以前导入过：账本有记录，明细已经汇总进那一天（3 次请求）
        insert_rollup(&db, "codex", day);
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO usage_import_ledger (data_source, request_id, created_at)
                 VALUES ('codex_session', 'counted', ?1)",
                [day],
            )
            .unwrap();
        }
        set_rebuild_pending(&db);

        let outcome = rebuild_with(&db, &unlimited(), |_| SessionSyncResult {
            errors: vec!["Codex 会话目录读取失败".to_string()],
            failed_sources: vec!["Codex".to_string()],
            ..Default::default()
        });
        assert!(outcome.is_err());

        // Codex 恢复可读，定时同步按增量导入同一个事件，之后照常汇总
        let result = import_events(&db, &[("codex_session", "codex", "counted", day)]);
        db.rollup_and_prune(USAGE_DETAIL_RETENTION_DAYS).unwrap();

        assert_eq!(result.imported, 0);
        assert_eq!(rollup_requests(&db, "codex", day), 3);
        set_test_import_floor(false);
    }

    fn set_rebuild_pending(db: &Database) {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, 'true')",
            [USAGE_REBUILD_PENDING_KEY],
        )
        .unwrap();
    }

    /// 重建里的导入步骤：过了 import_gate 才写库，和真实导入器一样
    fn import_events(db: &Database, events: &[(&str, &str, &str, i64)]) -> SessionSyncResult {
        let mut result = SessionSyncResult::default();
        for &(source, app, request_id, at) in events {
            if gate(db, source, request_id, at) {
                insert_session_row(db, source, app, request_id, at);
                result.imported += 1;
            } else {
                result.skipped += 1;
            }
        }
        result
    }

    /// (应用, 日期) 的汇总请求数
    fn rollup_requests(db: &Database, app: &str, at: i64) -> i64 {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT COALESCE(SUM(request_count), 0) FROM usage_daily_rollups
             WHERE app_type = ?1 AND date = date(?2, 'unixepoch', 'localtime')",
            rusqlite::params![app, at],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn unlimited() -> RebuildHorizons {
        RebuildHorizons::default()
    }

    #[test]
    fn rebuild_recounts_days_that_still_have_logs() {
        let db = Database::memory().unwrap();
        let now = Local::now().timestamp();
        let day = now - 60 * DAY;
        let other_day = now - 70 * DAY;
        // 两个会话的旧汇总（3 次请求），重建时只有一个会话的日志还在
        insert_rollup(&db, "claude", day);
        insert_rollup(&db, "claude", other_day);
        set_rebuild_pending(&db);

        let result = rebuild_with(&db, &unlimited(), |db| {
            import_events(db, &[("session_log", "claude", "kept", day)])
        })
        .unwrap();

        assert_eq!(result.imported, 1);
        assert_eq!(rollup_requests(&db, "claude", day), 1);
        // 没有日志的日期保留原有汇总
        assert_eq!(rollup_requests(&db, "claude", other_day), 3);
        assert!(!is_rebuild_pending(&db).unwrap());
    }

    #[test]
    fn rebuild_keeps_days_before_the_claude_cleanup_period() {
        let now = Local::now();
        let day = now.timestamp() - 60 * DAY;
        for (cleanup_days, expected) in [(30, 3), (720, 1)] {
            let db = Database::memory().unwrap();
            insert_rollup(&db, "claude", day);
            let horizons = RebuildHorizons(vec![(
                "session_log",
                compute_local_midnight_cutoff(now, cleanup_days).unwrap(),
            )]);
            rebuild_with(&db, &horizons, |db| {
                import_events(db, &[("session_log", "claude", "resumed", day)])
            })
            .unwrap();
            assert_eq!(
                rollup_requests(&db, "claude", day),
                expected,
                "cleanupPeriodDays = {cleanup_days}"
            );
        }
    }

    #[test]
    fn rebuild_replaces_claude_desktop_rollups_of_the_same_day() {
        let db = Database::memory().unwrap();
        let day = Local::now().timestamp() - 60 * DAY;
        insert_rollup(&db, "claude", day);
        insert_rollup(&db, "claude-desktop", day);

        rebuild_with(&db, &unlimited(), |db| {
            import_events(db, &[("session_log", "claude", "from-desktop", day)])
        })
        .unwrap();

        assert_eq!(rollup_requests(&db, "claude", day), 1);
        assert_eq!(rollup_requests(&db, "claude-desktop", day), 0);
    }

    #[test]
    fn rebuild_keeps_the_flag_when_a_source_fails() {
        let db = Database::memory().unwrap();
        set_rebuild_pending(&db);
        crate::usage_events::take_test_notify_count();

        let outcome = rebuild_with(&db, &unlimited(), |_| SessionSyncResult {
            errors: vec!["Codex 同步失败: database is locked".to_string()],
            failed_sources: vec!["Codex".to_string()],
            ..Default::default()
        });

        assert!(outcome.is_err());
        assert!(is_rebuild_pending(&db).unwrap());
        assert_eq!(crate::usage_events::take_test_notify_count(), 1);
    }

    #[test]
    fn rebuild_clears_the_flag_when_only_single_files_fail() {
        let db = Database::memory().unwrap();
        set_rebuild_pending(&db);
        let day = Local::now().timestamp() - 60 * DAY;

        let result = rebuild_with(&db, &unlimited(), |db| {
            let mut result = import_events(db, &[("codex_session", "codex", "good", day)]);
            result.errors.push("broken.jsonl: 解析失败".to_string());
            result
        })
        .unwrap();

        assert_eq!(result.errors.len(), 1);
        assert!(!is_rebuild_pending(&db).unwrap());
        assert_eq!(rollup_requests(&db, "codex", day), 1);
    }

    #[test]
    fn rebuild_keeps_session_rows_before_the_horizon() {
        let db = Database::memory().unwrap();
        let now = Local::now();
        let horizon = compute_local_midnight_cutoff(now, 7).unwrap();
        let before = horizon - DAY;
        insert_session_row(&db, "session_log", "claude", "old", before);
        insert_session_row(&db, "session_log", "claude", "recent", horizon + 60);
        let horizons = RebuildHorizons(vec![("session_log", horizon)]);

        rebuild_with(&db, &horizons, |db| {
            import_events(
                db,
                &[
                    ("session_log", "claude", "old", before),
                    ("session_log", "claude", "recent", horizon + 60),
                ],
            )
        })
        .unwrap();

        let conn = db.conn.lock().unwrap();
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM proxy_request_logs", [], |row| {
                row.get(0)
            })
            .unwrap();
        let ledgered: i64 = conn
            .query_row("SELECT COUNT(*) FROM usage_import_ledger", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!((rows, ledgered), (2, 2));
    }

    #[test]
    fn manual_sync_retries_a_failed_rebuild_but_timed_rounds_do_not() {
        let db = Database::memory().unwrap();
        assert!(!should_rebuild(&db, true).unwrap());
        set_rebuild_pending(&db);
        assert!(should_rebuild(&db, false).unwrap());

        REBUILD_FAILED_THIS_RUN.store(true, Ordering::Relaxed);
        let timed = should_rebuild(&db, false).unwrap();
        let manual = should_rebuild(&db, true).unwrap();
        REBUILD_FAILED_THIS_RUN.store(false, Ordering::Relaxed);
        assert!(!timed);
        assert!(manual);
    }

    #[test]
    fn parses_gemini_retention_durations() {
        assert_eq!(parse_retention_days("30d"), Some(30));
        assert_eq!(parse_retention_days("24h"), Some(1));
        assert_eq!(parse_retention_days("25h"), Some(2));
        assert_eq!(parse_retention_days("4w"), Some(28));
        assert_eq!(parse_retention_days("1y"), None);
        assert_eq!(parse_retention_days(""), None);
    }

    #[test]
    fn events_imported_late_are_added_to_their_day_once() {
        set_test_import_floor(true);
        let db = Database::memory().unwrap();
        let now = Local::now().timestamp();
        set_ledger_since(&db, now - 60 * DAY);
        let old = now - 45 * DAY;
        insert_rollup(&db, "gemini", old);

        assert!(gate(&db, "gemini_session", "late", old));
        insert_session_row(&db, "gemini_session", "gemini", "late", old);
        db.rollup_and_prune(USAGE_DETAIL_RETENTION_DAYS).unwrap();
        assert!(!gate(&db, "gemini_session", "late", old));

        let conn = db.conn.lock().unwrap();
        let (requests, details): (i64, i64) = conn
            .query_row(
                "SELECT (SELECT SUM(request_count) FROM usage_daily_rollups WHERE app_type = 'gemini'),
                        (SELECT COUNT(*) FROM proxy_request_logs)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((requests, details), (4, 0), "3 old requests + the late one");
        drop(conn);
        set_test_import_floor(false);
    }

    #[test]
    fn v21_migration_backfills_the_ledger() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        Database::create_tables_on_conn(&conn).unwrap();
        // 模拟升级前的库：没有触发器时写入的明细
        conn.execute_batch(
            "DROP TRIGGER usage_import_ledger_on_insert;
             INSERT INTO proxy_request_logs (request_id, provider_id, app_type, model,
                 input_tokens, output_tokens, total_cost_usd, latency_ms, status_code,
                 created_at, data_source) VALUES
                 ('s1', '_s', 'claude', 'm', 1, 1, '0', 0, 200, 100, 'session_log'),
                 ('s2', '_s', 'codex', 'm', 1, 1, '0', 0, 200, 200, 'codex_session'),
                 ('p1', 'x', 'claude', 'm', 1, 1, '0', 0, 200, 300, 'proxy');",
        )
        .unwrap();
        Database::set_user_version(&conn, 20).unwrap();

        Database::apply_schema_migrations_on_conn(&conn).unwrap();

        let ledger: Vec<String> = conn
            .prepare("SELECT request_id FROM usage_import_ledger ORDER BY request_id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(ledger, vec!["s1", "s2"]);
        let since: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [USAGE_IMPORT_LEDGER_SINCE_KEY],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert!(since.is_some());
        // 迁移之后的写入由触发器记账
        conn.execute(
            "INSERT INTO proxy_request_logs (request_id, provider_id, app_type, model,
                 input_tokens, output_tokens, total_cost_usd, latency_ms, status_code,
                 created_at, data_source)
             VALUES ('s3', '_s', 'gemini', 'm', 1, 1, '0', 0, 200, 400, 'gemini_session')",
            [],
        )
        .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM usage_import_ledger", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 3);
    }

    /// 在数据库副本上走一遍升级和重建，按月打印各应用的费用。`CC_SWITCH_TEST_HOME`
    /// 指向一个临时目录，里面放 `.ccs-lite/cc-switch.db` 的副本，各 CLI 的目录
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
    /// 只读日志，不碰数据目录：
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
