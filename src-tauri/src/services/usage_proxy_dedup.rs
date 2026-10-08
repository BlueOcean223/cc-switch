//! 会话用量和旧版本地路由记录的行去重。
//!
//! 上游 CC Switch 的本地路由按请求记一行（`data_source = 'proxy'`），同一个请求在
//! 客户端的会话日志里还有一份。路由已经移除，不会再产生新的 proxy 行，但升级前
//! 30 天内的 proxy 行还在明细里，汇总表里更早的也都是按它们算的。导入会话日志时，
//! 和窗口内某个 proxy 行指纹相同的事件不再入库（那次请求已经由 proxy 行计入），
//! 同时记进导入账本，过了保留期也不会被当成没导入过。
//!
//! 指纹：app_type（`claude` 也匹配旧路由的 `claude-desktop`）、模型、新增输入、
//! 输出、缓存命中、缓存写入（任一侧为 0 时不比缓存写入，并且也接受"新增输入 + 缓存写入"
//! 相等：没记缓存写入的一侧可能把它算进了输入），时间相差不超过
//! [`SESSION_PROXY_DEDUP_WINDOW_SECONDS`]，proxy 行成功（2xx）。proxy 行的输入按自己
//! 的 `input_token_semantics` 换算成新增输入后再比，会话行一律存新增输入。

use std::cell::Cell;

use rusqlite::{params, Connection};

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::sql_helpers::fresh_input_sql;

pub(crate) const SESSION_PROXY_DEDUP_WINDOW_SECONDS: i64 = 10 * 60;

/// 会话事件的指纹。`fresh_input_tokens` 是不含缓存的新增输入。
#[derive(Debug, Clone, Copy)]
pub(crate) struct DedupKey<'a> {
    pub app_type: &'a str,
    pub model: &'a str,
    pub fresh_input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_creation_tokens: u32,
    pub created_at: i64,
}

thread_local! {
    /// 本轮同步开始时库里最晚的 proxy 行时间：`None` 表示不知道（每次都查库），
    /// `Some(None)` 表示没有 proxy 行。升级后的新事件都晚于它，不用再查。
    static LATEST_PROXY_AT: Cell<Option<Option<i64>>> = const { Cell::new(None) };
}

/// 一轮同步期间缓存最晚的 proxy 行时间，见 [`begin_round`]。
pub(crate) struct RoundGuard {
    previous: Option<Option<i64>>,
}

impl Drop for RoundGuard {
    fn drop(&mut self) {
        LATEST_PROXY_AT.with(|cell| cell.set(self.previous));
    }
}

/// 同步开始时调用。查不到时不缓存，之后每个事件照常查库。
pub(crate) fn begin_round(db: &Database) -> RoundGuard {
    let previous = LATEST_PROXY_AT.with(Cell::get);
    let latest = (|| -> Result<Option<i64>, AppError> {
        let conn = lock_conn!(db.conn);
        conn.query_row(
            "SELECT MAX(created_at) FROM proxy_request_logs
             WHERE COALESCE(data_source, 'proxy') = 'proxy'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("查询旧路由用量的时间范围失败: {e}")))
    })();
    match latest {
        Ok(latest) => LATEST_PROXY_AT.with(|cell| cell.set(Some(latest))),
        Err(e) => log::warn!("[USAGE-DEDUP] {e}"),
    }
    RoundGuard { previous }
}

fn proxy_rows_possible_near(created_at: i64) -> bool {
    match LATEST_PROXY_AT.with(Cell::get) {
        None => true,
        Some(None) => false,
        Some(Some(latest)) => created_at <= latest + SESSION_PROXY_DEDUP_WINDOW_SECONDS,
    }
}

fn matching_proxy_sql() -> String {
    let fresh_input = fresh_input_sql("l");
    format!(
        "SELECT EXISTS (
            SELECT 1 FROM proxy_request_logs l
            WHERE l.created_at BETWEEN ?7 - ?8 AND ?7 + ?8
              AND COALESCE(l.data_source, 'proxy') = 'proxy'
              AND l.app_type IN (?1, CASE WHEN ?1 = 'claude' THEN 'claude-desktop' ELSE ?1 END)
              AND l.status_code >= 200 AND l.status_code < 300
              AND l.output_tokens = ?4
              AND l.cache_read_tokens = ?5
              AND (
                  (({fresh_input}) = ?3
                   AND (l.cache_creation_tokens = ?6 OR ?6 = 0 OR l.cache_creation_tokens = 0))
                  OR ((?6 = 0 OR l.cache_creation_tokens = 0)
                      AND ({fresh_input}) + l.cache_creation_tokens = ?3 + ?6)
              )
              AND (LOWER(l.model) = LOWER(?2)
                   OR LOWER(l.model) = 'unknown'
                   OR LOWER(?2) = 'unknown')
        )"
    )
}

