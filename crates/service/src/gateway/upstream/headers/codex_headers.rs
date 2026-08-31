const X_CODEX_INSTALLATION_ID_HEADER_NAME: &str = "x-codex-installation-id";
const X_CODEX_WINDOW_ID_HEADER_NAME: &str = "x-codex-window-id";
const X_CODEX_PARENT_THREAD_ID_HEADER_NAME: &str = "x-codex-parent-thread-id";
const X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER_NAME: &str =
    "x-responsesapi-include-timing-metrics";
const X_CODEX_INFERENCE_CALL_ID_HEADER_NAME: &str = "x-codex-inference-call-id";
const X_OAI_ATTESTATION_HEADER_NAME: &str = "x-oai-attestation";

fn anchor_fingerprint_or_dash(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(crate::gateway::anchor_fingerprint::fingerprint_anchor)
        .unwrap_or_else(|| "-".to_string())
}

fn normalize_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn looks_like_codex_identity(value: &str) -> bool {
    value.to_ascii_lowercase().contains("codex")
}

fn resolve_originator_header(
    incoming_originator: Option<&str>,
    preserve_client_identity: bool,
) -> String {
    normalize_non_empty(incoming_originator)
        .filter(|value| preserve_client_identity || looks_like_codex_identity(value))
        .map(str::to_string)
        .unwrap_or_else(crate::gateway::current_wire_originator)
}

fn resolve_user_agent_header(
    incoming_user_agent: Option<&str>,
    preserve_client_identity: bool,
) -> String {
    normalize_non_empty(incoming_user_agent)
        .filter(|value| preserve_client_identity || looks_like_codex_identity(value))
        .map(str::to_string)
        .unwrap_or_else(crate::gateway::current_codex_user_agent)
}

pub(crate) struct CodexUpstreamHeaderInput<'a> {
    pub(crate) auth_token: &'a str,
    pub(crate) chatgpt_account_id: Option<&'a str>,
    pub(crate) incoming_user_agent: Option<&'a str>,
    pub(crate) incoming_originator: Option<&'a str>,
    pub(crate) preserve_client_identity: bool,
    pub(crate) incoming_session_id: Option<&'a str>,
    pub(crate) incoming_window_id: Option<&'a str>,
    pub(crate) incoming_client_request_id: Option<&'a str>,
    pub(crate) incoming_subagent: Option<&'a str>,
    pub(crate) incoming_beta_features: Option<&'a str>,
    pub(crate) incoming_turn_metadata: Option<&'a str>,
    pub(crate) incoming_parent_thread_id: Option<&'a str>,
    pub(crate) incoming_responsesapi_include_timing_metrics: Option<&'a str>,
    pub(crate) incoming_inference_call_id: Option<&'a str>,
    pub(crate) incoming_oai_attestation: Option<&'a str>,
    pub(crate) passthrough_codex_headers: &'a [(String, String)],
    pub(crate) fallback_session_id: Option<&'a str>,
    pub(crate) incoming_turn_state: Option<&'a str>,
    pub(crate) include_turn_state: bool,
    pub(crate) strip_session_affinity: bool,
    pub(crate) has_body: bool,
}

pub(crate) struct CodexCompactUpstreamHeaderInput<'a> {
    pub(crate) auth_token: &'a str,
    pub(crate) chatgpt_account_id: Option<&'a str>,
    pub(crate) installation_id: Option<&'a str>,
    pub(crate) incoming_user_agent: Option<&'a str>,
    pub(crate) incoming_originator: Option<&'a str>,
    pub(crate) preserve_client_identity: bool,
    pub(crate) incoming_session_id: Option<&'a str>,
    pub(crate) thread_id: Option<&'a str>,
    pub(crate) incoming_window_id: Option<&'a str>,
    pub(crate) incoming_subagent: Option<&'a str>,
    pub(crate) incoming_parent_thread_id: Option<&'a str>,
    pub(crate) incoming_oai_attestation: Option<&'a str>,
    pub(crate) passthrough_codex_headers: &'a [(String, String)],
    pub(crate) fallback_session_id: Option<&'a str>,
    pub(crate) strip_session_affinity: bool,
    pub(crate) has_body: bool,
}

