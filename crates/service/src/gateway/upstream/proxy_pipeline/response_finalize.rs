use tiny_http::Request;

use super::super::super::request_log::RequestLogUsage;
use super::super::GatewayUpstreamResponse;
use super::execution_context::GatewayUpstreamExecutionContext;

pub(super) enum FinalizeUpstreamResponseOutcome {
    Handled,
    Failover,
}

/// 函数 `respond_terminal`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn respond_terminal(
    request: Request,
    status_code: u16,
    message: String,
    trace_id: Option<&str>,
) -> Result<(), String> {
    let response_message = super::super::super::error_message_for_client(
        super::super::super::prefers_raw_errors_for_tiny_http_request(&request),
        message,
    );
    let response = super::super::super::error_response::terminal_text_response(
        status_code,
        response_message,
        trace_id,
    );
    let _ = request.respond(response);
    Ok(())
}

/// 函数 `is_client_disconnect_error`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - message: 参数 message
///
/// # 返回
/// 返回函数执行结果
fn is_client_disconnect_error(message: &str) -> bool {
    let normalized = message.trim().to_ascii_lowercase();
    normalized.contains("broken pipe")
        || normalized.contains("connection reset")
        || normalized.contains("connection aborted")
        || normalized.contains("connection was forcibly closed")
        || normalized.contains("os error 32")
        || normalized.contains("os error 54")
        || normalized.contains("os error 104")
}

fn derive_final_error(
    status_code: u16,
    last_attempt_error: Option<&str>,
    upstream_error_hint: Option<&str>,
    bridge_error_message: Option<String>,
) -> Option<String> {
    upstream_error_hint
        .map(str::to_string)
        .or_else(|| {
            (status_code >= 400)
                .then(|| last_attempt_error.map(str::to_string))
                .flatten()
        })
        .or(bridge_error_message)
}

fn derive_status_for_log(
    status_code: u16,
    delivered_status_code: Option<u16>,
    bridge_ok: bool,
    gateway_failover: bool,
    upstream_stream_failed: bool,
    client_delivery_failed: bool,
) -> u16 {
    if client_delivery_failed {
        499
    } else if let Some(delivered_status_code) = delivered_status_code {
        delivered_status_code
    } else if status_code >= 400 {
        status_code
    } else if upstream_stream_failed || gateway_failover || !bridge_ok {
        502
    } else {
        status_code
    }
}

/// 函数 `respond_total_timeout`
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
pub(super) fn respond_total_timeout(
    request: Request,
    context: &GatewayUpstreamExecutionContext<'_>,
    trace_id: &str,
    started_at: std::time::Instant,
    model_for_log: Option<&str>,
    attempted_account_ids: Option<&[String]>,
) -> Result<(), String> {
    let message = "upstream total timeout exceeded".to_string();
    context.log_final_result_with_model(
        None,
        None,
        model_for_log,
        504,
        RequestLogUsage::default(),
        Some(message.as_str()),
        started_at.elapsed().as_millis(),
        attempted_account_ids,
    );
    respond_terminal(request, 504, message, Some(trace_id))
}

/// 函数 `finalize_terminal_candidate`
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
pub(super) fn finalize_terminal_candidate(
    request: Request,
    context: &GatewayUpstreamExecutionContext<'_>,
    account_id: &str,
    last_attempt_url: Option<&str>,
    status_code: u16,
    message: String,
    trace_id: &str,
    started_at: std::time::Instant,
    model_for_log: Option<&str>,
    attempted_account_ids: Option<&[String]>,
) -> Result<(), String> {
    let _ = context.mark_account_unavailable_for_gateway_error(account_id, &message);
    context.log_final_result_with_model(
        Some(account_id),
        last_attempt_url,
        model_for_log,
        status_code,
        RequestLogUsage::default(),
        Some(message.as_str()),
        started_at.elapsed().as_millis(),
        attempted_account_ids,
    );
    respond_terminal(request, status_code, message, Some(trace_id))
}

