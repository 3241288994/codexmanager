use crate::{
    app_shell::{request_show_main_window, set_unsaved_settings_draft_sections},
    commands::shared::{
        open_external_url_blocking, open_in_browser_blocking, open_in_file_manager_blocking,
    },
};

/// 函数 `open_in_browser`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - url: 参数 url
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn open_in_browser(url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open_in_browser_blocking(&url))
        .await
        .map_err(|err| format!("open_in_browser task failed: {err}"))?
}

#[tauri::command]
pub async fn open_external_url(url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open_external_url_blocking(&url))
        .await
        .map_err(|err| format!("open_external_url task failed: {err}"))?
}

/// 函数 `open_in_file_manager`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - path: 参数 path
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn open_in_file_manager(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open_in_file_manager_blocking(&path))
        .await
        .map_err(|err| format!("open_in_file_manager task failed: {err}"))?
}

/// 函数 `app_window_unsaved_draft_sections_set`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - sections: 参数 sections
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub fn app_window_unsaved_draft_sections_set(sections: Vec<String>) -> Result<(), String> {
    set_unsaved_settings_draft_sections(sections);
    Ok(())
}

#[tauri::command]
pub fn app_show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    request_show_main_window(&app)
}
