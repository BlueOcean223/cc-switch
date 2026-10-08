//! `live-state.json`：这台设备上每个应用的客户端文件状态。
//!
//! - `written`：CC Switch 上次写进客户端文件、之后要按记录删掉的东西（Grok 的模型表）；
//! - `pending`：一次写客户端文件的操作在发布前写下的意图，按文件记录写前、写后的
//!   hash 和已备好的临时文件，崩溃后据此前滚或丢弃（`mode::operation`）。
//!
//! 不认识的字段读写时原样保留（包括旧版代理模式留下的 `mode`、`proxy_route`、`stack` 等）。
//!
//! 文件是设备本地的（0600，不同步），路径见 [`DeviceStore`]。

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::config::stage_write;
use crate::error::AppError;
use crate::live::engine::DeviceStore;

pub const STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveState {
    #[serde(default = "state_version")]
    pub version: u32,
    #[serde(default)]
    pub apps: BTreeMap<String, AppLiveState>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn state_version() -> u32 {
    STATE_VERSION
}

impl Default for LiveState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            apps: BTreeMap::new(),
            extra: Map::new(),
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// CC Switch 上次写进客户端文件、切走时要按记录删掉的东西。
///
/// 不能按 live 现在的内容去找：客户端自己会改。Grok 的 `/settings` 会把 `models.default`
/// 改成内置模型，按它找表就会漏删上一家的表；而 CC Switch 默认的表名 `grok-4.5` 正好是
/// 内置模型 ID，留下的表会覆盖内置模型，把官方请求连同第三方 Key 发到第三方地址。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Written {
    /// Grok Build `config.toml` 里 CC Switch 写的 `[model."<名称>"]` 表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppLiveState {
    /// 没有值：这台设备上还没有新版写过这个应用的文件（升级前旧版写的，按行推断）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub written: Option<Written>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<Pending>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl AppLiveState {
    fn is_empty(&self) -> bool {
        self.pending.is_none() && self.written.is_none() && self.extra.is_empty()
    }
}

/// 操作名。启动恢复只回放 [`op::REPLAYABLE`] 里的操作，其他操作名（上游 CC Switch 的
/// enter、attach、route 等）直接丢弃，见 `operation::recover`。
pub mod op {
    /// 切换供应商（会改指针）。
    pub const SWITCH: &str = "switch";
    /// 把当前供应商重新写进客户端文件（编辑、同步等）。
    pub const APPLY: &str = "apply";
    /// ccs-lite 自己会写、恢复时可以回放的操作。新增操作名要加进来。
    pub const REPLAYABLE: &[&str] = &[SWITCH, APPLY];
}

/// 一次操作的写前意图。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pending {
    pub op: String,
    pub files: Vec<PendingFile>,
    #[serde(default)]
    pub target: PendingTarget,
    /// 已经开始发布：换进第一个文件之前记下，所以只要有文件发布过它就一定在。发布过的
    /// 文件之后可能又被客户端改掉（Codex 刷新登录），单看文件内容就分不出发布开始过没有，
    /// 恢复时靠它决定前滚还是丢弃。
    #[serde(default, skip_serializing_if = "is_false")]
    pub published: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingFile {
    pub path: PathBuf,
    /// 写前内容的 hash；`None` 表示写前文件不存在。
    pub pre: Option<String>,
    /// 写后内容的 hash；`None` 表示这个操作要删掉它。
    #[serde(default)]
    pub planned: Option<String>,
    /// 已写好写后内容、等着 rename 的临时文件；删文件时没有。
    #[serde(default)]
    pub staged: Option<PathBuf>,
}

/// 文件都写完之后要落定的状态。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PendingTarget {
    /// 指针：切换成功后当前供应商是谁。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointer: Option<String>,
    /// 写入记录：有值时整体替换。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub written: Option<Written>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl PendingTarget {
    /// 只改指针（`None` 表示不改）。
    pub fn pointer(pointer: Option<String>) -> Self {
        Self {
            pointer,
            ..Self::default()
        }
    }
}

/// 状态文件是所有应用共用的，读改写要串行。
fn state_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

/// 读状态文件。不存在时是空状态；内容坏了就挪到一旁（`live-state.json.corrupt-<时间>`）
/// 从空状态开始：这是 CC Switch 自己的文件，里面只有未完成操作的意图。
pub fn load(store: &DeviceStore) -> Result<LiveState, AppError> {
    let path = store.state_path();
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(LiveState::default()),
        Err(source) => return Err(AppError::io(&path, source)),
    };
    match serde_json::from_slice::<LiveState>(&bytes) {
        Ok(state) => Ok(state),
        Err(err) => {
            let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
            let aside = path.with_file_name(format!("live-state.json.corrupt-{stamp}"));
            log::warn!(
                "live-state.json 无法解析（{err}），已移到 {} 并从空状态开始",
                aside.display()
            );
            fs::rename(&path, &aside).map_err(|source| AppError::io(&path, source))?;
            Ok(LiveState::default())
        }
    }
}

