use serde::Serialize;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Serialize)]
pub struct InitErrorPayload {
    pub path: String,
    pub error: String,
    /// 错误类别。`Some("db_version_too_new")` 表示数据库版本过新（应用过旧），
    /// 前端据此展示「升级应用」恢复界面而非直接退出。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// 磁盘上数据库的 user_version（数据库版本过新时填充）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_version: Option<i32>,
    /// 当前应用支持的 SCHEMA_VERSION（数据库版本过新时填充）。
    /// 当升级到最新版后 db_version 仍 > supported_version，说明可能由第三方客户端创建。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_version: Option<i32>,
}

static INIT_ERROR: OnceLock<RwLock<Option<InitErrorPayload>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<InitErrorPayload>> {
    INIT_ERROR.get_or_init(|| RwLock::new(None))
}

pub fn set_init_error(payload: InitErrorPayload) {
    #[allow(clippy::unwrap_used)]
    if let Ok(mut guard) = cell().write() {
        *guard = Some(payload);
    }
}

pub fn get_init_error() -> Option<InitErrorPayload> {
    cell().read().ok()?.clone()
}

// ============================================================
// 迁移结果状态
// ============================================================

static MIGRATION_SUCCESS: OnceLock<RwLock<bool>> = OnceLock::new();

fn migration_cell() -> &'static RwLock<bool> {
    MIGRATION_SUCCESS.get_or_init(|| RwLock::new(false))
}

pub fn set_migration_success() {
    if let Ok(mut guard) = migration_cell().write() {
        *guard = true;
    }
}

/// 获取并消费迁移成功状态（只返回一次 true，之后返回 false）
pub fn take_migration_success() -> bool {
    if let Ok(mut guard) = migration_cell().write() {
        let val = *guard;
        *guard = false;
        val
    } else {
        false
    }
}

// ============================================================
// Skills SSOT 迁移结果状态
// ============================================================

#[derive(Debug, Clone, Serialize)]
pub struct SkillsMigrationPayload {
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

static SKILLS_MIGRATION_RESULT: OnceLock<RwLock<Option<SkillsMigrationPayload>>> = OnceLock::new();

fn skills_migration_cell() -> &'static RwLock<Option<SkillsMigrationPayload>> {
    SKILLS_MIGRATION_RESULT.get_or_init(|| RwLock::new(None))
}

pub fn set_skills_migration_result(count: usize) {
    if let Ok(mut guard) = skills_migration_cell().write() {
        *guard = Some(SkillsMigrationPayload { count, error: None });
    }
}

pub fn set_skills_migration_error(error: String) {
    if let Ok(mut guard) = skills_migration_cell().write() {
        *guard = Some(SkillsMigrationPayload {
            count: 0,
            error: Some(error),
        });
    }
}

/// 获取并消费 Skills 迁移结果（只返回一次 Some，之后返回 None）
pub fn take_skills_migration_result() -> Option<SkillsMigrationPayload> {
    if let Ok(mut guard) = skills_migration_cell().write() {
        guard.take()
    } else {
        None
    }
}

// ============================================================
// 首次启动从上游 CC Switch 导入的结果
// ============================================================

static UPSTREAM_IMPORT: OnceLock<RwLock<Option<crate::upstream_import::ImportRecord>>> =
    OnceLock::new();

fn upstream_import_cell() -> &'static RwLock<Option<crate::upstream_import::ImportRecord>> {
    UPSTREAM_IMPORT.get_or_init(|| RwLock::new(None))
}

pub fn set_upstream_import(record: crate::upstream_import::ImportRecord) {
    if let Ok(mut guard) = upstream_import_cell().write() {
        *guard = Some(record);
    }
}

/// 获取并消费本次启动的导入结果（只返回一次 Some）
pub fn take_upstream_import() -> Option<crate::upstream_import::ImportRecord> {
    upstream_import_cell().write().ok()?.take()
}

// ============================================================
// 升级时当作手改价导出到 model-pricing.json 的内置模型
// ============================================================

static EXPORTED_BUILTIN_PRICES: OnceLock<RwLock<Vec<String>>> = OnceLock::new();

fn exported_builtin_prices_cell() -> &'static RwLock<Vec<String>> {
    EXPORTED_BUILTIN_PRICES.get_or_init(|| RwLock::new(Vec::new()))
}

/// 记下 v21 迁移导出的模型，前端提示一次：判断是否手改过只能对照已知的内置价，
/// 用户要能看到导出了哪些，不是自己改的可以删掉。
pub fn add_exported_builtin_prices(model_ids: &[String]) {
    if let Ok(mut guard) = exported_builtin_prices_cell().write() {
        guard.extend(model_ids.iter().cloned());
    }
}

/// 取走记下的模型（之后返回空）
pub fn take_exported_builtin_prices() -> Vec<String> {
    exported_builtin_prices_cell()
        .write()
        .map(|mut guard| std::mem::take(&mut *guard))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_error_roundtrip() {
        let payload = InitErrorPayload {
            path: "/tmp/config.json".into(),
            error: "broken json".into(),
            kind: None,
            db_version: None,
            supported_version: None,
        };
        set_init_error(payload.clone());
        let got = get_init_error().expect("should get payload back");
        assert_eq!(got.path, payload.path);
        assert_eq!(got.error, payload.error);
    }

    #[test]
    #[serial_test::serial]
    fn exported_builtin_prices_are_taken_once() {
        add_exported_builtin_prices(&["claude-opus-4-8".to_string()]);
        add_exported_builtin_prices(&["glm-5".to_string()]);
        let taken = take_exported_builtin_prices();
        assert!(taken.contains(&"claude-opus-4-8".to_string()), "{taken:?}");
        assert!(taken.contains(&"glm-5".to_string()), "{taken:?}");
        assert!(take_exported_builtin_prices().is_empty());
    }
}
