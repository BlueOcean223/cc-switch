//! 首次启动时从上游 CC Switch 导入一次数据。
//!
//! ccs-lite 和上游 CC Switch 可以同时安装，各用各的数据目录（ccs-lite 是
//! `~/.ccs-lite`，上游是 `~/.cc-switch` 或上游设置里的自定义目录）。ccs-lite 数据目录里
//! 还没有数据库、设备目录里也没有导入记录时，从上游复制一份：
//!
//! - 数据库：只读打开上游库，`VACUUM INTO` 得到一致的副本（上游可能正在运行，不影响它），
//!   之后由 `Database::init` 按正常流程迁移；
//! - 文件：`model-pricing.json`、`skills/`、Codex 托管账号的两个文件、上游的设备设置
//!   `settings.json`；
//! - 不复制：`live-state.json`（上游的待执行操作）、`backups/`、`logs/`、`crash.log`、
//!   `skill-backups/`。
//!
//! 结果写进设备目录的 `upstream-import.json`，之后不再读写上游目录。

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use crate::database::lineage;
use crate::error::AppError;

/// 上游 CC Switch 的 Tauri identifier，它的 `app_paths.json` 在这个目录下。
const UPSTREAM_IDENTIFIER: &str = "com.ccswitch.desktop";
/// 上游 CC Switch 在用户目录下的默认目录名（也是它的设备目录）。
const UPSTREAM_DIR_NAME: &str = ".cc-switch";
const UPSTREAM_OVERRIDE_KEY: &str = "app_config_dir_override";
const DB_FILE_NAME: &str = "cc-switch.db";
const RECORD_FILE_NAME: &str = "upstream-import.json";

