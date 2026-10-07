//! 当前供应商（指针）。
//!
//! 读当前供应商一律经 [`provider_id`]。直接读指针（`settings::get_effective_current_provider`、
//! `Database::get_current_provider`）只允许在这里和它们自己的定义处，由下面的测试把关。

use crate::app_config::AppType;
use crate::database::Database;
use crate::error::AppError;
use crate::provider::Provider;

/// 当前供应商的 id（验证过存在：本地记录优先，回落到 DB 的 `is_current`）。
pub fn provider_id(db: &Database, app: &AppType) -> Result<Option<String>, AppError> {
    crate::settings::get_effective_current_provider(db, app)
}

/// 当前供应商那一行。
pub fn provider(db: &Database, app: &AppType) -> Result<Option<Provider>, AppError> {
    match provider_id(db, app)? {
        Some(id) => db.get_provider_by_id(&id, app.as_str()),
        None => Ok(None),
    }
}

/// 设备本地记录的指针（不验证是否存在、不回落到 DB）。改供应商 key 时用：本地记录指着
/// 旧 key 才跟着改。
pub fn local_pointer(app: &AppType) -> Option<String> {
    crate::settings::get_current_provider(app)
}

/// 删除前检查：本地记录、DB 的 `is_current` 任何一处指着它，就算正在用。
pub fn is_referenced(db: &Database, app: &AppType, id: &str) -> Result<bool, AppError> {
    Ok(
        crate::settings::get_current_provider(app).as_deref() == Some(id)
            || db.get_current_provider(app.as_str())?.as_deref() == Some(id),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    /// 直接读指针的地方：定义处和这里。
    const ALLOWED: &[&str] = &[
        "settings.rs",
        "database/dao/providers.rs",
        "mode/current.rs",
    ];

    fn scan(dir: &Path, root: &Path, hits: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                scan(&path, root, hits);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if ALLOWED.contains(&relative.as_str()) {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            // 测试模块里造数据可以直接读写，只查生产代码：跳过顶层的
            // `#[cfg(test)] mod xxx { … }`（到下一个顶格的 `}` 为止）。
            let mut in_tests = false;
            let mut previous = "";
            for (index, line) in text.lines().enumerate() {
                if in_tests {
                    in_tests = line != "}";
                    previous = line;
                    continue;
                }
                if previous == "#[cfg(test)]" && line.starts_with("mod ") && line.ends_with('{') {
                    in_tests = true;
                    previous = line;
                    continue;
                }
                previous = line;
                if line.contains("get_effective_current_provider(")
                    || line.contains(".get_current_provider(")
                    || line.contains("settings::get_current_provider(")
                {
                    hits.push(format!("{relative}:{}: {}", index + 1, line.trim()));
                }
            }
        }
    }

    #[test]
    fn current_provider_is_read_through_provider_id() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut hits = Vec::new();
        scan(&root, &root, &mut hits);
        assert!(
            hits.is_empty(),
            "读当前供应商要经 mode::current::provider_id:\n{}",
            hits.join("\n")
        );
    }
}
