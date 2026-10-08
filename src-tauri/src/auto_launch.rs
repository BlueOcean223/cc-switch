use crate::error::AppError;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};

/// 获取 macOS 上的 .app bundle 路径
/// 将 `/path/to/CC Switch.app/Contents/MacOS/CC Switch` 转换为 `/path/to/CC Switch.app`
#[cfg(target_os = "macos")]
fn get_macos_app_bundle_path(exe_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let path_str = exe_path.to_string_lossy();
    // 查找 .app/Contents/MacOS/ 模式
    if let Some(app_pos) = path_str.find(".app/Contents/MacOS/") {
        let app_bundle_end = app_pos + 4; // ".app" 的结束位置
        Some(std::path::PathBuf::from(&path_str[..app_bundle_end]))
    } else {
        None
    }
}

/// 初始化 AutoLaunch 实例
fn get_auto_launch() -> Result<AutoLaunch, AppError> {
    let app_name = "ccs-lite";
    let exe_path =
        std::env::current_exe().map_err(|e| AppError::Message(format!("无法获取应用路径: {e}")))?;

    // macOS 需要使用 .app bundle 路径，否则 AppleScript login item 会打开终端
    #[cfg(target_os = "macos")]
    let app_path = get_macos_app_bundle_path(&exe_path).unwrap_or(exe_path);

    #[cfg(not(target_os = "macos"))]
    let app_path = exe_path;

    // 使用 AutoLaunchBuilder 消除平台差异
    // macOS: 使用 AppleScript 方式（默认），需要 .app bundle 路径
    // Windows/Linux: 使用注册表/XDG autostart
    let auto_launch = AutoLaunchBuilder::new()
        .set_app_name(app_name)
        .set_app_path(&app_path.to_string_lossy())
        .build()
        .map_err(|e| AppError::Message(format!("创建 AutoLaunch 失败: {e}")))?;

    Ok(auto_launch)
}

/// 启用开机自启
pub fn enable_auto_launch() -> Result<(), AppError> {
    let auto_launch = get_auto_launch()?;
    auto_launch
        .enable()
        .map_err(|e| AppError::Message(format!("启用开机自启失败: {e}")))?;
    log::info!("已启用开机自启");
    Ok(())
}

/// 禁用开机自启
pub fn disable_auto_launch() -> Result<(), AppError> {
    let auto_launch = get_auto_launch()?;
    auto_launch
        .disable()
        .map_err(|e| AppError::Message(format!("禁用开机自启失败: {e}")))?;
    log::info!("已禁用开机自启");
    Ok(())
}

/// 检查是否已启用开机自启
fn is_auto_launch_enabled() -> Result<bool, AppError> {
    let auto_launch = get_auto_launch()?;
    auto_launch
        .is_enabled()
        .map_err(|e| AppError::Message(format!("检查开机自启状态失败: {e}")))
}

/// 启动时补上设置里开着、系统里却没有的登录项。
///
/// 设置可能来自上游 CC Switch 的导入（上游的登录项名字不同），也可能被用户在系统设置里
/// 删掉了登录项；这两种情况下开关显示开着，实际不会开机启动。
///
/// 设置关着时不查也不改：macOS 上查询要用 AppleScript 问 System Events，会弹"自动化"
/// 授权，没开这个功能的用户不该看到；关掉开关时设置页已经删过登录项。
///
/// 调试构建不做，免得把 target/debug 下的二进制加进登录项。
pub fn align_with_setting(wanted: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    if let Err(e) = align(wanted, is_auto_launch_enabled, enable_auto_launch) {
        log::warn!("对齐开机自启失败: {e}");
    }
}

/// [`align_with_setting`] 的步骤：设置开着才查询，没注册就注册。
fn align(
    wanted: bool,
    registered: impl FnOnce() -> Result<bool, AppError>,
    register: impl FnOnce() -> Result<(), AppError>,
) -> Result<(), AppError> {
    if !wanted || registered()? {
        return Ok(());
    }
    register()
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use std::cell::Cell;

    /// 跑一次 `align`，返回（查询次数，注册次数，结果是否成功）
    fn run_align(wanted: bool, registered: Result<bool, AppError>) -> (u32, u32, bool) {
        let queried = Cell::new(0);
        let registered_calls = Cell::new(0);
        let outcome = align(
            wanted,
            || {
                queried.set(queried.get() + 1);
                registered
            },
            || {
                registered_calls.set(registered_calls.get() + 1);
                Ok(())
            },
        );
        (queried.get(), registered_calls.get(), outcome.is_ok())
    }

    #[test]
    fn align_queries_only_when_the_setting_is_on() {
        // 设置关着：不查询（macOS 上不弹自动化授权），也不改登录项
        assert_eq!(run_align(false, Ok(true)), (0, 0, true));
        // 开着但没有登录项：注册
        assert_eq!(run_align(true, Ok(false)), (1, 1, true));
        // 已经注册：不动
        assert_eq!(run_align(true, Ok(true)), (1, 0, true));
        // 查询失败：不注册，报错给调用方记日志
        let failed = run_align(true, Err(AppError::Message("denied".to_string())));
        assert_eq!(failed, (1, 0, false));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_valid() {
        let exe_path = std::path::Path::new("/Applications/CC Switch.app/Contents/MacOS/CC Switch");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(
            result,
            Some(std::path::PathBuf::from("/Applications/CC Switch.app"))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_with_spaces() {
        let exe_path =
            std::path::Path::new("/Users/test/My Apps/CC Switch.app/Contents/MacOS/CC Switch");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(
            result,
            Some(std::path::PathBuf::from(
                "/Users/test/My Apps/CC Switch.app"
            ))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_not_in_bundle() {
        let exe_path = std::path::Path::new("/usr/local/bin/cc-switch");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(result, None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_dev_build() {
        // 开发环境下的路径通常不在 .app bundle 内
        let exe_path = std::path::Path::new("/Users/dev/project/target/debug/cc-switch");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(result, None);
    }
}