pub(crate) fn resolve_codex_installation_id(
    incoming_installation_id: Option<&str>,
) -> Option<String> {
    normalize_non_empty(incoming_installation_id)
        .map(str::to_string)
        .or_else(|| {
            crate::process_env::resolve_installation_id()
                .inspect_err(|err| {
                    log::warn!("event=gateway_installation_id_resolve_failed error={}", err);
                })
                .ok()
        })
}

/// 函数 `build_codex_upstream_headers`
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
pub(crate) fn build_codex_upstream_headers(
    input: CodexUpstreamHeaderInput<'_>,
) -> Vec<(String, String)> {
    let user_agent =
        resolve_user_agent_header(input.incoming_user_agent, input.preserve_client_identity);
    let originator =
        resolve_originator_header(input.incoming_originator, input.preserve_client_identity);
    let mut headers = Vec::with_capacity(16);
    headers.push((
        "Authorization".to_string(),
        format!("Bearer {}", input.auth_token),
    ));
    if let Some(account_id) = input
        .chatgpt_account_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push(("ChatGPT-Account-ID".to_string(), account_id.to_string()));
    }
    if input.has_body {
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
    }
    headers.push(("Accept".to_string(), "text/event-stream".to_string()));
    headers.push(("User-Agent".to_string(), user_agent));
    headers.push(("originator".to_string(), originator));
    if let Some(residency_requirement) = crate::gateway::current_residency_requirement() {
        headers.push((
            crate::gateway::runtime_config::RESIDENCY_HEADER_NAME.to_string(),
            residency_requirement,
        ));
    }
    if let Some(client_request_id) = resolve_client_request_id(input.incoming_client_request_id) {
        headers.push(("x-client-request-id".to_string(), client_request_id));
    }
    if let Some(subagent) = input
        .incoming_subagent
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push(("x-openai-subagent".to_string(), subagent.to_string()));
    }
    if let Some(beta_features) = input
        .incoming_beta_features
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            "x-codex-beta-features".to_string(),
            beta_features.to_string(),
        ));
    }
    if let Some(turn_metadata) = input
        .incoming_turn_metadata
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            "x-codex-turn-metadata".to_string(),
            turn_metadata.to_string(),
        ));
    }
    if let Some(parent_thread_id) = input
        .incoming_parent_thread_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            X_CODEX_PARENT_THREAD_ID_HEADER_NAME.to_string(),
            parent_thread_id.to_string(),
        ));
    }
    if let Some(include_timing_metrics) = input
        .incoming_responsesapi_include_timing_metrics
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER_NAME.to_string(),
            include_timing_metrics.to_string(),
        ));
    }
    if let Some(inference_call_id) = input
        .incoming_inference_call_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            X_CODEX_INFERENCE_CALL_ID_HEADER_NAME.to_string(),
            inference_call_id.to_string(),
        ));
    }
    if let Some(oai_attestation) = input
        .incoming_oai_attestation
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            X_OAI_ATTESTATION_HEADER_NAME.to_string(),
            oai_attestation.to_string(),
        ));
    }
    let resolved_session_id = resolve_optional_session_id(
        input.incoming_session_id,
        input.fallback_session_id,
        input.strip_session_affinity,
    );
    if let Some(session_id) = resolved_session_id.as_deref() {
        headers.push(("session_id".to_string(), session_id.to_string()));
    }
    if let Some(window_id) = resolve_window_id(
        input.incoming_window_id,
        resolved_session_id.as_deref(),
        input.strip_session_affinity,
    ) {
        headers.push((X_CODEX_WINDOW_ID_HEADER_NAME.to_string(), window_id));
    }
    append_passthrough_codex_headers(
        &mut headers,
        input.passthrough_codex_headers,
        !input.strip_session_affinity,
    );

    if !input.strip_session_affinity {
        if input.include_turn_state {
            if let Some(turn_state) = input.incoming_turn_state {
                headers.push(("x-codex-turn-state".to_string(), turn_state.to_string()));
            }
        }
    }

    headers
}

