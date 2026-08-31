use crate::commands::shared::rpc_call_in_background;

/// 函数 `service_plugin_catalog_list`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - market_mode: 参数 market_mode
/// - source_url: 参数 source_url
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_catalog_list(
    addr: Option<String>,
    market_mode: Option<String>,
    source_url: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "marketMode": market_mode,
        "sourceUrl": source_url,
    });
    rpc_call_in_background("plugin/catalog/list", addr, Some(params)).await
}

/// 函数 `service_plugin_catalog_refresh`
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
pub async fn service_plugin_catalog_refresh(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("plugin/catalog/refresh", addr, None).await
}

/// 函数 `service_plugin_install`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - entry: 参数 entry
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_install(
    addr: Option<String>,
    entry: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "entry": entry });
    rpc_call_in_background("plugin/install", addr, Some(params)).await
}

/// 函数 `service_plugin_update`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - entry: 参数 entry
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_update(
    addr: Option<String>,
    entry: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "entry": entry });
    rpc_call_in_background("plugin/update", addr, Some(params)).await
}

/// 函数 `service_plugin_uninstall`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - plugin_id: 参数 plugin_id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_uninstall(
    addr: Option<String>,
    plugin_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "pluginId": plugin_id });
    rpc_call_in_background("plugin/uninstall", addr, Some(params)).await
}

/// 函数 `service_plugin_list`
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
pub async fn service_plugin_list(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("plugin/list", addr, None).await
}

/// 函数 `service_plugin_enable`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - plugin_id: 参数 plugin_id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_enable(
    addr: Option<String>,
    plugin_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "pluginId": plugin_id });
    rpc_call_in_background("plugin/enable", addr, Some(params)).await
}

/// 函数 `service_plugin_disable`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - plugin_id: 参数 plugin_id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_disable(
    addr: Option<String>,
    plugin_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "pluginId": plugin_id });
    rpc_call_in_background("plugin/disable", addr, Some(params)).await
}

/// 函数 `service_plugin_tasks_update`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - task_id: 参数 task_id
/// - interval_seconds: 参数 interval_seconds
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_tasks_update(
    addr: Option<String>,
    task_id: String,
    interval_seconds: i64,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "taskId": task_id,
        "intervalSeconds": interval_seconds,
    });
    rpc_call_in_background("plugin/tasks/update", addr, Some(params)).await
}

/// 函数 `service_plugin_tasks_list`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - plugin_id: 参数 plugin_id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_tasks_list(
    addr: Option<String>,
    plugin_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "pluginId": plugin_id,
    });
    rpc_call_in_background("plugin/tasks/list", addr, Some(params)).await
}

/// 函数 `service_plugin_tasks_run`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - task_id: 参数 task_id
/// - input: 参数 input
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_tasks_run(
    addr: Option<String>,
    task_id: String,
    input: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "taskId": task_id,
        "input": input,
    });
    rpc_call_in_background("plugin/tasks/run", addr, Some(params)).await
}

/// 函数 `service_plugin_logs_list`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - plugin_id: 参数 plugin_id
/// - task_id: 参数 task_id
/// - limit: 参数 limit
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_plugin_logs_list(
    addr: Option<String>,
    plugin_id: Option<String>,
    task_id: Option<String>,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "pluginId": plugin_id,
        "taskId": task_id,
        "limit": limit,
    });
    rpc_call_in_background("plugin/logs/list", addr, Some(params)).await
}
