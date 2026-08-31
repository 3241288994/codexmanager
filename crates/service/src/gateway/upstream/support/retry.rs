use bytes::Bytes;
use codexmanager_core::storage::Account;
use reqwest::StatusCode;
use std::time::{Duration, Instant};

use super::super::attempt_flow::transport::send_upstream_request;
use super::super::attempt_flow::transport::UpstreamRequestContext;
use super::super::GatewayUpstreamResponse;

fn value_contains_codex(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|value| value.to_ascii_lowercase().contains("codex"))
}

fn looks_like_native_codex_client(
    incoming_headers: &super::super::super::IncomingHeaderSnapshot,
) -> bool {
    value_contains_codex(incoming_headers.user_agent())
        || value_contains_codex(incoming_headers.originator())
        || incoming_headers.client_request_id().is_some()
        || incoming_headers.subagent().is_some()
        || incoming_headers.beta_features().is_some()
        || incoming_headers.window_id().is_some()
        || incoming_headers.turn_metadata().is_some()
        || incoming_headers.turn_state().is_some()
        || incoming_headers.parent_thread_id().is_some()
}

fn should_skip_codex_v1_alt_for_api_client(
    request_ctx: UpstreamRequestContext<'_>,
    incoming_headers: &super::super::super::IncomingHeaderSnapshot,
    alt_url: &str,
) -> bool {
    request_ctx.protocol_type == crate::apikey_profile::PROTOCOL_OPENAI_COMPAT
        && request_ctx.request_path.starts_with("/v1/responses")
        && alt_url
            .to_ascii_lowercase()
            .contains("/backend-api/codex/v1/")
        && !looks_like_native_codex_client(incoming_headers)
}

pub(in super::super) enum AltPathRetryResult {
    NotTriggered,
    Upstream(GatewayUpstreamResponse),
    Failover,
    Terminal { status_code: u16, message: String },
}

/// 函数 `retry_with_alternate_path`
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
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn retry_with_alternate_path<F>(
    client: &reqwest::blocking::Client,
    method: &reqwest::Method,
    alt_url: Option<&str>,
    request_deadline: Option<Instant>,
    request_ctx: UpstreamRequestContext<'_>,
    incoming_headers: &super::super::super::IncomingHeaderSnapshot,
    body: &Bytes,
    is_stream: bool,
    auth_token: &str,
    account: &Account,
    strip_session_affinity: bool,
    status: StatusCode,
    debug: bool,
    has_more_candidates: bool,
    mut log_gateway_result: F,
) -> AltPathRetryResult
where
    F: FnMut(Option<&str>, u16, Option<&str>),
{
    let Some(alt_url) = alt_url else {
        return AltPathRetryResult::NotTriggered;
    };
    if !matches!(status.as_u16(), 400 | 404) {
        return AltPathRetryResult::NotTriggered;
    }
    if should_skip_codex_v1_alt_for_api_client(request_ctx, incoming_headers, alt_url) {
        log::warn!(
            "event=gateway_upstream_alt_retry_skipped path={} status={} reason=api_client_codex_v1_alt_blocked upstream_url={}",
            request_ctx.request_path,
            status.as_u16(),
            alt_url
        );
        return AltPathRetryResult::NotTriggered;
    }
    if debug {
        log::warn!(
            "event=gateway_upstream_alt_retry path={} status={} account_id={} upstream_url={}",
            request_ctx.request_path,
            status.as_u16(),
            account.id,
            alt_url
        );
    }
    if super::deadline::is_expired(request_deadline) {
        return AltPathRetryResult::Terminal {
            status_code: 504,
            message: "upstream total timeout exceeded".to_string(),
        };
    }
    if !super::backoff::sleep_with_exponential_jitter(
        Duration::from_millis(40),
        Duration::from_millis(200),
        0,
        request_deadline,
    ) {
        return AltPathRetryResult::Terminal {
            status_code: 504,
            message: "upstream total timeout exceeded".to_string(),
        };
    }
    match send_upstream_request(
        client,
        method,
        alt_url,
        request_deadline,
        request_ctx,
        incoming_headers,
        body,
        is_stream,
        auth_token,
        account,
        strip_session_affinity,
    ) {
        Ok(response) => AltPathRetryResult::Upstream(response),
        Err(err) => {
            let err_msg = err.to_string();
            super::super::super::mark_account_cooldown(
                &account.id,
                super::super::super::CooldownReason::Network,
            );
            log_gateway_result(Some(alt_url), 502, Some(err_msg.as_str()));
            // 中文注释：alt 路径失败时若还有候选账号必须优先切换，
            // 不这样做会把单账号路径差异放大成整次请求失败。
            if has_more_candidates {
                AltPathRetryResult::Failover
            } else {
                AltPathRetryResult::Terminal {
                    status_code: 502,
                    message: format!("upstream error: {err}"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{should_skip_codex_v1_alt_for_api_client, UpstreamRequestContext};
    use crate::gateway::IncomingHeaderSnapshot;
    use axum::http::{HeaderMap, HeaderValue};

    fn headers(user_agent: Option<&str>, originator: Option<&str>) -> IncomingHeaderSnapshot {
        let mut headers = HeaderMap::new();
        if let Some(user_agent) = user_agent {
            headers.insert(
                "user-agent",
                HeaderValue::from_str(user_agent).expect("user agent header"),
            );
        }
        if let Some(originator) = originator {
            headers.insert(
                "originator",
                HeaderValue::from_str(originator).expect("originator header"),
            );
        }
        IncomingHeaderSnapshot::from_http_headers(&headers)
    }

    #[test]
    fn api_client_responses_request_skips_codex_v1_alt_retry() {
        let request_ctx = UpstreamRequestContext {
            request_path: "/v1/responses",
            protocol_type: crate::apikey_profile::PROTOCOL_OPENAI_COMPAT,
        };
        let incoming_headers = headers(Some("CherryStudio/1.0"), None);

        assert!(should_skip_codex_v1_alt_for_api_client(
            request_ctx,
            &incoming_headers,
            "https://chatgpt.com/backend-api/codex/v1/responses"
        ));
    }

    #[test]
    fn native_codex_responses_request_keeps_codex_v1_alt_retry_available() {
        let request_ctx = UpstreamRequestContext {
            request_path: "/v1/responses",
            protocol_type: crate::apikey_profile::PROTOCOL_OPENAI_COMPAT,
        };
        let incoming_headers = headers(Some("codex-cli/0.1.0"), Some("codex_cli_rs"));

        assert!(!should_skip_codex_v1_alt_for_api_client(
            request_ctx,
            &incoming_headers,
            "https://chatgpt.com/backend-api/codex/v1/responses"
        ));
    }
}
