use std::sync::atomic::Ordering;

use crate::app_shell::{
    CLOSE_TO_TRAY_ON_CLOSE, KEEP_ALIVE_FOR_LIGHTWEIGHT_CLOSE, LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY,
    TRAY_AVAILABLE,
};

/// 函数 `effective_lightweight_mode_on_close_to_tray`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - requested: 参数 requested
/// - close_to_tray_effective: 参数 close_to_tray_effective
///
/// # 返回
/// 返回函数执行结果
pub fn effective_lightweight_mode_on_close_to_tray(
    requested: bool,
    close_to_tray_effective: bool,
) -> bool {
    requested && close_to_tray_effective
}

/// 函数 `tray_available`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn tray_available() -> bool {
    TRAY_AVAILABLE.load(Ordering::Relaxed)
}

/// 函数 `effective_close_to_tray_requested`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn effective_close_to_tray_requested() -> bool {
    codexmanager_service::current_close_to_tray_on_close_setting() && tray_available()
}

/// 函数 `sync_window_runtime_state_from_settings`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - settings: 参数 settings
///
/// # 返回
/// 无
pub fn sync_window_runtime_state_from_settings(settings: &mut serde_json::Value) {
    let requested_close_to_tray = settings
        .get("closeToTrayOnClose")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let supported = settings
        .get("closeToTraySupported")
        .and_then(|value| value.as_bool())
        .unwrap_or_else(tray_available);
    let requested_lightweight_mode = settings
        .get("lightweightModeOnCloseToTray")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let effective_close_to_tray = requested_close_to_tray && supported;
    let effective_lightweight_mode = effective_lightweight_mode_on_close_to_tray(
        requested_lightweight_mode,
        effective_close_to_tray,
    );
    if let Some(object) = settings.as_object_mut() {
        object.insert(
            "closeToTrayOnClose".to_string(),
            serde_json::json!(effective_close_to_tray),
        );
        object.insert(
            "closeToTraySupported".to_string(),
            serde_json::json!(supported),
        );
        object.insert(
            "lightweightModeOnCloseToTray".to_string(),
            serde_json::json!(requested_lightweight_mode),
        );
    }
    CLOSE_TO_TRAY_ON_CLOSE.store(effective_close_to_tray, Ordering::Relaxed);
    LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY.store(effective_lightweight_mode, Ordering::Relaxed);
    if !effective_lightweight_mode {
        KEEP_ALIVE_FOR_LIGHTWEIGHT_CLOSE.store(false, Ordering::Relaxed);
    }
}