/// 函数 `build_codex_compact_upstream_headers`
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
pub(crate) fn build_codex_compact_upstream_headers(
    input: CodexCompactUpstreamHeaderInput<'_>,
) -> Vec<(String, String)> {
    let user_agent =
        resolve_user_agent_header(input.incoming_user_agent, input.preserve_client_identity);
    let originator =
        resolve_originator_header(input.incoming_originator, input.preserve_client_identity);
    let mut headers = Vec::with_capacity(13);
    headers.push((
        "Authorization".to_string(),
        format!("Bearer {}", input.auth_token),
    ));
    if let Some(account_id) = input
        .chatgpt_account_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push(("ChatGPT-Account-ID".to_string(), account_id.to_string()));
    }
    if let Some(installation_id) = input
        .installation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            X_CODEX_INSTALLATION_ID_HEADER_NAME.to_string(),
            installation_id.to_string(),
        ));
    }
    if input.has_body {
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
    }
    headers.push(("Accept".to_string(), "application/json".to_string()));
    headers.push(("User-Agent".to_string(), user_agent));
    headers.push(("originator".to_string(), originator));
    if let Some(residency_requirement) = crate::gateway::current_residency_requirement() {
        headers.push((
            crate::gateway::runtime_config::RESIDENCY_HEADER_NAME.to_string(),
            residency_requirement,
        ));
    }
    if let Some(subagent) = input
        .incoming_subagent
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push(("x-openai-subagent".to_string(), subagent.to_string()));
    }
    if let Some(parent_thread_id) = input
        .incoming_parent_thread_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            X_CODEX_PARENT_THREAD_ID_HEADER_NAME.to_string(),
            parent_thread_id.to_string(),
        ));
    }
    if let Some(oai_attestation) = input
        .incoming_oai_attestation
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            X_OAI_ATTESTATION_HEADER_NAME.to_string(),
            oai_attestation.to_string(),
        ));
    }
    let resolved_session_id = resolve_optional_session_id(
        input.incoming_session_id,
        input.fallback_session_id,
        input.strip_session_affinity,
    );
    if let Some(session_id) = resolved_session_id.clone() {
        headers.push(("session_id".to_string(), session_id));
    }
    if let Some(thread_id) = normalize_non_empty(input.thread_id) {
        headers.push(("thread_id".to_string(), thread_id.to_string()));
    }
    if let Some(window_id) = resolve_window_id(
        input.incoming_window_id,
        resolved_session_id.as_deref(),
        input.strip_session_affinity,
    ) {
        headers.push((X_CODEX_WINDOW_ID_HEADER_NAME.to_string(), window_id));
    }
    append_passthrough_codex_headers(
        &mut headers,
        input.passthrough_codex_headers,
        !input.strip_session_affinity,
    );
    headers
}

