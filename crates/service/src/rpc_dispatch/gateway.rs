use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;

/// 函数 `try_handle`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 返回函数执行结果
pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "gateway/routeStrategy/get" => {
            let strategy = crate::gateway::current_route_strategy();
            super::as_json(serde_json::json!({
                "strategy": strategy,
                "options": ["ordered", "balanced"],
                "manualPreferredAccountId": crate::gateway::manual_preferred_account(),
            }))
        }
        "gateway/routeStrategy/set" => {
            let strategy = super::str_param(req, "strategy").unwrap_or("");
            super::value_or_error(crate::set_gateway_route_strategy(strategy).map(|applied| {
                serde_json::json!({
                    "strategy": applied
                })
            }))
        }
        "gateway/manualAccount/get" => super::as_json(serde_json::json!({
            "accountId": crate::gateway::manual_preferred_account()
        })),
        "gateway/manualAccount/set" => {
            let account_id = super::str_param(req, "accountId").unwrap_or("");
            super::ok_or_error(crate::gateway::set_manual_preferred_account(account_id))
        }
        "gateway/manualAccount/clear" => {
            crate::gateway::clear_manual_preferred_account();
            super::ok_result()
        }
        "gateway/backgroundTasks/get" => {
            super::as_json(crate::usage_refresh::background_tasks_settings())
        }
        "gateway/concurrencyRecommendation/get" => {
            super::as_json(crate::gateway::current_gateway_concurrency_recommendation())
        }
        "gateway/codexLatestVersion/get" => {
            super::value_or_error(crate::fetch_codex_latest_version())
        }
        "gateway/upstreamProxy/get" => super::as_json(serde_json::json!({
            "proxyUrl": crate::gateway::current_upstream_proxy_url(),
            "envKey": "CODEXMANAGER_UPSTREAM_PROXY_URL",
            "requiresRestart": false,
        })),
        "gateway/upstreamProxy/set" => {
            let requested = req
                .params
                .as_ref()
                .and_then(|params| params.get("proxyUrl"))
                .and_then(|value| match value {
                    Value::Null => Some(None),
                    Value::String(text) => Some(Some(text.as_str())),
                    _ => None,
                })
                .or_else(|| super::str_param(req, "url").map(|value| Some(value)));
            let proxy_url = requested.unwrap_or(None);
            super::value_or_error(
                crate::set_gateway_upstream_proxy_url(proxy_url).map(|applied| {
                    serde_json::json!({
                        "proxyUrl": applied,
                        "envKey": "CODEXMANAGER_UPSTREAM_PROXY_URL",
                        "requiresRestart": false,
                    })
                }),
            )
        }
        "gateway/transport/get" => super::as_json(serde_json::json!({
            "sseKeepaliveIntervalMs": crate::current_gateway_sse_keepalive_interval_ms(),
            "upstreamStreamTimeoutMs": crate::current_gateway_upstream_stream_timeout_ms(),
            "upstreamTotalTimeoutMs": crate::current_gateway_upstream_total_timeout_ms(),
            "envKeys": [
                "CODEXMANAGER_SSE_KEEPALIVE_INTERVAL_MS",
                "CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS",
                "CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS"
            ],
            "requiresRestart": false,
        })),
        "gateway/transport/set" => {
            let requested_sse_keepalive_interval_ms = u64_param(req, "sseKeepaliveIntervalMs");
            let requested_upstream_stream_timeout_ms = u64_param(req, "upstreamStreamTimeoutMs");
            let requested_upstream_total_timeout_ms = u64_param(req, "upstreamTotalTimeoutMs");
            super::value_or_error((|| {
                let sse_keepalive_interval_ms =
                    if let Some(value) = requested_sse_keepalive_interval_ms {
                        crate::set_gateway_sse_keepalive_interval_ms(value)?
                    } else {
                        crate::current_gateway_sse_keepalive_interval_ms()
                    };
                let upstream_stream_timeout_ms =
                    if let Some(value) = requested_upstream_stream_timeout_ms {
                        crate::set_gateway_upstream_stream_timeout_ms(value)?
                    } else {
                        crate::current_gateway_upstream_stream_timeout_ms()
                    };
                let upstream_total_timeout_ms =
                    if let Some(value) = requested_upstream_total_timeout_ms {
                        crate::set_gateway_upstream_total_timeout_ms(value)?
                    } else {
                        crate::current_gateway_upstream_total_timeout_ms()
                    };
                Ok(serde_json::json!({
                    "sseKeepaliveIntervalMs": sse_keepalive_interval_ms,
                    "upstreamStreamTimeoutMs": upstream_stream_timeout_ms,
                    "upstreamTotalTimeoutMs": upstream_total_timeout_ms,
                    "requiresRestart": false,
                }))
            })())
        }
        "gateway/backgroundTasks/set" => {
            let patch = crate::usage_refresh::BackgroundTasksSettingsPatch {
                usage_polling_enabled: super::bool_param(req, "usagePollingEnabled")
                    .or_else(|| super::bool_param(req, "usagePolling")),
                usage_poll_interval_secs: u64_param(req, "usagePollIntervalSecs"),
                gateway_keepalive_enabled: super::bool_param(req, "gatewayKeepaliveEnabled")
                    .or_else(|| super::bool_param(req, "gatewayKeepalive")),
                gateway_keepalive_interval_secs: u64_param(req, "gatewayKeepaliveIntervalSecs"),
                token_refresh_polling_enabled: super::bool_param(req, "tokenRefreshPollingEnabled")
                    .or_else(|| super::bool_param(req, "tokenRefreshPolling")),
                token_refresh_poll_interval_secs: u64_param(req, "tokenRefreshPollIntervalSecs"),
                usage_refresh_workers: usize_param(req, "usageRefreshWorkers"),
                http_worker_factor: usize_param(req, "httpWorkerFactor"),
                http_worker_min: usize_param(req, "httpWorkerMin"),
                http_stream_worker_factor: usize_param(req, "httpStreamWorkerFactor"),
                http_stream_worker_min: usize_param(req, "httpStreamWorkerMin"),
                warmup_cron_enabled: super::bool_param(req, "warmupCronEnabled"),
                warmup_cron_expression: super::string_param(req, "warmupCronExpression"),
            };
            let input = crate::BackgroundTasksInput {
                usage_polling_enabled: patch.usage_polling_enabled,
                usage_poll_interval_secs: patch.usage_poll_interval_secs,
                gateway_keepalive_enabled: patch.gateway_keepalive_enabled,
                gateway_keepalive_interval_secs: patch.gateway_keepalive_interval_secs,
                token_refresh_polling_enabled: patch.token_refresh_polling_enabled,
                token_refresh_poll_interval_secs: patch.token_refresh_poll_interval_secs,
                usage_refresh_workers: patch.usage_refresh_workers,
                http_worker_factor: patch.http_worker_factor,
                http_worker_min: patch.http_worker_min,
                http_stream_worker_factor: patch.http_stream_worker_factor,
                http_stream_worker_min: patch.http_stream_worker_min,
                warmup_cron_enabled: patch.warmup_cron_enabled,
                warmup_cron_expression: patch.warmup_cron_expression,
            };
            super::value_or_error(crate::set_gateway_background_tasks(input))
        }
        _ => return None,
    };

    Some(super::response(req, result))
}

/// 函数 `u64_param`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - req: 参数 req
/// - key: 参数 key
///
/// # 返回
/// 返回函数执行结果
fn u64_param(req: &JsonRpcRequest, key: &str) -> Option<u64> {
    let value = req.params.as_ref()?.get(key)?;
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    }
}

/// 函数 `usize_param`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - req: 参数 req
/// - key: 参数 key
///
/// # 返回
/// 返回函数执行结果
fn usize_param(req: &JsonRpcRequest, key: &str) -> Option<usize> {
    u64_param(req, key).and_then(|value| usize::try_from(value).ok())
}