/// 函数 `finalize_upstream_response`
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
#[allow(clippy::too_many_arguments)]
pub(super) fn finalize_upstream_response(
    request: Request,
    response: GatewayUpstreamResponse,
    inflight_guard: super::super::super::AccountInFlightGuard,
    context: &GatewayUpstreamExecutionContext<'_>,
    account_id: &str,
    last_attempt_url: Option<&str>,
    last_attempt_error: Option<&str>,
    response_adapter: super::super::super::ResponseAdapter,
    gemini_stream_output_mode: Option<super::super::super::GeminiStreamOutputMode>,
    tool_name_restore_map: &super::super::super::ToolNameRestoreMap,
    client_is_stream: bool,
    path: &str,
    trace_id: &str,
    started_at: std::time::Instant,
    model_for_log: Option<&str>,
    attempted_account_ids: Option<&[String]>,
    has_more_candidates: bool,
) -> Result<FinalizeUpstreamResponseOutcome, String> {
    let status_code = response.status().as_u16();

    let bridge = super::super::super::respond_with_upstream(
        request,
        response,
        inflight_guard,
        response_adapter,
        None,
        gemini_stream_output_mode,
        path,
        Some(tool_name_restore_map),
        client_is_stream,
        has_more_candidates,
        Some(trace_id),
        model_for_log,
        started_at,
    )?;
    let bridge_output_text_len = bridge
        .usage
        .output_text
        .as_deref()
        .map(str::trim)
        .map(str::len)
        .unwrap_or(0);
    super::super::super::trace_log::log_bridge_result(
        super::super::super::trace_log::BridgeResultLog {
            trace_id,
            adapter: format!("{response_adapter:?}").as_str(),
            path,
            is_stream: client_is_stream,
            stream_terminal_seen: bridge.stream_terminal_seen,
            stream_terminal_error: bridge.stream_terminal_error.as_deref(),
            delivery_error: bridge.delivery_error.as_deref(),
            output_text_len: bridge_output_text_len,
            output_tokens: bridge.usage.output_tokens,
            first_response_ms: bridge.usage.first_response_ms,
            delivered_status_code: bridge.delivered_status_code,
            upstream_error_hint: bridge.upstream_error_hint.as_deref(),
            upstream_request_id: bridge.upstream_request_id.as_deref(),
            upstream_cf_ray: bridge.upstream_cf_ray.as_deref(),
            upstream_auth_error: bridge.upstream_auth_error.as_deref(),
            upstream_identity_error_code: bridge.upstream_identity_error_code.as_deref(),
            upstream_content_type: bridge.upstream_content_type.as_deref(),
            last_sse_event_type: bridge.last_sse_event_type.as_deref(),
        },
    );
    let bridge_ok = bridge.is_ok(client_is_stream);
    let bridge_error_message = (!bridge_ok).then(|| {
        bridge
            .error_message(client_is_stream)
            .unwrap_or_else(|| "upstream response incomplete".to_string())
    });
    let final_error = derive_final_error(
        status_code,
        last_attempt_error,
        bridge.upstream_error_hint.as_deref(),
        bridge_error_message,
    );
    let gateway_error_follow_up = final_error
        .as_deref()
        .map(|error| context.apply_gateway_error_follow_up(account_id, error, has_more_candidates));
    let gateway_failover =
        gateway_error_follow_up.is_some_and(|follow_up| follow_up.should_failover);

    let upstream_stream_failed = client_is_stream
        && (!bridge.stream_terminal_seen || bridge.stream_terminal_error.is_some());
    let client_delivery_failed = bridge
        .delivery_error
        .as_deref()
        .is_some_and(is_client_disconnect_error);
    let status_for_log = derive_status_for_log(
        status_code,
        bridge.delivered_status_code,
        bridge_ok,
        gateway_failover,
        upstream_stream_failed,
        client_delivery_failed,
    );

    if upstream_stream_failed {
        super::super::super::mark_account_cooldown(
            account_id,
            super::super::super::CooldownReason::Network,
        );
        super::super::super::record_route_quality(account_id, 502);
    }

    let usage = bridge.usage;
    context.log_final_result_with_model(
        Some(account_id),
        last_attempt_url,
        model_for_log,
        status_for_log,
        RequestLogUsage {
            input_tokens: usage.input_tokens,
            cached_input_tokens: usage.cached_input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            reasoning_output_tokens: usage.reasoning_output_tokens,
            first_response_ms: usage.first_response_ms,
        },
        final_error.as_deref(),
        started_at.elapsed().as_millis(),
        attempted_account_ids,
    );
    if gateway_failover {
        return Ok(FinalizeUpstreamResponseOutcome::Failover);
    }
    Ok(FinalizeUpstreamResponseOutcome::Handled)
}

#[cfg(test)]
mod tests {
    use super::{derive_final_error, derive_status_for_log, is_client_disconnect_error};

    #[test]
    fn derive_final_error_prefers_upstream_hint_then_http_error_then_bridge_error() {
        assert_eq!(
            derive_final_error(
                429,
                Some("last attempt"),
                Some("upstream hint"),
                Some("bridge error".to_string()),
            )
            .as_deref(),
            Some("upstream hint")
        );
        assert_eq!(
            derive_final_error(
                429,
                Some("last attempt"),
                None,
                Some("bridge error".to_string())
            )
            .as_deref(),
            Some("last attempt")
        );
        assert_eq!(
            derive_final_error(200, None, None, Some("bridge error".to_string())).as_deref(),
            Some("bridge error")
        );
    }

    #[test]
    fn derive_status_for_log_respects_disconnect_delivery_and_bridge_fallbacks() {
        assert_eq!(
            derive_status_for_log(200, None, true, false, false, true),
            499
        );
        assert_eq!(
            derive_status_for_log(200, Some(207), true, false, false, false),
            207
        );
        assert_eq!(
            derive_status_for_log(404, None, true, false, false, false),
            404
        );
        assert_eq!(
            derive_status_for_log(200, None, true, true, false, false),
            502
        );
        assert_eq!(
            derive_status_for_log(200, None, false, false, false, false),
            502
        );
        assert_eq!(
            derive_status_for_log(200, None, true, false, false, false),
            200
        );
    }

    #[test]
    fn client_disconnect_error_matches_common_socket_messages() {
        assert!(is_client_disconnect_error("broken pipe"));
        assert!(is_client_disconnect_error("connection reset by peer"));
        assert!(!is_client_disconnect_error("upstream timeout"));
    }
}
