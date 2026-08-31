use crate::commands::shared::rpc_call_in_background;

/// 函数 `service_gateway_route_strategy_get`
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
pub async fn service_gateway_route_strategy_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/routeStrategy/get", addr, None).await
}

/// 函数 `service_gateway_route_strategy_set`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - strategy: 参数 strategy
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_gateway_route_strategy_set(
    addr: Option<String>,
    strategy: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "strategy": strategy });
    rpc_call_in_background("gateway/routeStrategy/set", addr, Some(params)).await
}

/// 函数 `service_gateway_manual_account_get`
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
pub async fn service_gateway_manual_account_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/manualAccount/get", addr, None).await
}

/// 函数 `service_gateway_manual_account_set`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - account_id: 参数 account_id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_gateway_manual_account_set(
    addr: Option<String>,
    account_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "accountId": account_id });
    rpc_call_in_background("gateway/manualAccount/set", addr, Some(params)).await
}

/// 函数 `service_gateway_manual_account_clear`
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
pub async fn service_gateway_manual_account_clear(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/manualAccount/clear", addr, None).await
}

/// 函数 `service_gateway_background_tasks_get`
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
pub async fn service_gateway_background_tasks_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/backgroundTasks/get", addr, None).await
}

/// 函数 `service_gateway_background_tasks_set`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - usage_polling_enabled: 参数 usage_polling_enabled
/// - usage_poll_interval_secs: 参数 usage_poll_interval_secs
/// - gateway_keepalive_enabled: 参数 gateway_keepalive_enabled
/// - gateway_keepalive_interval_secs: 参数 gateway_keepalive_interval_secs
/// - token_refresh_polling_enabled: 参数 token_refresh_polling_enabled
/// - token_refresh_poll_interval_secs: 参数 token_refresh_poll_interval_secs
/// - usage_refresh_workers: 参数 usage_refresh_workers
/// - http_worker_factor: 参数 http_worker_factor
/// - http_worker_min: 参数 http_worker_min
/// - http_stream_worker_factor: 参数 http_stream_worker_factor
/// - http_stream_worker_min: 参数 http_stream_worker_min
///
/// # 返回
/// 返回函数执行结果
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn service_gateway_background_tasks_set(
    addr: Option<String>,
    usage_polling_enabled: Option<bool>,
    usage_poll_interval_secs: Option<u64>,
    gateway_keepalive_enabled: Option<bool>,
    gateway_keepalive_interval_secs: Option<u64>,
    token_refresh_polling_enabled: Option<bool>,
    token_refresh_poll_interval_secs: Option<u64>,
    usage_refresh_workers: Option<u64>,
    http_worker_factor: Option<u64>,
    http_worker_min: Option<u64>,
    http_stream_worker_factor: Option<u64>,
    http_stream_worker_min: Option<u64>,
    warmup_cron_enabled: Option<bool>,
    warmup_cron_expression: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "usagePollingEnabled": usage_polling_enabled,
      "usagePollIntervalSecs": usage_poll_interval_secs,
      "gatewayKeepaliveEnabled": gateway_keepalive_enabled,
      "gatewayKeepaliveIntervalSecs": gateway_keepalive_interval_secs,
      "tokenRefreshPollingEnabled": token_refresh_polling_enabled,
      "tokenRefreshPollIntervalSecs": token_refresh_poll_interval_secs,
      "usageRefreshWorkers": usage_refresh_workers,
      "httpWorkerFactor": http_worker_factor,
      "httpWorkerMin": http_worker_min,
      "httpStreamWorkerFactor": http_stream_worker_factor,
      "httpStreamWorkerMin": http_stream_worker_min,
      "warmupCronEnabled": warmup_cron_enabled,
      "warmupCronExpression": warmup_cron_expression
    });
    rpc_call_in_background("gateway/backgroundTasks/set", addr, Some(params)).await
}

/// 函数 `service_gateway_concurrency_recommend_get`
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
pub async fn service_gateway_concurrency_recommend_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/concurrencyRecommendation/get", addr, None).await
}

/// 函数 `service_gateway_codex_latest_version_get`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-11
///
/// # 参数
/// - addr: 参数 addr
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_gateway_codex_latest_version_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/codexLatestVersion/get", addr, None).await
}

/// 函数 `service_gateway_upstream_proxy_get`
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
pub async fn service_gateway_upstream_proxy_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/upstreamProxy/get", addr, None).await
}

/// 函数 `service_gateway_upstream_proxy_set`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - proxy_url: 参数 proxy_url
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_gateway_upstream_proxy_set(
    addr: Option<String>,
    proxy_url: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "proxyUrl": proxy_url });
    rpc_call_in_background("gateway/upstreamProxy/set", addr, Some(params)).await
}

/// 函数 `service_gateway_transport_get`
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
pub async fn service_gateway_transport_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/transport/get", addr, None).await
}

/// 函数 `service_gateway_transport_set`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - sse_keepalive_interval_ms: 参数 sse_keepalive_interval_ms
/// - upstream_stream_timeout_ms: 参数 upstream_stream_timeout_ms
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_gateway_transport_set(
    addr: Option<String>,
    sse_keepalive_interval_ms: Option<u64>,
    upstream_stream_timeout_ms: Option<u64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "sseKeepaliveIntervalMs": sse_keepalive_interval_ms,
      "upstreamStreamTimeoutMs": upstream_stream_timeout_ms
    });
    rpc_call_in_background("gateway/transport/set", addr, Some(params)).await
}
