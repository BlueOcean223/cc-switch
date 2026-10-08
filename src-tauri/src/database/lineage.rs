//! 数据库血统：区分 ccs-lite 自己的库、可以升级的上游 CC Switch 库和不能打开的库。
//!
//! ccs-lite 从上游 CC Switch 分出来时沿用了上游的 `user_version` 编号，v21 起两边的
//! schema 不再相同（v21 删掉了上游还在用的表）。上游以后发布的 v21 及更高版本和
//! ccs-lite 的同号版本不是一回事，所以光看 `user_version` 不够，还要看
//! `PRAGMA application_id`：ccs-lite 迁移完成后把它设成 [`FORK_APPLICATION_ID`]。
//!
//! | application_id | user_version | 判断 |
//! |---|---|---|
//! | FORK | 任意 | ccs-lite 自己的库 |
//! | 0 | ≤ 20 | 上游库，按现有迁移升级，完成后打 FORK 标记 |
//! | 0 | 21，没有 `proxy_config` 表、`proxy_request_logs` 有 `native_cost` 列 | 加血统标记之前的 ccs-lite 开发库 |
//! | 0 | ≥ 21（不满足上一行） | 更新版本的上游库，拒绝 |
//! | 其他 | 任意 | 不是 CC Switch 系的库，拒绝 |

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use super::Database;
use crate::error::AppError;

/// ccs-lite 数据库的 `PRAGMA application_id`，即 ASCII "CCSL"。
pub(crate) const FORK_APPLICATION_ID: i32 = 0x4343_534C;

/// 上游 CC Switch 与 ccs-lite 共用编号的最后一个 schema 版本。
pub(crate) const LAST_SHARED_UPSTREAM_VERSION: i32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lineage {
    /// 带 FORK 标记的 ccs-lite 库
    Fork,
    /// 加血统标记之前的 ccs-lite 开发库（v21，无标记）
    UnmarkedFork,
    /// 上游 v20 及以前的库（含空库），可以升级
    Upstream,
    /// 上游更新版本的库，ccs-lite 不能打开
    NewerUpstream(i32),
    /// application_id 不认识
    Foreign(i32),
}

impl Lineage {
    pub(crate) fn is_supported(self) -> bool {
        matches!(self, Self::Fork | Self::UnmarkedFork | Self::Upstream)
    }

    /// 是 ccs-lite 自己的库（有标记或开发期的无标记库）
    pub(crate) fn is_fork(self) -> bool {
        matches!(self, Self::Fork | Self::UnmarkedFork)
    }
}

pub(crate) fn application_id(conn: &Connection) -> Result<i32, AppError> {
    conn.query_row("PRAGMA application_id;", [], |row| row.get(0))
        .map_err(|e| AppError::Database(format!("读取 application_id 失败: {e}")))
}

pub(crate) fn mark_fork(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(&format!("PRAGMA application_id = {FORK_APPLICATION_ID};"))
        .map_err(|e| AppError::Database(format!("写入 application_id 失败: {e}")))
}

pub(crate) fn classify(conn: &Connection) -> Result<Lineage, AppError> {
    let app_id = application_id(conn)?;
    if app_id == FORK_APPLICATION_ID {
        return Ok(Lineage::Fork);
    }
    if app_id != 0 {
        return Ok(Lineage::Foreign(app_id));
    }
    let version = Database::get_user_version(conn)?;
    if version <= LAST_SHARED_UPSTREAM_VERSION {
        return Ok(Lineage::Upstream);
    }
    if version == 21
        && !Database::table_exists(conn, "proxy_config")?
        && Database::table_exists(conn, "proxy_request_logs")?
        && Database::has_column(conn, "proxy_request_logs", "native_cost")?
    {
        return Ok(Lineage::UnmarkedFork);
    }
    Ok(Lineage::NewerUpstream(version))
}

/// 只读打开 `db_path` 判断血统。文件不存在返回 `None`。
pub(crate) fn classify_file(db_path: &Path) -> Result<Option<Lineage>, AppError> {
    if !db_path.exists() {
        return Ok(None);
    }
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    classify(&conn).map(Some)
}

pub(crate) fn unsupported_error(lineage: Lineage) -> AppError {
    match lineage {
        Lineage::NewerUpstream(version) => AppError::localized(
            "database.lineage.newerUpstream",
            format!(
                "这是更新版本的上游 CC Switch 数据（数据库版本 v{version}），ccs-lite 不能打开。"
            ),
            format!(
                "This is data from a newer upstream CC Switch (database v{version}); ccs-lite cannot open it."
            ),
        ),
        Lineage::Foreign(app_id) => AppError::localized(
            "database.lineage.foreign",
            format!("这不是 CC Switch 或 ccs-lite 的数据库（application_id = {app_id:#x}）。"),
            format!("This is not a CC Switch or ccs-lite database (application_id = {app_id:#x})."),
        ),
        _ => AppError::Database(format!("unexpected supported lineage {lineage:?}")),
    }
}

/// 不能打开的库返回错误；能打开的返回血统。
pub(crate) fn ensure_supported(conn: &Connection) -> Result<Lineage, AppError> {
    let lineage = classify(conn)?;
    if lineage.is_supported() {
        Ok(lineage)
    } else {
        Err(unsupported_error(lineage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn_with(app_id: i32, version: i32) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!(
            "PRAGMA application_id = {app_id}; PRAGMA user_version = {version};"
        ))
        .unwrap();
        conn
    }

    #[test]
    fn classifies_each_row_of_the_lineage_table() {
        assert_eq!(
            classify(&conn_with(FORK_APPLICATION_ID, 30)).unwrap(),
            Lineage::Fork
        );
        assert_eq!(classify(&conn_with(0, 0)).unwrap(), Lineage::Upstream);
        assert_eq!(classify(&conn_with(0, 20)).unwrap(), Lineage::Upstream);
        assert_eq!(
            classify(&conn_with(0, 22)).unwrap(),
            Lineage::NewerUpstream(22)
        );
        assert_eq!(classify(&conn_with(7, 3)).unwrap(), Lineage::Foreign(7));

        let upstream_v21 = conn_with(0, 21);
        upstream_v21
            .execute_batch(
                "CREATE TABLE proxy_config (id INTEGER);
                 CREATE TABLE proxy_request_logs (request_id TEXT, native_cost INTEGER);",
            )
            .unwrap();
        assert_eq!(classify(&upstream_v21).unwrap(), Lineage::NewerUpstream(21));

        let dev_fork = conn_with(0, 21);
        dev_fork
            .execute_batch(
                "CREATE TABLE proxy_request_logs (request_id TEXT, native_cost INTEGER);",
            )
            .unwrap();
        assert_eq!(classify(&dev_fork).unwrap(), Lineage::UnmarkedFork);
    }

    #[test]
    fn mark_fork_sets_the_application_id() {
        let conn = conn_with(0, 20);
        mark_fork(&conn).unwrap();
        assert_eq!(application_id(&conn).unwrap(), FORK_APPLICATION_ID);
    }
}