/// 数据目录里按原名复制的文件
const DATA_FILES: &[&str] = &[
    "model-pricing.json",
    "codex_oauth_auth.json",
    "codex_managed_oauth_live_auth.json",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportResult {
    /// 已导入
    Imported,
    /// 没找到上游数据，按全新安装
    NoUpstream,
    /// 上游数据是 ccs-lite 打不开的版本，没有导入
    Skipped,
    /// 导入失败，按全新安装
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRecord {
    pub result: ImportResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    pub at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_user_version: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 导入涉及的路径。单独成结构是为了测试能把它们都指向临时目录。
#[derive(Debug, Clone)]
pub(crate) struct ImportPaths {
    /// 用户目录（找默认的 `~/.cc-switch`）
    pub home: PathBuf,
    /// 上游的 Tauri 应用数据目录（`app_paths.json` 所在目录）
    pub upstream_app_data: Option<PathBuf>,
    /// Windows 上 v3.10.3 可能把数据放在 `HOME` 环境变量指向的目录下
    pub legacy_home: Option<PathBuf>,
    /// ccs-lite 的数据目录
    pub data_dir: PathBuf,
    /// ccs-lite 的设备目录
    pub device_dir: PathBuf,
}

impl ImportPaths {
    fn for_this_device() -> Self {
        let home = crate::config::get_home_dir();
        let testing = std::env::var_os("CC_SWITCH_TEST_HOME").is_some();
        // 测试 home 覆盖生效时不读真实的应用数据目录，避免测试读到本机的上游设置
        let upstream_app_data = if testing {
            None
        } else {
            dirs::data_dir().map(|dir| dir.join(UPSTREAM_IDENTIFIER))
        };
        Self {
            upstream_app_data,
            legacy_home: if testing { None } else { windows_legacy_home() },
            data_dir: crate::config::get_app_config_dir(),
            device_dir: crate::config::get_device_dir(),
            home,
        }
    }
}

/// Windows：v3.10.3 曾按 `HOME` 环境变量（可能由 Git/MSYS 注入）定位数据目录。
#[cfg(windows)]
fn windows_legacy_home() -> Option<PathBuf> {
    let raw = std::env::var("HOME").ok()?;
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

#[cfg(not(windows))]
fn windows_legacy_home() -> Option<PathBuf> {
    None
}

/// 展开 `~`、`~/`、`~\`，规则与 `app_store::resolve_path` 相同。
fn expand_home(raw: &str, home: &Path) -> PathBuf {
    if raw == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        return home.join(rest);
    }
    PathBuf::from(raw)
}

fn has_db(dir: &Path) -> bool {
    dir.join(DB_FILE_NAME).is_file()
}

/// 找上游 CC Switch 的数据目录：先看上游 `app_paths.json` 里的自定义目录，再看
/// `~/.cc-switch`，Windows 上再看 `HOME` 环境变量下的 `.cc-switch`。目录里有
/// `cc-switch.db` 才算找到。
pub(crate) fn locate_upstream_data_dir(paths: &ImportPaths) -> Option<PathBuf> {
    if let Some(app_data) = &paths.upstream_app_data {
        if let Some(dir) = read_upstream_override(&app_data.join("app_paths.json"), &paths.home) {
            if has_db(&dir) {
                return Some(dir);
            }
            log::info!(
                "上游 CC Switch 的自定义数据目录 {} 里没有数据库，改看默认位置",
                dir.display()
            );
        }
    }
    let default_dir = paths.home.join(UPSTREAM_DIR_NAME);
    if has_db(&default_dir) {
        return Some(default_dir);
    }
    let legacy_dir = paths.legacy_home.as_ref()?.join(UPSTREAM_DIR_NAME);
    has_db(&legacy_dir).then_some(legacy_dir)
}

fn read_upstream_override(store_path: &Path, home: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(store_path).ok()?;
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(e) => {
            log::warn!("解析上游 {} 失败: {e}", store_path.display());
            return None;
        }
    };
    let raw = value.get(UPSTREAM_OVERRIDE_KEY)?.as_str()?.trim();
    if raw.is_empty() {
        return None;
    }
    let dir = expand_home(raw, home);
    dir.is_dir().then_some(dir)
}

fn record_path(device_dir: &Path) -> PathBuf {
    device_dir.join(RECORD_FILE_NAME)
}

#[cfg(test)]
fn read_record(device_dir: &Path) -> Option<ImportRecord> {
    let content = fs::read_to_string(record_path(device_dir)).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_record(device_dir: &Path, record: &ImportRecord) -> Result<(), AppError> {
    let json =
        serde_json::to_vec_pretty(record).map_err(|e| AppError::JsonSerialize { source: e })?;
    crate::config::atomic_write(&record_path(device_dir), &json)
}

/// 应用启动时调用：需要时执行导入。返回本次启动写下的导入记录（没有执行导入时返回 `None`）。
pub fn run_on_first_launch() -> Option<ImportRecord> {
    let paths = ImportPaths::for_this_device();
    let record = run_with(&paths)?;
    if record.result == ImportResult::Imported {
        // 设备设置可能在导入前已被读进缓存
        if let Err(e) = crate::settings::reload_settings() {
            log::warn!("导入后重新加载设置失败: {e}");
        }
    }
    Some(record)
}

pub(crate) fn run_with(paths: &ImportPaths) -> Option<ImportRecord> {
    if has_db(&paths.data_dir) || record_path(&paths.device_dir).exists() {
        return None;
    }
    let record = import(paths);
    match record.result {
        ImportResult::Imported => log::info!(
            "已从上游 CC Switch 导入数据: {}",
            record.from.as_deref().unwrap_or("")
        ),
        ImportResult::NoUpstream => log::info!("没有找到上游 CC Switch 的数据，按全新安装启动"),
        ImportResult::Skipped | ImportResult::Failed => log::warn!(
            "没有导入上游 CC Switch 的数据（{:?}）: {}",
            record.result,
            record.error.as_deref().unwrap_or("")
        ),
    }
    if let Err(e) = fs::create_dir_all(&paths.device_dir)
        .map_err(|e| AppError::io(&paths.device_dir, e))
        .and_then(|_| write_record(&paths.device_dir, &record))
    {
        log::warn!("写入上游导入记录失败: {e}");
    }
    Some(record)
}

fn import(paths: &ImportPaths) -> ImportRecord {
    let now = chrono::Utc::now().timestamp();
    let Some(upstream) = locate_upstream_data_dir(paths) else {
        return ImportRecord {
            result: ImportResult::NoUpstream,
            from: None,
            at: now,
            upstream_user_version: None,
            error: None,
        };
    };
    let from = Some(upstream.display().to_string());
    let failed = |error: String, version: Option<i32>| ImportRecord {
        result: ImportResult::Failed,
        from: from.clone(),
        at: now,
        upstream_user_version: version,
        error: Some(error),
    };

    // 防御性检查：`run_with` 只在 ccs-lite 数据目录里还没有数据库时才导入，找到的上游目录里
    // 一定有数据库，按现在的调用条件走不到这里。导入会覆盖数据目录里的文件，留着这个检查，
    // 免得以后调用条件变了，拿上游目录覆盖它自己。
    if same_dir(&upstream, &paths.data_dir) {
        return failed("上游数据目录与 ccs-lite 数据目录相同".to_string(), None);
    }

    let (lineage, version) = match inspect_upstream_db(&upstream.join(DB_FILE_NAME)) {
        Ok(found) => found,
        Err(e) => return failed(e.to_string(), None),
    };
    if !lineage.is_supported() {
        return ImportRecord {
            result: ImportResult::Skipped,
            from,
            at: now,
            upstream_user_version: Some(version),
            error: Some(lineage::unsupported_error(lineage).to_string()),
        };
    }

    if let Err(e) = copy_database(&upstream.join(DB_FILE_NAME), &paths.data_dir) {
        return failed(e.to_string(), Some(version));
    }

    for name in DATA_FILES {
        copy_file_if_present(&upstream.join(name), &paths.data_dir.join(name));
    }
    let skills = upstream.join("skills");
    if skills.is_dir() {
        if let Err(e) = copy_dir(&skills, &paths.data_dir.join("skills")) {
            log::warn!("复制上游技能目录失败: {e}");
        }
    }
    // 上游的设备设置固定在 ~/.cc-switch，不跟随它的数据目录覆盖
    let settings_dest = paths.device_dir.join("settings.json");
    if !settings_dest.exists() {
        copy_file_if_present(
            &paths.home.join(UPSTREAM_DIR_NAME).join("settings.json"),
            &settings_dest,
        );
    }

    ImportRecord {
        result: ImportResult::Imported,
        from,
        at: now,
        upstream_user_version: Some(version),
        error: None,
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn inspect_upstream_db(db_path: &Path) -> Result<(lineage::Lineage, i32), AppError> {
    let conn = open_read_only(db_path)?;
    let lineage = lineage::classify(&conn)?;
    let version = crate::database::Database::get_user_version(&conn)?;
    Ok((lineage, version))
}

fn open_read_only(db_path: &Path) -> Result<Connection, AppError> {
    Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| AppError::Database(format!("只读打开上游数据库失败: {e}")))
}

/// `VACUUM INTO` 到 `<数据目录>/cc-switch.db.importing`，成功后改名为 `cc-switch.db`。
fn copy_database(source: &Path, data_dir: &Path) -> Result<(), AppError> {
    fs::create_dir_all(data_dir).map_err(|e| AppError::io(data_dir, e))?;
    let staging = data_dir.join(format!("{DB_FILE_NAME}.importing"));
    let _ = fs::remove_file(&staging);
    let result = (|| {
        let conn = open_read_only(source)?;
        conn.execute("VACUUM INTO ?1", [staging.to_string_lossy().as_ref()])
            .map_err(|e| AppError::Database(format!("复制上游数据库失败: {e}")))?;
        drop(conn);
        let target = data_dir.join(DB_FILE_NAME);
        fs::rename(&staging, &target).map_err(|e| AppError::io(&target, e))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staging);
    }
    result
}

fn copy_file_if_present(source: &Path, dest: &Path) {
    if !source.is_file() {
        return;
    }
    if let Some(parent) = dest.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            log::warn!("创建目录 {} 失败: {e}", parent.display());
            return;
        }
    }
    if let Err(e) = fs::copy(source, dest) {
        log::warn!("复制 {} 到 {} 失败: {e}", source.display(), dest.display());
    }
}

/// 递归复制目录。符号链接按链接本身复制（Unix），不跟随，避免链接环和越出目录。
fn copy_dir(source: &Path, dest: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dest).map_err(|e| AppError::io(dest, e))?;
    for entry in fs::read_dir(source).map_err(|e| AppError::io(source, e))? {
        let entry = entry.map_err(|e| AppError::io(source, e))?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| AppError::io(&path, e))?;
        if file_type.is_symlink() {
            #[cfg(unix)]
            {
                let link = fs::read_link(&path).map_err(|e| AppError::io(&path, e))?;
                if let Err(e) = std::os::unix::fs::symlink(&link, &target) {
                    log::warn!("复制符号链接 {} 失败: {e}", path.display());
                }
            }
            #[cfg(not(unix))]
            log::warn!("跳过符号链接 {}", path.display());
        } else if file_type.is_dir() {
            copy_dir(&path, &target)?;
        } else if let Err(e) = fs::copy(&path, &target) {
            log::warn!("复制 {} 失败: {e}", path.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::time::SystemTime;

    struct Fixture {
        _root: tempfile::TempDir,
        paths: ImportPaths,
    }

    fn fixture() -> Fixture {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let paths = ImportPaths {
            upstream_app_data: Some(root.path().join("app-data").join(UPSTREAM_IDENTIFIER)),
            legacy_home: None,
            data_dir: home.join(crate::config::APP_DIR_NAME),
            device_dir: home.join(crate::config::APP_DIR_NAME),
            home,
        };
        Fixture { _root: root, paths }
    }

    fn make_upstream(dir: &Path, version: i32) {
        fs::create_dir_all(dir.join("skills").join("demo")).unwrap();
        let conn = Connection::open(dir.join(DB_FILE_NAME)).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE providers (id TEXT, app_type TEXT, name TEXT);
             INSERT INTO providers VALUES ('p1', 'claude', 'Upstream Provider');
             CREATE TABLE proxy_config (id INTEGER);
             PRAGMA user_version = {version};"
        ))
        .unwrap();
        fs::write(dir.join("model-pricing.json"), b"{\"version\":1}").unwrap();
        fs::write(dir.join("codex_oauth_auth.json"), b"{\"accounts\":[]}").unwrap();
        fs::write(dir.join("live-state.json"), b"{}").unwrap();
        fs::create_dir_all(dir.join("backups")).unwrap();
        fs::write(dir.join("skills").join("demo").join("SKILL.md"), b"# demo").unwrap();
    }

    /// 目录下所有文件的 (相对路径 → (内容, mtime))
    fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
        let mut out = BTreeMap::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            for entry in fs::read_dir(&current).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let meta = fs::metadata(&path).unwrap();
                    out.insert(
                        path.strip_prefix(dir).unwrap().to_path_buf(),
                        (fs::read(&path).unwrap(), meta.modified().unwrap()),
                    );
                }
            }
        }
        out
    }

    #[test]
    fn imports_upstream_data_once_without_touching_it() {
        let fx = fixture();
        let upstream = fx.paths.home.join(UPSTREAM_DIR_NAME);
        make_upstream(&upstream, 19);
        fs::write(upstream.join("settings.json"), b"{\"language\":\"zh\"}").unwrap();
        let before = snapshot(&upstream);

        let record = run_with(&fx.paths).expect("first launch imports");
        assert_eq!(record.result, ImportResult::Imported, "{record:?}");
        assert_eq!(record.upstream_user_version, Some(19));

        let data = &fx.paths.data_dir;
        // 查完就关闭连接：Windows 上打开着的文件删不掉，后面要删数据库
        let name: String = Connection::open(data.join(DB_FILE_NAME))
            .unwrap()
            .query_row("SELECT name FROM providers", [], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "Upstream Provider");
        assert!(data.join("model-pricing.json").is_file());
        assert!(data.join("codex_oauth_auth.json").is_file());
        assert!(data.join("skills/demo/SKILL.md").is_file());
        assert!(fx.paths.device_dir.join("settings.json").is_file());
        assert!(!data.join("live-state.json").exists());
        assert!(!data.join("backups").exists());
        assert!(!data.join("cc-switch.db.importing").exists());

        assert_eq!(
            snapshot(&upstream),
            before,
            "upstream files must not change"
        );

        // 第二次启动：已经有数据库和记录，不再导入
        fs::remove_file(data.join(DB_FILE_NAME)).unwrap();
        assert!(run_with(&fx.paths).is_none());
    }

    #[test]
    fn newer_upstream_database_is_not_imported() {
        let fx = fixture();
        make_upstream(&fx.paths.home.join(UPSTREAM_DIR_NAME), 22);

        let record = run_with(&fx.paths).unwrap();

        assert_eq!(record.result, ImportResult::Skipped);
        assert!(!fx.paths.data_dir.join(DB_FILE_NAME).exists());
        assert!(read_record(&fx.paths.device_dir).is_some());
    }

    #[test]
    fn missing_upstream_means_a_fresh_install() {
        let fx = fixture();

        let record = run_with(&fx.paths).unwrap();

        assert_eq!(record.result, ImportResult::NoUpstream);
        assert!(!fx.paths.data_dir.join(DB_FILE_NAME).exists());
        assert!(run_with(&fx.paths).is_none(), "record prevents a retry");
    }

    #[test]
    fn corrupt_upstream_database_fails_without_leaving_a_partial_copy() {
        let fx = fixture();
        let upstream = fx.paths.home.join(UPSTREAM_DIR_NAME);
        fs::create_dir_all(&upstream).unwrap();
        fs::write(upstream.join(DB_FILE_NAME), b"not a sqlite database at all").unwrap();

        let record = run_with(&fx.paths).unwrap();

        assert_eq!(record.result, ImportResult::Failed, "{record:?}");
        assert!(!fx.paths.data_dir.join(DB_FILE_NAME).exists());
        assert!(!fx.paths.data_dir.join("cc-switch.db.importing").exists());
    }

    #[test]
    fn upstream_custom_data_dir_is_used_when_it_exists() {
        let fx = fixture();
        let custom = fx.paths.home.join("Dropbox").join("cc-switch-data");
        make_upstream(&custom, 20);
        make_upstream(&fx.paths.home.join(UPSTREAM_DIR_NAME), 19);
        let app_data = fx.paths.upstream_app_data.clone().unwrap();
        fs::create_dir_all(&app_data).unwrap();
        fs::write(
            app_data.join("app_paths.json"),
            br#"{"app_config_dir_override":"~/Dropbox/cc-switch-data"}"#,
        )
        .unwrap();

        assert_eq!(locate_upstream_data_dir(&fx.paths), Some(custom.clone()));

        // 覆盖目录不存在 → 回落到默认目录
        fs::write(
            app_data.join("app_paths.json"),
            br#"{"app_config_dir_override":"~/missing"}"#,
        )
        .unwrap();
        assert_eq!(
            locate_upstream_data_dir(&fx.paths),
            Some(fx.paths.home.join(UPSTREAM_DIR_NAME))
        );

        // 没有覆盖 → 默认目录
        fs::write(app_data.join("app_paths.json"), b"{}").unwrap();
        assert_eq!(
            locate_upstream_data_dir(&fx.paths),
            Some(fx.paths.home.join(UPSTREAM_DIR_NAME))
        );
    }

    #[test]
    fn legacy_home_dir_is_found_when_the_default_is_missing() {
        let mut fx = fixture();
        let legacy_home = fx.paths.home.join("msys-home");
        make_upstream(&legacy_home.join(UPSTREAM_DIR_NAME), 18);
        fx.paths.legacy_home = Some(legacy_home.clone());

        assert_eq!(
            locate_upstream_data_dir(&fx.paths),
            Some(legacy_home.join(UPSTREAM_DIR_NAME))
        );
    }
}