/// 时间窗口内有指纹相同的 proxy 行：这次请求已经由旧路由记过。
pub(crate) fn has_matching_proxy_usage_log(
    conn: &Connection,
    key: &DedupKey,
) -> Result<bool, AppError> {
    if !proxy_rows_possible_near(key.created_at) {
        return Ok(false);
    }
    conn.prepare_cached(&matching_proxy_sql())
        .and_then(|mut stmt| {
            stmt.query_row(
                params![
                    key.app_type,
                    key.model,
                    i64::from(key.fresh_input_tokens),
                    i64::from(key.output_tokens),
                    i64::from(key.cache_read_tokens),
                    i64::from(key.cache_creation_tokens),
                    key.created_at,
                    SESSION_PROXY_DEDUP_WINDOW_SECONDS,
                ],
                |row| row.get(0),
            )
        })
        .map_err(|e| AppError::Database(format!("查询重复的旧路由用量失败: {e}")))
}

/// 跳过和 proxy 行重复的事件，并记进导入账本：proxy 行过了保留期被汇总删掉以后，
/// 这个事件也不会被当成没导入过再导入一次。返回 true 表示应跳过。
pub(crate) fn skip_if_recorded_by_proxy(
    conn: &Connection,
    data_source: &str,
    request_id: &str,
    key: &DedupKey,
) -> Result<bool, AppError> {
    if !has_matching_proxy_usage_log(conn, key)? {
        return Ok(false);
    }
    record_skipped(conn, data_source, request_id, key.created_at)?;
    Ok(true)
}

/// Grok Build 版的 [`skip_if_recorded_by_proxy`]，判断见 [`has_recent_grokbuild_proxy_activity`]。
pub(crate) fn skip_if_grokbuild_routed(
    conn: &Connection,
    request_id: &str,
    created_at: i64,
) -> Result<bool, AppError> {
    if !has_recent_grokbuild_proxy_activity(conn, created_at)? {
        return Ok(false);
    }
    record_skipped(conn, "grok_session", request_id, created_at)?;
    Ok(true)
}

fn record_skipped(
    conn: &Connection,
    data_source: &str,
    request_id: &str,
    created_at: i64,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO usage_import_ledger (data_source, request_id, created_at)
         VALUES (?1, ?2, ?3)",
        params![data_source, request_id, created_at],
    )
    .map_err(|e| AppError::Database(format!("记录跳过的会话用量失败: {e}")))?;
    Ok(())
}

/// Grok Build：时间窗口内有任何 grokbuild 的 proxy 行，就当作当时走的是旧路由。
///
/// Grok 的会话事件按轮聚合，和 proxy 的逐请求行指纹不会相等，只能按"当时是不是在走
/// 路由"判断。不看状态码：失败的请求也说明流量在走路由。窗口不分会话，路由和直连在
/// 十分钟内交替时直连的轮次会被跳过（少记，不会多记）。
pub(crate) fn has_recent_grokbuild_proxy_activity(
    conn: &Connection,
    created_at: i64,
) -> Result<bool, AppError> {
    if !proxy_rows_possible_near(created_at) {
        return Ok(false);
    }
    conn.prepare_cached(
        "SELECT EXISTS (
            SELECT 1 FROM proxy_request_logs l
            WHERE l.created_at BETWEEN ?1 - ?2 AND ?1 + ?2
              AND COALESCE(l.data_source, 'proxy') = 'proxy'
              AND l.app_type = 'grokbuild'
        )",
    )
    .and_then(|mut stmt| {
        stmt.query_row(
            params![created_at, SESSION_PROXY_DEDUP_WINDOW_SECONDS],
            |row| row.get(0),
        )
    })
    .map_err(|e| AppError::Database(format!("查询 Grok 旧路由活动失败: {e}")))
}

