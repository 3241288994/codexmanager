use bytes::Bytes;
use codexmanager_core::storage::{Account, Storage, Token};
use std::time::Instant;
use tiny_http::Request;

use super::super::attempt_flow::transport::UpstreamRequestContext;
use super::super::executor::CandidateUpstreamDecision;
use super::super::support::candidates::{
    account_model_override, allow_openai_fallback_for_account, free_account_model_override,
};
use super::super::support::deadline;
use super::candidate_attempt::{
    run_candidate_attempt, CandidateAttemptParams, CandidateAttemptTrace,
};
use super::candidate_state::CandidateExecutionState;
use super::execution_context::GatewayUpstreamExecutionContext;
use super::request_setup::UpstreamRequestSetup;
use super::response_finalize::{
    finalize_terminal_candidate, finalize_upstream_response, respond_total_timeout,
    FinalizeUpstreamResponseOutcome,
};

/// 函数 `extract_prompt_cache_key_for_trace`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - body: 参数 body
///
/// # 返回
/// 返回函数执行结果
fn extract_prompt_cache_key_for_trace(body: &[u8]) -> Option<String> {
    if body.is_empty() || body.len() > 64 * 1024 {
        return None;
    }
    let value = serde_json::from_slice::<serde_json::Value>(body).ok()?;
    value
        .get("prompt_cache_key")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn should_forward_thread_anchor_as_prompt_cache_key(protocol_type: &str) -> bool {
    protocol_type != crate::apikey_profile::PROTOCOL_GEMINI_NATIVE
}

pub(in super::super) enum CandidateExecutionResult {
    Handled,
    Exhausted {
        request: Request,
        attempted_account_ids: Vec<String>,
        skipped_cooldown: usize,
        skipped_inflight: usize,
        last_attempt_url: Option<String>,
        last_attempt_error: Option<String>,
    },
}

pub(in super::super) struct CandidateExecutorParams<'a> {
    pub(in super::super) storage: &'a Storage,
    pub(in super::super) method: &'a reqwest::Method,
    pub(in super::super) incoming_headers: &'a super::super::super::IncomingHeaderSnapshot,
    pub(in super::super) body: &'a Bytes,
    pub(in super::super) path: &'a str,
    pub(in super::super) request_shape: Option<&'a str>,
    pub(in super::super) trace_id: &'a str,
    pub(in super::super) model_for_log: Option<&'a str>,
    pub(in super::super) response_adapter: super::super::super::ResponseAdapter,
    pub(in super::super) gemini_stream_output_mode:
        Option<super::super::super::GeminiStreamOutputMode>,
    pub(in super::super) tool_name_restore_map: &'a super::super::super::ToolNameRestoreMap,
    pub(in super::super) context: &'a GatewayUpstreamExecutionContext<'a>,
    pub(in super::super) setup: &'a UpstreamRequestSetup,
    pub(in super::super) request_deadline: Option<Instant>,
    pub(in super::super) started_at: Instant,
    pub(in super::super) client_is_stream: bool,
    pub(in super::super) upstream_is_stream: bool,
    pub(in super::super) debug: bool,
    pub(in super::super) allow_openai_fallback: bool,
    pub(in super::super) disable_challenge_stateless_retry: bool,
}

fn record_failover_attempt(
    attempt_trace: &mut CandidateAttemptTrace,
    last_attempt_url: &mut Option<String>,
    last_attempt_error: &mut Option<String>,
) {
    super::super::super::record_gateway_failover_attempt();
    *last_attempt_url = attempt_trace.last_attempt_url.take();
    *last_attempt_error = attempt_trace.last_attempt_error.take();
}

fn is_challenge_failover_error(error: Option<&str>) -> bool {
    error
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|value| {
            let normalized = value.to_ascii_lowercase();
            normalized.contains("challenge")
                || normalized.contains("cloudflare")
                || normalized.contains("cf_ray")
        })
}

fn should_force_strip_after_anthropic_challenge(
    context: &GatewayUpstreamExecutionContext<'_>,
    attempt_trace: &CandidateAttemptTrace,
) -> bool {
    context.protocol_type() == crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE
        && is_challenge_failover_error(attempt_trace.last_attempt_error.as_deref())
}

