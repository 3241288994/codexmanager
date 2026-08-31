use crate::app_storage::apply_runtime_storage_env;
use crate::commands::shared::rpc_call_in_background;

/// 函数 `service_startup_snapshot`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - app: 参数 app
/// - addr: 参数 addr
/// - request_log_limit: 参数 request_log_limit
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_startup_snapshot(
    app: tauri::AppHandle,
    addr: Option<String>,
    request_log_limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    apply_runtime_storage_env(&app);
    let params = request_log_limit.map(|value| serde_json::json!({ "requestLogLimit": value }));
    rpc_call_in_background("startup/snapshot", addr, params).await
}