fn save(store: &DeviceStore, state: &LiveState) -> Result<(), AppError> {
    let bytes =
        serde_json::to_vec_pretty(state).map_err(|source| AppError::JsonSerialize { source })?;
    stage_write(&store.state_path(), &bytes, Some(0o600), true)?.commit()
}

/// 读改写状态文件。
pub fn update<R>(
    store: &DeviceStore,
    change: impl FnOnce(&mut LiveState) -> R,
) -> Result<R, AppError> {
    let _guard = state_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut state = load(store)?;
    let result = change(&mut state);
    state.apps.retain(|_, app| !app.is_empty());
    save(store, &state)?;
    Ok(result)
}

pub fn pending(store: &DeviceStore, app: &str) -> Result<Option<Pending>, AppError> {
    let _guard = state_lock().lock().unwrap_or_else(|e| e.into_inner());
    Ok(load(store)?
        .apps
        .get(app)
        .and_then(|state| state.pending.clone()))
}

pub fn set_pending(
    store: &DeviceStore,
    app: &str,
    pending: Option<Pending>,
) -> Result<(), AppError> {
    update(store, |state| {
        state.apps.entry(app.to_string()).or_default().pending = pending;
    })
}

/// 这个应用的写入记录；`None` 表示新版还没写过。
pub fn written(store: &DeviceStore, app: &str) -> Result<Option<Written>, AppError> {
    let _guard = state_lock().lock().unwrap_or_else(|e| e.into_inner());
    Ok(load(store)?
        .apps
        .get(app)
        .and_then(|state| state.written.clone()))
}

/// 有未完成操作的应用。
pub fn apps_with_pending(store: &DeviceStore) -> Result<Vec<String>, AppError> {
    let _guard = state_lock().lock().unwrap_or_else(|e| e.into_inner());
    Ok(load(store)?
        .apps
        .into_iter()
        .filter(|(_, state)| state.pending.is_some())
        .map(|(app, _)| app)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_pending() -> Pending {
        Pending {
            op: op::SWITCH.to_string(),
            files: vec![PendingFile {
                path: PathBuf::from("/tmp/settings.json"),
                pre: None,
                planned: Some("abc".to_string()),
                staged: Some(PathBuf::from("/tmp/settings.json.tmp.1")),
            }],
            target: PendingTarget::pointer(Some("p1".to_string())),
            published: false,
        }
    }

    #[test]
    fn pending_round_trips_and_clears() {
        let dir = tempfile::tempdir().unwrap();
        let store = DeviceStore::at(dir.path());

        assert_eq!(pending(&store, "claude").unwrap(), None);
        set_pending(&store, "claude", Some(sample_pending())).unwrap();
        assert_eq!(pending(&store, "claude").unwrap(), Some(sample_pending()));
        assert_eq!(
            apps_with_pending(&store).unwrap(),
            vec!["claude".to_string()]
        );

        set_pending(&store, "claude", None).unwrap();
        assert_eq!(pending(&store, "claude").unwrap(), None);
        let raw: Value = serde_json::from_slice(&fs::read(store.state_path()).unwrap()).unwrap();
        assert_eq!(raw, json!({"version": 1, "apps": {}}));
    }

    #[test]
    fn unknown_fields_survive_a_rewrite() {
        let dir = tempfile::tempdir().unwrap();
        let store = DeviceStore::at(dir.path());
        fs::write(
            store.state_path(),
            r#"{"version": 2, "future": true, "apps": {"codex": {"mode": "proxy"}}}"#,
        )
        .unwrap();

        set_pending(&store, "claude", Some(sample_pending())).unwrap();
        set_pending(&store, "claude", None).unwrap();

        let raw: Value = serde_json::from_slice(&fs::read(store.state_path()).unwrap()).unwrap();
        assert_eq!(
            raw,
            json!({"version": 2, "future": true, "apps": {"codex": {"mode": "proxy"}}})
        );
    }

    #[test]
    fn a_corrupt_state_file_is_moved_aside() {
        let dir = tempfile::tempdir().unwrap();
        let store = DeviceStore::at(dir.path());
        fs::write(store.state_path(), "{ not json").unwrap();

        assert_eq!(load(&store).unwrap(), LiveState::default());
        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names
                .iter()
                .any(|name| name.starts_with("live-state.json.corrupt-")),
            "{names:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn state_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = DeviceStore::at(dir.path());
        set_pending(&store, "claude", Some(sample_pending())).unwrap();
        let mode = fs::metadata(store.state_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