fn should_failover_terminal_gateway_error(
    context: &GatewayUpstreamExecutionContext<'_>,
    account_id: &str,
    has_more_candidates: bool,
    message: &str,
    attempt_trace: &mut CandidateAttemptTrace,
    last_attempt_url: &mut Option<String>,
    last_attempt_error: &mut Option<String>,
) -> bool {
    let gateway_error_follow_up =
        context.apply_gateway_error_follow_up(account_id, message, has_more_candidates);
    if !gateway_error_follow_up.should_failover {
        return false;
    }
    super::super::super::record_gateway_failover_attempt();
    *last_attempt_url = attempt_trace.last_attempt_url.take();
    *last_attempt_error = Some(message.to_string());
    true
}

#[allow(clippy::too_many_arguments)]
fn respond_terminal_attempt(
    request: Request,
    context: &GatewayUpstreamExecutionContext<'_>,
    account_id: &str,
    last_attempt_url: Option<&str>,
    status_code: u16,
    message: String,
    trace_id: &str,
    started_at: Instant,
    model_for_log: Option<&str>,
    attempted_account_ids: Option<&[String]>,
) -> Result<CandidateExecutionResult, String> {
    finalize_terminal_candidate(
        request,
        context,
        account_id,
        last_attempt_url,
        status_code,
        message,
        trace_id,
        started_at,
        model_for_log,
        attempted_account_ids,
    )?;
    Ok(CandidateExecutionResult::Handled)
}

