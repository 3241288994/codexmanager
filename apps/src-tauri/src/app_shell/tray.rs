use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::service_runtime::stop_service;

use super::prompts::confirm_discard_unsaved_settings_for_app_exit;
use super::state::{
    has_unsaved_settings_draft_sections, mark_skip_next_unsaved_settings_exit_confirm,
    APP_EXIT_REQUESTED, KEEP_ALIVE_FOR_LIGHTWEIGHT_CLOSE, TRAY_AVAILABLE,
};
use super::window::{request_show_main_window, toggle_tray_preview_window};

const TRAY_MENU_SHOW_MAIN: &str = "tray_show_main";
const TRAY_MENU_QUIT_APP: &str = "tray_quit_app";

/// 函数 `setup_tray`
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
pub(crate) fn setup_tray(app: &tauri::AppHandle) -> Result<(), tauri::Error> {
    TRAY_AVAILABLE.store(false, std::sync::atomic::Ordering::Relaxed);
    let show_main = MenuItem::with_id(app, TRAY_MENU_SHOW_MAIN, "显示主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_MENU_QUIT_APP, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_main, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_MENU_SHOW_MAIN => {
                if let Err(err) = request_show_main_window(app) {
                    log::warn!("request show main window from tray failed: {}", err);
                }
            }
            TRAY_MENU_QUIT_APP => {
                if has_unsaved_settings_draft_sections() {
                    if !confirm_discard_unsaved_settings_for_app_exit() {
                        log::info!("tray exit canceled because settings drafts are still unsaved");
                        return;
                    }
                    mark_skip_next_unsaved_settings_exit_confirm();
                }
                APP_EXIT_REQUESTED.store(true, std::sync::atomic::Ordering::Relaxed);
                KEEP_ALIVE_FOR_LIGHTWEIGHT_CLOSE.store(false, std::sync::atomic::Ordering::Relaxed);
                stop_service();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                rect,
                ..
            } = event
            {
                toggle_tray_preview_window(tray.app_handle(), position, rect);
            }
        });
    if let Ok(icon) =
        tauri::image::Image::from_bytes(include_bytes!("../../icons/tray-template.png"))
    {
        tray = tray.icon(icon).icon_as_template(true);
    } else if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    TRAY_AVAILABLE.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
