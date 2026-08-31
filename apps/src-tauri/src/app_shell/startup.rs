use crate::commands::settings::sync_window_runtime_state_from_settings;

use super::state::TRAY_AVAILABLE;

/// 函数 `sync_startup_window_state`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 无
pub(crate) fn sync_startup_window_state() {
    if let Ok(mut settings) = codexmanager_service::app_settings_get_with_overrides(
        Some(
            codexmanager_service::current_close_to_tray_on_close_setting()
                && TRAY_AVAILABLE.load(std::sync::atomic::Ordering::Relaxed),
        ),
        Some(TRAY_AVAILABLE.load(std::sync::atomic::Ordering::Relaxed)),
    ) {
        sync_window_runtime_state_from_settings(&mut settings);
    }
}