/// 函数 `execute_candidate_sequence`
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
pub(in super::super) fn execute_candidate_sequence(
    request: Request,
    candidates: Vec<(Account, Token)>,
    params: CandidateExecutorParams<'_>,
) -> Result<CandidateExecutionResult, String> {
    let CandidateExecutorParams {
        storage,
        method,
        incoming_headers,
        body,
        path,
        request_shape,
        trace_id,
        model_for_log,
        response_adapter,
        gemini_stream_output_mode,
        tool_name_restore_map,
        context,
        setup,
        request_deadline,
        started_at,
        client_is_stream,
        upstream_is_stream,
        debug,
        allow_openai_fallback,
        disable_challenge_stateless_retry,
    } = params;
    let mut request = Some(request);
    let mut state = CandidateExecutionState::default();
    let mut attempted_account_ids = Vec::new();
    let mut skipped_cooldown = 0usize;
    let mut skipped_inflight = 0usize;
    let mut last_attempt_url = None;
    let mut last_attempt_error = None;
    let mut force_strip_session_affinity_after_challenge = false;
    for (idx, (account, mut token)) in candidates.into_iter().enumerate() {
        if deadline::is_expired(request_deadline) {
            let request = request
                .take()
                .expect("request should be available before timeout response");
            respond_total_timeout(
                request,
                context,
                trace_id,
                started_at,
                model_for_log,
                Some(attempted_account_ids.as_slice()),
            )?;
            return Ok(CandidateExecutionResult::Handled);
        }

        let strip_session_affinity = force_strip_session_affinity_after_challenge
            || state.strip_session_affinity(&account, idx, setup.anthropic_has_thread_anchor);
        let attempt_thread = super::super::super::conversation_binding::resolve_attempt_thread(
            setup.conversation_routing.as_ref(),
            &account,
        );
        let attempt_headers = attempt_thread
            .as_ref()
            .map(|thread| {
                incoming_headers.with_thread_affinity_override(
                    Some(thread.thread_anchor.as_str()),
                    thread.reset_session_affinity,
                )
            })
            .unwrap_or_else(|| incoming_headers.clone());
        let attempt_model_override = account_model_override(storage, model_for_log, &account)
            .or_else(|| free_account_model_override(storage, &account, &token));
        let attempt_allow_openai_fallback =
            allow_openai_fallback && allow_openai_fallback_for_account(storage, &account, &token);
        let attempt_model_for_log = attempt_model_override.as_deref().or(model_for_log);
        let attempt_prompt_cache_key =
            if should_forward_thread_anchor_as_prompt_cache_key(context.protocol_type()) {
                attempt_thread
                    .as_ref()
                    .map(|thread| thread.thread_anchor.as_str())
            } else {
                None
            };
        let body_for_attempt = state.body_for_attempt(
            path,
            body,
            strip_session_affinity,
            setup,
            attempt_model_override.as_deref(),
            attempt_prompt_cache_key,
        );
        context.log_candidate_start(&account.id, idx, strip_session_affinity);
        if let Some(skip_reason) = context.should_skip_candidate(&account.id, idx) {
            context.log_candidate_skip(&account.id, idx, skip_reason);
            match skip_reason {
                super::super::support::candidates::CandidateSkipReason::Cooldown => {
                    skipped_cooldown += 1;
                }
                super::super::support::candidates::CandidateSkipReason::Inflight => {
                    skipped_inflight += 1;
                }
            }
            continue;
        }
        attempted_account_ids.push(account.id.clone());

        let request_ref = request
            .as_ref()
            .ok_or_else(|| "request already consumed".to_string())?;
        let request_ctx =
            UpstreamRequestContext::from_request(request_ref, context.protocol_type());
        let incoming_session_id = attempt_headers.session_id();
        let incoming_turn_state = attempt_headers.turn_state();
        let incoming_conversation_id = attempt_headers.conversation_id();
        let prompt_cache_key_for_trace =
            extract_prompt_cache_key_for_trace(body_for_attempt.as_ref());
        super::super::super::trace_log::log_attempt_profile(
            super::super::super::trace_log::AttemptProfileLog {
                trace_id,
                account_id: &account.id,
                candidate_index: idx,
                total: setup.candidate_count,
                strip_session_affinity,
                has_incoming_session: incoming_session_id.is_some()
                    || setup.has_sticky_fallback_session,
                has_incoming_turn_state: incoming_turn_state.is_some(),
                has_incoming_conversation: incoming_conversation_id.is_some()
                    || setup.has_sticky_fallback_conversation,
                prompt_cache_key: prompt_cache_key_for_trace.as_deref(),
                request_shape,
                body_len: body_for_attempt.len(),
                body_model: attempt_model_for_log,
            },
        );

        let mut inflight_guard = Some(super::super::super::acquire_account_inflight(&account.id));
        let mut attempt_trace = CandidateAttemptTrace::default();
        let decision = run_candidate_attempt(CandidateAttemptParams {
            storage,
            method,
            request_ctx,
            incoming_headers: &attempt_headers,
            body: &body_for_attempt,
            upstream_is_stream,
            path,
            request_deadline,
            account: &account,
            token: &mut token,
            strip_session_affinity,
            debug,
            allow_openai_fallback: attempt_allow_openai_fallback,
            disable_challenge_stateless_retry,
            has_more_candidates: context.has_more_candidates(idx),
            context,
            setup,
            trace: &mut attempt_trace,
        });

        match decision {
            CandidateUpstreamDecision::Failover => {
                if should_force_strip_after_anthropic_challenge(context, &attempt_trace) {
                    force_strip_session_affinity_after_challenge = true;
                    log::warn!(
                        "event=gateway_anthropic_challenge_strip_affinity trace_id={} account_id={} next_attempt_strip_session_affinity=true",
                        trace_id,
                        account.id
                    );
                }
                record_failover_attempt(
                    &mut attempt_trace,
                    &mut last_attempt_url,
                    &mut last_attempt_error,
                );
                continue;
            }
            CandidateUpstreamDecision::Terminal {
                status_code,
                message,
            } => {
                if should_failover_terminal_gateway_error(
                    context,
                    &account.id,
                    context.has_more_candidates(idx),
                    &message,
                    &mut attempt_trace,
                    &mut last_attempt_url,
                    &mut last_attempt_error,
                ) {
                    if context.protocol_type() == crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE
                        && is_challenge_failover_error(Some(message.as_str()))
                    {
                        force_strip_session_affinity_after_challenge = true;
                        log::warn!(
                            "event=gateway_anthropic_challenge_strip_affinity trace_id={} account_id={} next_attempt_strip_session_affinity=true",
                            trace_id,
                            account.id
                        );
                    }
                    continue;
                }
                let request = request
                    .take()
                    .expect("request should be available before terminal response");
                return respond_terminal_attempt(
                    request,
                    context,
                    &account.id,
                    attempt_trace.last_attempt_url.as_deref(),
                    status_code,
                    message,
                    trace_id,
                    started_at,
                    attempt_model_for_log,
                    Some(attempted_account_ids.as_slice()),
                );
            }
            CandidateUpstreamDecision::RespondUpstream(mut resp) => {
                if resp.status().as_u16() == 400
                    && !strip_session_affinity
                    && (incoming_turn_state.is_some() || setup.has_body_encrypted_content)
                {
                    let retry_body = state.retry_body(
                        path,
                        body,
                        setup,
                        attempt_model_override.as_deref(),
                        attempt_prompt_cache_key,
                    );
                    let retry_decision = run_candidate_attempt(CandidateAttemptParams {
                        storage,
                        method,
                        request_ctx,
                        incoming_headers: &attempt_headers,
                        body: &retry_body,
                        upstream_is_stream,
                        path,
                        request_deadline,
                        account: &account,
                        token: &mut token,
                        strip_session_affinity: true,
                        debug,
                        allow_openai_fallback: attempt_allow_openai_fallback,
                        disable_challenge_stateless_retry,
                        has_more_candidates: context.has_more_candidates(idx),
                        context,
                        setup,
                        trace: &mut attempt_trace,
                    });

                    match retry_decision {
                        CandidateUpstreamDecision::RespondUpstream(retry_resp) => {
                            resp = retry_resp;
                        }
                        CandidateUpstreamDecision::Failover => {
                            record_failover_attempt(
                                &mut attempt_trace,
                                &mut last_attempt_url,
                                &mut last_attempt_error,
                            );
                            continue;
                        }
                        CandidateUpstreamDecision::Terminal {
                            status_code,
                            message,
                        } => {
                            let request = request
                                .take()
                                .expect("request should be available before terminal response");
                            return respond_terminal_attempt(
                                request,
                                context,
                                &account.id,
                                attempt_trace.last_attempt_url.as_deref(),
                                status_code,
                                message,
                                trace_id,
                                started_at,
                                attempt_model_for_log,
                                Some(attempted_account_ids.as_slice()),
                            );
                        }
                    }
                }
                let request = request
                    .take()
                    .expect("request should be available before terminal response");
                let guard = inflight_guard
                    .take()
                    .expect("inflight guard should be available before terminal response");
                let response_status = resp.status().as_u16();
                match finalize_upstream_response(
                    request,
                    resp,
                    guard,
                    context,
                    &account.id,
                    attempt_trace.last_attempt_url.as_deref(),
                    attempt_trace.last_attempt_error.as_deref(),
                    response_adapter,
                    gemini_stream_output_mode,
                    tool_name_restore_map,
                    client_is_stream,
                    path,
                    trace_id,
                    started_at,
                    attempt_model_for_log,
                    Some(attempted_account_ids.as_slice()),
                    context.has_more_candidates(idx),
                )? {
                    FinalizeUpstreamResponseOutcome::Handled => {
                        if let Err(err) = super::super::super::conversation_binding::record_conversation_binding_terminal_response(
                            storage,
                            setup.conversation_routing.as_ref(),
                            &account,
                            attempt_model_for_log,
                            response_status,
                        ) {
                            log::warn!(
                                "event=gateway_conversation_binding_update_failed trace_id={} account_id={} err={}",
                                trace_id,
                                account.id,
                                err
                            );
                        }
                        return Ok(CandidateExecutionResult::Handled);
                    }
                    FinalizeUpstreamResponseOutcome::Failover => {
                        record_failover_attempt(
                            &mut attempt_trace,
                            &mut last_attempt_url,
                            &mut last_attempt_error,
                        );
                        continue;
                    }
                }
            }
        }
    }

    Ok(CandidateExecutionResult::Exhausted {
        request: request
            .expect("request should still exist when no candidate handled the response"),
        attempted_account_ids,
        skipped_cooldown,
        skipped_inflight,
        last_attempt_url,
        last_attempt_error,
    })
}

#[cfg(test)]
mod tests {
    use super::{is_challenge_failover_error, should_forward_thread_anchor_as_prompt_cache_key};

    #[test]
    fn gemini_native_does_not_forward_thread_anchor_as_prompt_cache_key() {
        assert!(!should_forward_thread_anchor_as_prompt_cache_key(
            crate::apikey_profile::PROTOCOL_GEMINI_NATIVE
        ));
    }

    #[test]
    fn non_gemini_protocols_keep_thread_anchor_forwarding() {
        assert!(should_forward_thread_anchor_as_prompt_cache_key(
            crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE
        ));
        assert!(should_forward_thread_anchor_as_prompt_cache_key(
            crate::apikey_profile::PROTOCOL_OPENAI_COMPAT
        ));
    }

    #[test]
    fn challenge_failover_error_detection_matches_cloudflare_markers() {
        assert!(is_challenge_failover_error(Some(
            "upstream challenge blocked"
        )));
        assert!(is_challenge_failover_error(Some(
            "Cloudflare 安全验证页 [cf_ray=abc]"
        )));
        assert!(!is_challenge_failover_error(Some("upstream rate-limited")));
        assert!(!is_challenge_failover_error(None));
    }
}