/// 删掉已经入库、和 proxy 行重复的会话明细行（升级前的版本没有在导入时去重）。
/// 条件与 [`has_matching_proxy_usage_log`] 相同；Grok 按窗口内有没有 grokbuild
/// proxy 行判断。返回删掉的行数。
pub(crate) fn delete_session_rows_duplicating_proxy(conn: &Connection) -> Result<usize, AppError> {
    let fresh_input = fresh_input_sql("p");
    let window = SESSION_PROXY_DEDUP_WINDOW_SECONDS;
    conn.execute(
        &format!(
            "DELETE FROM proxy_request_logs WHERE rowid IN (
                SELECT s.rowid
                FROM proxy_request_logs p
                JOIN proxy_request_logs s
                  ON s.created_at BETWEEN p.created_at - {window} AND p.created_at + {window}
                 AND s.app_type = CASE WHEN p.app_type = 'claude-desktop' THEN 'claude'
                                       ELSE p.app_type END
                 AND COALESCE(s.data_source, 'proxy') NOT IN ('proxy', 'grok_session')
                 AND s.output_tokens = p.output_tokens
                 AND s.cache_read_tokens = p.cache_read_tokens
                 AND (
                     (s.input_tokens = ({fresh_input})
                      AND (s.cache_creation_tokens = p.cache_creation_tokens
                           OR s.cache_creation_tokens = 0 OR p.cache_creation_tokens = 0))
                     OR ((s.cache_creation_tokens = 0 OR p.cache_creation_tokens = 0)
                         AND s.input_tokens + s.cache_creation_tokens
                             = ({fresh_input}) + p.cache_creation_tokens)
                 )
                 AND (LOWER(p.model) = LOWER(s.model)
                      OR LOWER(p.model) = 'unknown'
                      OR LOWER(s.model) = 'unknown')
                WHERE COALESCE(p.data_source, 'proxy') = 'proxy'
                  AND p.status_code >= 200 AND p.status_code < 300
                UNION
                SELECT s.rowid
                FROM proxy_request_logs s
                WHERE s.data_source = 'grok_session'
                  AND EXISTS (
                      SELECT 1 FROM proxy_request_logs p
                      WHERE p.created_at BETWEEN s.created_at - {window} AND s.created_at + {window}
                        AND COALESCE(p.data_source, 'proxy') = 'proxy'
                        AND p.app_type = 'grokbuild'
                  )
            )"
        ),
        [],
    )
    .map_err(|e| AppError::Database(format!("清理与旧路由重复的会话用量失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::sql_helpers::{INPUT_TOKEN_SEMANTICS_FRESH, INPUT_TOKEN_SEMANTICS_TOTAL};

    #[allow(clippy::too_many_arguments)]
    fn insert_row(
        conn: &Connection,
        request_id: &str,
        app: &str,
        source: &str,
        model: &str,
        tokens: (u32, u32, u32, u32),
        semantics: i64,
        created_at: i64,
    ) {
        conn.execute(
            "INSERT INTO proxy_request_logs (
                 request_id, provider_id, app_type, model, input_tokens, output_tokens,
                 cache_read_tokens, cache_creation_tokens, total_cost_usd, latency_ms,
                 status_code, created_at, data_source, input_token_semantics)
             VALUES (?1, 'p', ?2, ?3, ?4, ?5, ?6, ?7, '0', 0, 200, ?8, ?9, ?10)",
            params![
                request_id, app, model, tokens.0, tokens.1, tokens.2, tokens.3, created_at, source,
                semantics
            ],
        )
        .unwrap();
    }

    fn key<'a>(
        app: &'a str,
        model: &'a str,
        tokens: (u32, u32, u32, u32),
        at: i64,
    ) -> DedupKey<'a> {
        DedupKey {
            app_type: app,
            model,
            fresh_input_tokens: tokens.0,
            output_tokens: tokens.1,
            cache_read_tokens: tokens.2,
            cache_creation_tokens: tokens.3,
            created_at: at,
        }
    }

    #[test]
    fn matches_proxy_rows_inside_the_window_only() {
        let db = Database::memory().unwrap();
        let conn = db.conn.lock().unwrap();
        insert_row(
            &conn,
            "proxy-1",
            "claude",
            "proxy",
            "claude-sonnet-5",
            (100, 50, 10, 0),
            INPUT_TOKEN_SEMANTICS_FRESH,
            1_000,
        );

        let tokens = (100, 50, 10, 0);
        assert!(has_matching_proxy_usage_log(
            &conn,
            &key("claude", "claude-sonnet-5", tokens, 1_300)
        )
        .unwrap());
        assert!(!has_matching_proxy_usage_log(
            &conn,
            &key("claude", "claude-sonnet-5", tokens, 1_000 + 601)
        )
        .unwrap());
        assert!(!has_matching_proxy_usage_log(
            &conn,
            &key("claude", "claude-opus-5", tokens, 1_000)
        )
        .unwrap());
        assert!(!has_matching_proxy_usage_log(
            &conn,
            &key("claude", "claude-sonnet-5", (101, 50, 10, 0), 1_000)
        )
        .unwrap());
        assert!(!has_matching_proxy_usage_log(
            &conn,
            &key("codex", "claude-sonnet-5", tokens, 1_000)
        )
        .unwrap());
    }

    #[test]
    fn total_input_proxy_rows_match_fresh_input_session_events() {
        let db = Database::memory().unwrap();
        let conn = db.conn.lock().unwrap();
        // proxy 行的输入含缓存（TOTAL 口径）：1000 = 新增 700 + 命中 200 + 写入 100
        insert_row(
            &conn,
            "proxy-codex",
            "codex",
            "proxy",
            "gpt-5.6-sol",
            (1_000, 40, 200, 100),
            INPUT_TOKEN_SEMANTICS_TOTAL,
            5_000,
        );
        assert!(has_matching_proxy_usage_log(
            &conn,
            &key("codex", "gpt-5.6-sol", (700, 40, 200, 100), 5_010)
        )
        .unwrap());
        // 会话侧没记缓存写入时，缓存写入算在输入里
        assert!(has_matching_proxy_usage_log(
            &conn,
            &key("codex", "gpt-5.6-sol", (800, 40, 200, 0), 5_010)
        )
        .unwrap());
        assert!(!has_matching_proxy_usage_log(
            &conn,
            &key("codex", "gpt-5.6-sol", (701, 40, 200, 100), 5_010)
        )
        .unwrap());
    }

    #[test]
    fn claude_desktop_proxy_rows_match_claude_sessions() {
        let db = Database::memory().unwrap();
        let conn = db.conn.lock().unwrap();
        insert_row(
            &conn,
            "proxy-desktop",
            "claude-desktop",
            "proxy",
            "claude-sonnet-5",
            (10, 5, 0, 0),
            INPUT_TOKEN_SEMANTICS_FRESH,
            9_000,
        );
        assert!(has_matching_proxy_usage_log(
            &conn,
            &key("claude", "claude-sonnet-5", (10, 5, 0, 0), 9_000)
        )
        .unwrap());
    }

    #[test]
    fn grok_turns_are_skipped_while_routing_was_active() {
        let db = Database::memory().unwrap();
        let conn = db.conn.lock().unwrap();
        insert_row(
            &conn,
            "proxy-grok",
            "grokbuild",
            "proxy",
            "grok-4.7",
            (1, 1, 0, 0),
            INPUT_TOKEN_SEMANTICS_FRESH,
            20_000,
        );
        assert!(has_recent_grokbuild_proxy_activity(&conn, 20_500).unwrap());
        assert!(!has_recent_grokbuild_proxy_activity(&conn, 20_000 + 601).unwrap());
    }

    #[test]
    fn round_cache_skips_queries_for_events_after_the_last_proxy_row() {
        let db = Database::memory().unwrap();
        {
            let conn = db.conn.lock().unwrap();
            insert_row(
                &conn,
                "proxy-1",
                "claude",
                "proxy",
                "m",
                (1, 1, 0, 0),
                INPUT_TOKEN_SEMANTICS_FRESH,
                1_000,
            );
        }
        let _round = begin_round(&db);
        assert!(proxy_rows_possible_near(1_500));
        assert!(!proxy_rows_possible_near(1_000 + 601));
    }

    #[test]
    fn cleanup_removes_session_rows_that_duplicate_proxy_rows() {
        let db = Database::memory().unwrap();
        let conn = db.conn.lock().unwrap();
        insert_row(
            &conn,
            "proxy-1",
            "claude-desktop",
            "proxy",
            "m",
            (10, 5, 0, 0),
            INPUT_TOKEN_SEMANTICS_FRESH,
            1_000,
        );
        insert_row(
            &conn,
            "session-dup",
            "claude",
            "session_log",
            "m",
            (10, 5, 0, 0),
            INPUT_TOKEN_SEMANTICS_FRESH,
            1_100,
        );
        insert_row(
            &conn,
            "session-own",
            "claude",
            "session_log",
            "m",
            (11, 5, 0, 0),
            INPUT_TOKEN_SEMANTICS_FRESH,
            1_100,
        );
        insert_row(
            &conn,
            "proxy-grok",
            "grokbuild",
            "proxy",
            "g",
            (1, 1, 0, 0),
            INPUT_TOKEN_SEMANTICS_FRESH,
            50_000,
        );
        insert_row(
            &conn,
            "grok-turn",
            "grokbuild",
            "grok_session",
            "g",
            (99, 9, 0, 0),
            INPUT_TOKEN_SEMANTICS_FRESH,
            50_100,
        );

        assert_eq!(delete_session_rows_duplicating_proxy(&conn).unwrap(), 2);

        let remaining: Vec<String> = conn
            .prepare("SELECT request_id FROM proxy_request_logs ORDER BY request_id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(remaining, vec!["proxy-1", "proxy-grok", "session-own"]);
    }
}