/// 函数 `resolve_optional_session_id`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - incoming: 参数 incoming
/// - fallback_session_id: 参数 fallback_session_id
/// - strip_session_affinity: 参数 strip_session_affinity
///
/// # 返回
/// 返回函数执行结果
fn resolve_optional_session_id(
    incoming: Option<&str>,
    fallback_session_id: Option<&str>,
    strip_session_affinity: bool,
) -> Option<String> {
    if strip_session_affinity {
        return fallback_session_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
    }
    if let Some(value) = incoming {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    fallback_session_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn resolve_window_id(
    incoming_window_id: Option<&str>,
    resolved_session_id: Option<&str>,
    strip_session_affinity: bool,
) -> Option<String> {
    let normalized_session_id = resolved_session_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if !strip_session_affinity {
        if let Some(window_id) = incoming_window_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let matches_session = match normalized_session_id {
                Some(session_id) => {
                    window_id == session_id
                        || window_id.starts_with(format!("{session_id}:").as_str())
                }
                None => true,
            };
            if matches_session {
                return Some(window_id.to_string());
            }
            log::info!(
                "event=gateway_window_id_rebuilt reason=session_mismatch incoming_window_fp={} resolved_session_fp={}",
                anchor_fingerprint_or_dash(Some(window_id)),
                anchor_fingerprint_or_dash(normalized_session_id),
            );
        }
    }
    normalized_session_id.map(|session_id| format!("{session_id}:0"))
}

fn append_passthrough_codex_headers(
    headers: &mut Vec<(String, String)>,
    passthrough_headers: &[(String, String)],
    enabled: bool,
) {
    // 中文注释：Codex wire shape 不接受额外透传头；这里保留参数只为兼容调用签名，
    // 但实际行为是完全丢弃。
    let _ = headers;
    let _ = passthrough_headers;
    let _ = enabled;
}

/// 函数 `resolve_client_request_id`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - incoming_client_request_id: 参数 incoming_client_request_id
///
/// # 返回
/// 返回函数执行结果
fn resolve_client_request_id(incoming_client_request_id: Option<&str>) -> Option<String> {
    if let Some(value) = incoming_client_request_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(value.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        build_codex_compact_upstream_headers, build_codex_upstream_headers,
        resolve_codex_installation_id,
    };
    use crate::gateway::{
        set_codex_user_agent_version, set_originator, CodexCompactUpstreamHeaderInput,
        CodexUpstreamHeaderInput,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    const CODEXMANAGER_DB_PATH_ENV: &str = "CODEXMANAGER_DB_PATH";

    struct RuntimeEnvGuard {
        name: &'static str,
        previous_value: Option<String>,
    }

    impl RuntimeEnvGuard {
        fn set(name: &'static str, value: &str) -> Self {
            let previous_value = std::env::var(name).ok();
            std::env::set_var(name, value);
            crate::gateway::reload_runtime_config_from_env();
            Self {
                name,
                previous_value,
            }
        }
    }

    impl Drop for RuntimeEnvGuard {
        fn drop(&mut self) {
            match self.previous_value.as_deref() {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
            crate::gateway::reload_runtime_config_from_env();
        }
    }

    fn isolated_db_path(label: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "codexmanager-codex-headers-{}-{}-{}.db",
                label,
                std::process::id(),
                nanos
            ))
            .to_string_lossy()
            .into_owned()
    }

    /// 函数 `header_value`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - headers: 参数 headers
    /// - name: 参数 name
    ///
    /// # 返回
    /// 返回函数执行结果
    fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn resolve_codex_installation_id_prefers_incoming_header() {
        let _guard = crate::test_env_guard();

        assert_eq!(
            resolve_codex_installation_id(Some(" install-from-client ")).as_deref(),
            Some("install-from-client")
        );
    }

    #[test]
    fn resolve_codex_installation_id_uses_persisted_fallback_when_incoming_missing() {
        let _guard = crate::test_env_guard();
        let _db_guard = RuntimeEnvGuard::set(
            CODEXMANAGER_DB_PATH_ENV,
            isolated_db_path("compact-installation-id").as_str(),
        );

        let first = resolve_codex_installation_id(None).expect("first installation id");
        let second = resolve_codex_installation_id(None).expect("second installation id");

        assert_eq!(first, second);
        assert_eq!(first.len(), 36);
        assert_eq!(first.as_bytes().get(14).copied(), Some(b'4'));
    }

    /// 函数 `build_codex_upstream_headers_keeps_final_affinity_shape`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn build_codex_upstream_headers_keeps_final_affinity_shape() {
        let _guard = crate::test_env_guard();
        let _ = set_originator("codex_cli_rs_tests").expect("set originator");
        let _ = set_codex_user_agent_version("0.999.0").expect("set ua version");
        let passthrough = vec![(
            "x-codex-other-limit-name".to_string(),
            "promo_header_a".to_string(),
        )];

        let headers = build_codex_upstream_headers(CodexUpstreamHeaderInput {
            auth_token: "token-123",
            chatgpt_account_id: Some("account-123"),
            incoming_user_agent: None,
            incoming_originator: None,
            preserve_client_identity: false,
            incoming_session_id: Some("conversation-anchor"),
            incoming_window_id: Some("conversation-anchor:7"),
            incoming_client_request_id: Some("conversation-anchor"),
            incoming_subagent: Some("subagent-a"),
            incoming_beta_features: Some("beta-a"),
            incoming_turn_metadata: Some("meta-a"),
            incoming_parent_thread_id: Some("thread-parent-a"),
            incoming_responsesapi_include_timing_metrics: Some("true"),
            incoming_inference_call_id: Some("inference-call-a"),
            incoming_oai_attestation: Some("attestation-a"),
            passthrough_codex_headers: passthrough.as_slice(),
            fallback_session_id: Some("conversation-anchor"),
            incoming_turn_state: Some("turn-state-a"),
            include_turn_state: true,
            strip_session_affinity: false,
            has_body: true,
        });

        assert_eq!(
            header_value(&headers, "Authorization"),
            Some("Bearer token-123")
        );
        assert_eq!(
            header_value(&headers, "ChatGPT-Account-ID"),
            Some("account-123")
        );
        assert_eq!(
            header_value(&headers, "Content-Type"),
            Some("application/json")
        );
        assert_eq!(header_value(&headers, "Accept"), Some("text/event-stream"));
        assert_eq!(header_value(&headers, "OpenAI-Beta"), None);
        assert_eq!(
            header_value(&headers, "x-responsesapi-include-timing-metrics"),
            Some("true")
        );
        assert_eq!(
            header_value(&headers, "x-codex-inference-call-id"),
            Some("inference-call-a")
        );
        assert_eq!(
            header_value(&headers, "x-oai-attestation"),
            Some("attestation-a")
        );
        let expected_user_agent_prefix =
            format!("{}/0.999.0", crate::gateway::current_wire_originator());
        assert_eq!(
            header_value(&headers, "User-Agent")
                .map(|value| value.starts_with(expected_user_agent_prefix.as_str())),
            Some(true)
        );
        assert_eq!(
            header_value(&headers, "originator"),
            Some("codex_cli_rs_tests")
        );
        assert_eq!(header_value(&headers, "version"), None);
        assert_eq!(header_value(&headers, "OpenAI-Organization"), None);
        assert_eq!(header_value(&headers, "OpenAI-Project"), None);
        assert_eq!(
            header_value(&headers, "x-client-request-id"),
            Some("conversation-anchor")
        );
        assert_eq!(
            header_value(&headers, "session_id"),
            Some("conversation-anchor")
        );
        assert_eq!(
            header_value(&headers, "x-codex-window-id"),
            Some("conversation-anchor:7")
        );
        assert_eq!(
            header_value(&headers, "x-codex-turn-state"),
            Some("turn-state-a")
        );
        assert_eq!(
            header_value(&headers, "x-codex-parent-thread-id"),
            Some("thread-parent-a")
        );
        assert_eq!(header_value(&headers, "x-codex-other-limit-name"), None);
    }

    /// 函数 `build_codex_upstream_headers_clears_turn_state_when_affinity_diverges`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn build_codex_upstream_headers_clears_turn_state_when_affinity_diverges() {
        let _guard = crate::test_env_guard();
        let _ = set_originator("codex_cli_rs_tests").expect("set originator");
        let _ = set_codex_user_agent_version("0.999.1").expect("set ua version");
        let passthrough = vec![(
            "x-codex-other-limit-name".to_string(),
            "promo_header_b".to_string(),
        )];

        let headers = build_codex_upstream_headers(CodexUpstreamHeaderInput {
            auth_token: "token-456",
            chatgpt_account_id: None,
            incoming_user_agent: None,
            incoming_originator: None,
            preserve_client_identity: false,
            incoming_session_id: Some("conversation-anchor"),
            incoming_window_id: Some("conversation-anchor:9"),
            incoming_client_request_id: Some("conversation-anchor"),
            incoming_subagent: None,
            incoming_beta_features: None,
            incoming_turn_metadata: None,
            incoming_parent_thread_id: Some("thread-parent-b"),
            incoming_responsesapi_include_timing_metrics: None,
            incoming_inference_call_id: None,
            incoming_oai_attestation: None,
            passthrough_codex_headers: passthrough.as_slice(),
            fallback_session_id: Some("prompt-cache-anchor"),
            incoming_turn_state: None,
            include_turn_state: true,
            strip_session_affinity: false,
            has_body: false,
        });

        assert_eq!(header_value(&headers, "Accept"), Some("text/event-stream"));
        assert_eq!(
            header_value(&headers, "x-client-request-id"),
            Some("conversation-anchor")
        );
        assert_eq!(
            header_value(&headers, "session_id"),
            Some("conversation-anchor")
        );
        assert_eq!(
            header_value(&headers, "x-codex-window-id"),
            Some("conversation-anchor:9")
        );
        assert_eq!(header_value(&headers, "x-codex-turn-state"), None);
        assert_eq!(
            header_value(&headers, "x-codex-parent-thread-id"),
            Some("thread-parent-b")
        );
        assert_eq!(header_value(&headers, "x-codex-other-limit-name"), None);
    }

    /// 函数 `build_codex_compact_upstream_headers_use_session_fallback_only`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn build_codex_compact_upstream_headers_use_session_fallback_only() {
        let _guard = crate::test_env_guard();
        let _ = set_originator("codex_cli_rs_tests").expect("set originator");
        let _ = set_codex_user_agent_version("0.999.2").expect("set ua version");
        let passthrough = vec![(
            "x-codex-other-limit-name".to_string(),
            "promo_header_c".to_string(),
        )];

        let headers = build_codex_compact_upstream_headers(CodexCompactUpstreamHeaderInput {
            auth_token: "token-789",
            chatgpt_account_id: Some("account-compact"),
            installation_id: Some("install-compact-internal"),
            incoming_user_agent: None,
            incoming_originator: None,
            preserve_client_identity: false,
            incoming_session_id: None,
            thread_id: Some("thread-anchor-c"),
            incoming_window_id: Some("conversation-anchor:11"),
            incoming_subagent: Some("subagent-b"),
            incoming_parent_thread_id: Some("thread-parent-c"),
            incoming_oai_attestation: Some("attestation-compact"),
            passthrough_codex_headers: passthrough.as_slice(),
            fallback_session_id: Some("conversation-anchor"),
            strip_session_affinity: true,
            has_body: true,
        });

        assert_eq!(header_value(&headers, "Accept"), Some("application/json"));
        assert_eq!(
            header_value(&headers, "ChatGPT-Account-ID"),
            Some("account-compact")
        );
        assert_eq!(
            header_value(&headers, "x-codex-installation-id"),
            Some("install-compact-internal")
        );
        assert_eq!(
            header_value(&headers, "x-oai-attestation"),
            Some("attestation-compact")
        );
        assert_eq!(header_value(&headers, "x-client-request-id"), None);
        assert_eq!(
            header_value(&headers, "session_id"),
            Some("conversation-anchor")
        );
        assert_eq!(header_value(&headers, "thread_id"), Some("thread-anchor-c"));
        assert_eq!(
            header_value(&headers, "x-codex-window-id"),
            Some("conversation-anchor:0")
        );
        assert_eq!(header_value(&headers, "x-codex-turn-state"), None);
        assert_eq!(header_value(&headers, "OpenAI-Beta"), None);
        assert_eq!(
            header_value(&headers, "x-responsesapi-include-timing-metrics"),
            None
        );
        assert_eq!(header_value(&headers, "version"), None);
        assert_eq!(
            header_value(&headers, "x-openai-subagent"),
            Some("subagent-b")
        );
        assert_eq!(
            header_value(&headers, "x-codex-parent-thread-id"),
            Some("thread-parent-c")
        );
        assert_eq!(header_value(&headers, "x-codex-other-limit-name"), None);
    }

    #[test]
    fn build_codex_upstream_headers_rebuilds_mismatched_window_id_from_session() {
        let _guard = crate::test_env_guard();
        let _ = set_originator("codex_cli_rs_tests").expect("set originator");
        let _ = set_codex_user_agent_version("0.999.3").expect("set ua version");

        let headers = build_codex_upstream_headers(CodexUpstreamHeaderInput {
            auth_token: "token-window-fix",
            chatgpt_account_id: None,
            incoming_user_agent: None,
            incoming_originator: None,
            preserve_client_identity: false,
            incoming_session_id: Some("session-anchor"),
            incoming_window_id: Some("stale-window-anchor:9"),
            incoming_client_request_id: Some("request-anchor"),
            incoming_subagent: None,
            incoming_beta_features: None,
            incoming_turn_metadata: None,
            incoming_parent_thread_id: None,
            incoming_responsesapi_include_timing_metrics: None,
            incoming_inference_call_id: None,
            incoming_oai_attestation: None,
            passthrough_codex_headers: &[],
            fallback_session_id: Some("fallback-anchor"),
            incoming_turn_state: Some("turn-state-window-fix"),
            include_turn_state: true,
            strip_session_affinity: false,
            has_body: true,
        });

        assert_eq!(header_value(&headers, "session_id"), Some("session-anchor"));
        assert_eq!(
            header_value(&headers, "x-codex-window-id"),
            Some("session-anchor:0")
        );
    }

    #[test]
    fn build_codex_upstream_headers_prefers_incoming_codex_identity() {
        let _guard = crate::test_env_guard();
        let _ = set_originator("codex_cli_rs_tests").expect("set originator");
        let _ = set_codex_user_agent_version("0.999.4").expect("set ua version");

        let headers = build_codex_upstream_headers(CodexUpstreamHeaderInput {
            auth_token: "token-ident",
            chatgpt_account_id: None,
            incoming_user_agent: Some("codex_sdk_ts/1.2.3 (Windows 11; x86_64) node"),
            incoming_originator: Some("codex_sdk_ts"),
            preserve_client_identity: false,
            incoming_session_id: Some("thread-ident"),
            incoming_window_id: Some("thread-ident:0"),
            incoming_client_request_id: Some("thread-ident"),
            incoming_subagent: None,
            incoming_beta_features: None,
            incoming_turn_metadata: None,
            incoming_parent_thread_id: None,
            incoming_responsesapi_include_timing_metrics: None,
            incoming_inference_call_id: None,
            incoming_oai_attestation: None,
            passthrough_codex_headers: &[],
            fallback_session_id: Some("thread-ident"),
            incoming_turn_state: None,
            include_turn_state: true,
            strip_session_affinity: false,
            has_body: true,
        });

        assert_eq!(header_value(&headers, "originator"), Some("codex_sdk_ts"));
        assert_eq!(
            header_value(&headers, "User-Agent"),
            Some("codex_sdk_ts/1.2.3 (Windows 11; x86_64) node")
        );
    }

    #[test]
    fn build_codex_upstream_headers_preserves_non_codex_identity_for_compat_routes() {
        let _guard = crate::test_env_guard();
        let _ = set_originator("codex_cli_rs_tests").expect("set originator");
        let _ = set_codex_user_agent_version("0.999.5").expect("set ua version");

        let headers = build_codex_upstream_headers(CodexUpstreamHeaderInput {
            auth_token: "token-compat",
            chatgpt_account_id: None,
            incoming_user_agent: Some("gemini-cli/0.1.14 (Windows 11; x86_64)"),
            incoming_originator: Some("gemini_cli"),
            preserve_client_identity: true,
            incoming_session_id: Some("thread-compat"),
            incoming_window_id: Some("thread-compat:0"),
            incoming_client_request_id: Some("thread-compat"),
            incoming_subagent: None,
            incoming_beta_features: None,
            incoming_turn_metadata: None,
            incoming_parent_thread_id: None,
            incoming_responsesapi_include_timing_metrics: None,
            incoming_inference_call_id: None,
            incoming_oai_attestation: None,
            passthrough_codex_headers: &[],
            fallback_session_id: Some("thread-compat"),
            incoming_turn_state: None,
            include_turn_state: true,
            strip_session_affinity: false,
            has_body: true,
        });

        assert_eq!(
            header_value(&headers, "User-Agent"),
            Some("gemini-cli/0.1.14 (Windows 11; x86_64)")
        );
        assert_eq!(header_value(&headers, "originator"), Some("gemini_cli"));
    }

    #[test]
    fn build_codex_compact_upstream_headers_omits_thread_id_when_missing() {
        let _guard = crate::test_env_guard();
        let headers = build_codex_compact_upstream_headers(CodexCompactUpstreamHeaderInput {
            auth_token: "token-thread-missing",
            chatgpt_account_id: None,
            installation_id: None,
            incoming_user_agent: None,
            incoming_originator: None,
            preserve_client_identity: false,
            incoming_session_id: Some("session-anchor"),
            thread_id: None,
            incoming_window_id: None,
            incoming_subagent: None,
            incoming_parent_thread_id: None,
            incoming_oai_attestation: None,
            passthrough_codex_headers: &[],
            fallback_session_id: Some("session-anchor"),
            strip_session_affinity: false,
            has_body: false,
        });

        assert_eq!(header_value(&headers, "thread_id"), None);
    }
}
