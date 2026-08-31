use rfd::FileDialog;

use crate::app_storage::{
    read_account_import_contents_from_directory, read_account_import_contents_from_files,
};
use crate::commands::shared::rpc_call_in_background;
use crate::rpc_client::rpc_call;

/// 函数 `service_account_import`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - contents: 参数 contents
/// - content: 参数 content
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_account_import(
    addr: Option<String>,
    contents: Option<Vec<String>>,
    content: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut payload_contents = contents.unwrap_or_default();
    if let Some(single) = content {
        if !single.trim().is_empty() {
            payload_contents.push(single);
        }
    }
    let params = serde_json::json!({ "contents": payload_contents });
    rpc_call_in_background("account/import", addr, Some(params)).await
}

/// 函数 `service_account_import_by_directory`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - _addr: 参数 _addr
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_account_import_by_directory(
    _addr: Option<String>,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let selected_dir = FileDialog::new()
            .set_title("选择账号导入目录")
            .pick_folder();
        let Some(dir_path) = selected_dir else {
            return Ok(serde_json::json!({
              "result": {
                "ok": true,
                "canceled": true
              }
            }));
        };

        let (json_files, contents) = read_account_import_contents_from_directory(&dir_path)?;
        Ok(serde_json::json!({
          "result": {
            "ok": true,
            "canceled": false,
            "directoryPath": dir_path.to_string_lossy().to_string(),
            "fileCount": json_files.len(),
            "contents": contents
          }
        }))
    })
    .await
    .map_err(|err| format!("service_account_import_by_directory task failed: {err}"))?
}

/// 函数 `service_account_import_by_file`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - _addr: 参数 _addr
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_account_import_by_file(
    _addr: Option<String>,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let selected_files = FileDialog::new()
            .set_title("选择账号导入文件")
            .add_filter("账号文件", &["json", "txt"])
            .pick_files();
        let Some(file_paths) = selected_files else {
            return Ok(serde_json::json!({
              "result": {
                "ok": true,
                "canceled": true
              }
            }));
        };

        let contents = read_account_import_contents_from_files(&file_paths)?;
        Ok(serde_json::json!({
          "result": {
            "ok": true,
            "canceled": false,
            "filePaths": file_paths
              .iter()
              .map(|path| path.to_string_lossy().to_string())
              .collect::<Vec<_>>(),
            "fileCount": file_paths.len(),
            "contents": contents
          }
        }))
    })
    .await
    .map_err(|err| format!("service_account_import_by_file task failed: {err}"))?
}

/// 函数 `service_account_export_by_account_files`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_account_export_by_account_files(
    addr: Option<String>,
    selected_account_ids: Option<Vec<String>>,
    export_mode: Option<String>,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let selected_dir = FileDialog::new()
            .set_title("选择账号导出目录")
            .pick_folder();
        let Some(dir_path) = selected_dir else {
            return Ok(serde_json::json!({
              "result": {
                "ok": true,
                "canceled": true
              }
            }));
        };
        let params = serde_json::json!({
          "outputDir": dir_path.to_string_lossy().to_string(),
          "selectedAccountIds": selected_account_ids.unwrap_or_default(),
          "exportMode": export_mode.unwrap_or_else(|| "multiple".to_string())
        });
        rpc_call("account/export", addr, Some(params))
    })
    .await
    .map_err(|err| format!("service_account_export_by_account_files task failed: {err}"))?
}
