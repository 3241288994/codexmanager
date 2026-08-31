use super::{
    build_usage_request_headers, summarize_usage_error_response, usage_http_client,
    CHATGPT_ACCOUNT_ID_HEADER_NAME,
};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use reqwest::StatusCode;
use std::sync::MutexGuard;
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Response, Server, StatusCode as TinyStatusCode};

struct RecordedSubscriptionRequest {
    path: String,
    authorization: Option<String>,
    chatgpt_account_id: Option<String>,
    originator: Option<String>,
    residency: Option<String>,
    origin: Option<String>,
    referer: Option<String>,
    accept: Option<String>,
}

/// 函数 `usage_header_runtime_scope`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
fn usage_header_runtime_scope() -> (MutexGuard<'static, ()>, UsageHeaderRuntimeRestore) {
    let guard = crate::test_env_guard();
    let restore = UsageHeaderRuntimeRestore::capture();
    let _ = crate::gateway::set_originator("codex_cli_rs");
    let _ = crate::gateway::set_residency_requirement(None);
    (guard, restore)
}

struct UsageHeaderRuntimeRestore {
    originator: String,
    residency_requirement: Option<String>,
}

impl UsageHeaderRuntimeRestore {
    /// 函数 `capture`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 返回函数执行结果
    fn capture() -> Self {
        Self {
            originator: crate::gateway::current_originator(),
            residency_requirement: crate::gateway::current_residency_requirement(),
        }
    }
}

impl Drop for UsageHeaderRuntimeRestore {
    /// 函数 `drop`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    ///
    /// # 返回
    /// 无
    fn drop(&mut self) {
        let _ = crate::gateway::set_originator(&self.originator);
        let _ = crate::gateway::set_residency_requirement(self.residency_requirement.as_deref());
    }
}

/// 函数 `usage_http_client_is_cloneable`
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
fn usage_http_client_is_cloneable() {
    let first = usage_http_client();
    let second = usage_http_client();
    let first_ptr = &first as *const Client;
    let second_ptr = &second as *const Client;
    assert_ne!(first_ptr, second_ptr);
}

/// 函数 `refresh_token_status_error_omits_empty_body`
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
fn refresh_token_status_error_omits_empty_body() {
    assert_eq!(
        super::format_refresh_token_status_error(StatusCode::FORBIDDEN, "   "),
        "refresh token failed with status 403 Forbidden"
    );
}

/// 函数 `refresh_token_status_error_includes_body_snippet`
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
fn refresh_token_status_error_includes_body_snippet() {
    assert_eq!(
        super::format_refresh_token_status_error(
            StatusCode::BAD_REQUEST,
            "{\n  \"error\": \"invalid_grant\"\n}"
        ),
        "refresh token failed with status 400 Bad Request: invalid_grant"
    );
}

#[test]
fn refresh_token_status_error_maps_app_session_terminated_400_to_official_message() {
    assert_eq!(
        super::format_refresh_token_status_error(
            StatusCode::BAD_REQUEST,
            r#"{"code":"app_session_terminated","type":"invalid_request_error","message":"Your session has ended. Please log in again."}"#
        ),
        "refresh token failed with status 400 Bad Request: Your session has ended. Please log in again."
    );
}

#[test]
fn refresh_token_body_matches_codex_refresh_scope() {
    assert_eq!(
        super::build_refresh_token_body("client-id", "refresh-token"),
        "client_id=client-id&grant_type=refresh_token&refresh_token=refresh-token&scope=openid+profile+email"
    );
}

/// 函数 `refresh_token_status_error_maps_invalidated_401_to_official_message`
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
fn refresh_token_status_error_maps_invalidated_401_to_official_message() {
    assert_eq!(
        super::format_refresh_token_status_error(
            StatusCode::UNAUTHORIZED,
            "{\"error\":\"refresh_token_invalidated\"}"
        ),
        "refresh token failed with status 401 Unauthorized: Your access token could not be refreshed because your refresh token was revoked. Please log out and sign in again."
    );
}

/// 函数 `refresh_token_status_error_maps_unknown_401_to_official_message`
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
fn refresh_token_status_error_maps_unknown_401_to_official_message() {
    assert_eq!(
        super::format_refresh_token_status_error(
            StatusCode::UNAUTHORIZED,
            "{\"error\":\"something_else\"}"
        ),
        "refresh token failed with status 401 Unauthorized: Your access token could not be refreshed. Please log out and sign in again."
    );
}

/// 函数 `classify_refresh_token_auth_error_reason_maps_known_and_unknown_401`
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
fn classify_refresh_token_auth_error_reason_maps_known_and_unknown_401() {
    assert_eq!(
        super::classify_refresh_token_auth_error_reason(
            StatusCode::UNAUTHORIZED,
            "{\"error\":\"refresh_token_invalidated\"}"
        ),
        Some(super::RefreshTokenAuthErrorReason::Invalidated)
    );
    assert_eq!(
        super::classify_refresh_token_auth_error_reason(
            StatusCode::UNAUTHORIZED,
            "{\"error\":\"something_else\"}"
        ),
        Some(super::RefreshTokenAuthErrorReason::Unknown401)
    );
    assert_eq!(
        super::classify_refresh_token_auth_error_reason(
            StatusCode::FORBIDDEN,
            "{\"error\":\"refresh_token_invalidated\"}"
        ),
        None
    );
}

/// 函数 `refresh_token_status_error_ignores_headers_for_401_reason_when_body_lacks_code`
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
fn refresh_token_status_error_ignores_headers_for_401_reason_when_body_lacks_code() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-error-json",
        HeaderValue::from_static("{\"identity_error_code\":\"refresh_token_invalidated\"}"),
    );
    headers.insert(
        "x-openai-authorization-error",
        HeaderValue::from_static("refresh_token_expired"),
    );

    assert_eq!(
        super::format_refresh_token_status_error_with_headers(
            StatusCode::UNAUTHORIZED,
            Some(&headers),
            "<html><title>Just a moment...</title></html>"
        ),
        "refresh token failed with status 401 Unauthorized: Your access token could not be refreshed. Please log out and sign in again."
    );
}

/// 函数 `refresh_token_status_error_stabilizes_html_and_debug_headers_for_non_401`
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
fn refresh_token_status_error_stabilizes_html_and_debug_headers_for_non_401() {
    let mut headers = HeaderMap::new();
    headers.insert("x-request-id", HeaderValue::from_static("req_refresh_123"));
    headers.insert("cf-ray", HeaderValue::from_static("cf_refresh_123"));
    headers.insert(
        "x-openai-authorization-error",
        HeaderValue::from_static("missing_authorization_header"),
    );
    headers.insert(
        "x-error-json",
        HeaderValue::from_static("{\"identity_error_code\":\"token_expired\"}"),
    );

    let message = super::format_refresh_token_status_error_with_headers(
        StatusCode::FORBIDDEN,
        Some(&headers),
        "<html><head><title>Just a moment...</title></head><body>challenge</body></html>",
    );

    assert!(message.contains("refresh token failed with status 403 Forbidden"));
    assert!(message.contains("Cloudflare 安全验证页"));
    assert!(message.contains("kind=cloudflare_challenge"));
    assert!(message.contains("request_id=req_refresh_123"));
    assert!(message.contains("cf_ray=cf_refresh_123"));
    assert!(message.contains("auth_error=missing_authorization_header"));
    assert!(message.contains("identity_error_code=token_expired"));
}

/// 函数 `refresh_token_status_error_uses_header_only_debug_suffix_for_empty_body`
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
fn refresh_token_status_error_uses_header_only_debug_suffix_for_empty_body() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-request-id",
        HeaderValue::from_static("req_refresh_empty"),
    );
    headers.insert("cf-ray", HeaderValue::from_static("cf_refresh_empty"));

    let message = super::format_refresh_token_status_error_with_headers(
        StatusCode::BAD_GATEWAY,
        Some(&headers),
        "",
    );

    assert!(message.contains("refresh token failed with status 502 Bad Gateway"));
    assert!(message.contains("kind=cloudflare_edge"));
    assert!(message.contains("request_id=req_refresh_empty"));
    assert!(message.contains("cf_ray=cf_refresh_empty"));
}

#[test]
fn refresh_token_status_error_detects_region_blocked_header_marker() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-openai-authorization-error",
        HeaderValue::from_static("unsupported_country_region_territory"),
    );
    headers.insert("cf-ray", HeaderValue::from_static("cf_refresh_region"));

    let message = super::format_refresh_token_status_error_with_headers(
        StatusCode::FORBIDDEN,
        Some(&headers),
        "",
    );

    assert!(message.contains("refresh token failed with status 403 Forbidden"));
    assert!(message.contains("kind=cloudflare_blocked"));
    assert!(message.contains("auth_error=unsupported_country_region_territory"));
    assert!(super::is_region_blocked_error_message(&message));
    assert!(super::is_refresh_token_region_blocked_error_message(
        &message
    ));
}

#[test]
fn refresh_token_status_error_plain_forbidden_is_not_region_blocked() {
    let message = super::format_refresh_token_status_error(StatusCode::FORBIDDEN, "");

    assert_eq!(message, "refresh token failed with status 403 Forbidden");
    assert!(!super::is_region_blocked_error_message(&message));
    assert!(!super::is_refresh_token_region_blocked_error_message(
        &message
    ));
}

/// 函数 `refresh_token_auth_error_reason_from_message_tracks_canonical_messages`
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
fn refresh_token_auth_error_reason_from_message_tracks_canonical_messages() {
    let invalidated = super::format_refresh_token_status_error(
        StatusCode::UNAUTHORIZED,
        "{\"error\":\"refresh_token_invalidated\"}",
    );
    assert_eq!(
        super::refresh_token_auth_error_reason_from_message(&invalidated),
        Some(super::RefreshTokenAuthErrorReason::Invalidated)
    );

    let unknown = super::format_refresh_token_status_error(
        StatusCode::UNAUTHORIZED,
        "{\"error\":\"something_else\"}",
    );
    assert_eq!(
        super::refresh_token_auth_error_reason_from_message(&unknown),
        Some(super::RefreshTokenAuthErrorReason::Unknown401)
    );

    let invalid_grant = super::format_refresh_token_status_error(
        StatusCode::BAD_REQUEST,
        "{\"error\":\"invalid_grant\"}",
    );
    assert_eq!(
        super::refresh_token_auth_error_reason_from_message(&invalid_grant),
        Some(super::RefreshTokenAuthErrorReason::InvalidGrant)
    );

    let app_session_terminated = super::format_refresh_token_status_error(
        StatusCode::BAD_REQUEST,
        r#"{"code":"app_session_terminated","type":"invalid_request_error","message":"Your session has ended. Please log in again."}"#,
    );
    assert_eq!(
        super::refresh_token_auth_error_reason_from_message(&app_session_terminated),
        Some(super::RefreshTokenAuthErrorReason::AppSessionTerminated)
    );

    let legacy_app_session_terminated =
        "refresh token failed with status 400 Bad Request: code=app_session_terminated type=invalid_request_error Your session has ended. Please log in again.";
    assert_eq!(
        super::refresh_token_auth_error_reason_from_message(legacy_app_session_terminated),
        Some(super::RefreshTokenAuthErrorReason::AppSessionTerminated)
    );
}

/// 函数 `usage_http_default_headers_follow_gateway_runtime_profile`
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
fn usage_http_default_headers_follow_gateway_runtime_profile() {
    let (_guard, _restore) = usage_header_runtime_scope();
    crate::gateway::set_originator("codex_cli_rs_usage").expect("set gateway originator");
    crate::gateway::set_residency_requirement(Some("us"))
        .expect("set gateway residency requirement");

    let headers = super::build_usage_http_default_headers();

    assert_eq!(
        headers
            .get("originator")
            .and_then(|value| value.to_str().ok()),
        Some("codex_cli_rs_usage")
    );
    assert_eq!(
        headers
            .get("x-openai-internal-codex-residency")
            .and_then(|value| value.to_str().ok()),
        Some("us")
    );
}

/// 函数 `usage_request_headers_use_official_chatgpt_account_header_name`
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
fn usage_request_headers_use_official_chatgpt_account_header_name() {
    let headers = build_usage_request_headers(Some("workspace_123"));

    assert_eq!(
        headers
            .get(CHATGPT_ACCOUNT_ID_HEADER_NAME)
            .and_then(|value| value.to_str().ok()),
        Some("workspace_123")
    );
    assert_eq!(headers.len(), 1);
}

/// 函数 `subscription_request_uses_only_authorization_without_custom_usage_headers`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-17
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn subscription_request_uses_only_authorization_without_custom_usage_headers() {
    let (_guard, _restore) = usage_header_runtime_scope();
    crate::gateway::set_originator("codex_cli_rs_usage").expect("set gateway originator");
    crate::gateway::set_residency_requirement(Some("us"))
        .expect("set gateway residency requirement");

    let server = Server::http("127.0.0.1:0").expect("start mock subscription server");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let request = server
            .recv_timeout(Duration::from_secs(5))
            .expect("subscription server timeout")
            .expect("receive subscription request");
        let path = request.url().to_string();
        let authorization = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("Authorization"))
            .map(|header| header.value.as_str().to_string());
        let chatgpt_account_id = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("ChatGPT-Account-ID"))
            .map(|header| header.value.as_str().to_string());
        let originator = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("originator"))
            .map(|header| header.value.as_str().to_string());
        let residency = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("x-openai-internal-codex-residency"))
            .map(|header| header.value.as_str().to_string());
        let origin = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("Origin"))
            .map(|header| header.value.as_str().to_string());
        let referer = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("Referer"))
            .map(|header| header.value.as_str().to_string());
        let accept = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("Accept"))
            .map(|header| header.value.as_str().to_string());
        tx.send(RecordedSubscriptionRequest {
            path: path.clone(),
            authorization,
            chatgpt_account_id,
            originator,
            residency,
            origin,
            referer,
            accept,
        })
        .expect("send subscription request");
        let response_body = match path.as_str() {
            "/accounts/check/v4-2023-04-27" => {
                r#"{"accounts":{"32673762-4fd7-4cef-8d9e-fa96aec5b5c4":{"account":{"plan_type":"pro","is_default":true},"entitlement":{"subscription_plan":"plus","expires_at":"2026-05-06T03:31:29Z","next_renewal_at":"2026-04-20T03:31:29Z","has_active_subscription":true}}}}"#
            }
            other => panic!("unexpected subscription path: {other}"),
        };
        let response = Response::from_string(response_body)
            .with_status_code(TinyStatusCode(200))
            .with_header(
                Header::from_bytes("Content-Type", "application/json")
                    .expect("content-type header"),
            );
        request
            .respond(response)
            .expect("respond subscription request");
    });

    let snapshot = super::fetch_account_subscription(
        &addr,
        "token_123",
        "32673762-4fd7-4cef-8d9e-fa96aec5b5c4",
        Some("workspace_123"),
    )
    .expect("fetch subscription");

    let recorded = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("receive recorded request");
    handle.join().expect("join subscription server");

    assert!(snapshot.has_subscription);
    assert_eq!(snapshot.account_plan_type.as_deref(), Some("pro"));
    assert_eq!(snapshot.plan_type.as_deref(), Some("plus"));
    assert_eq!(snapshot.expires_at, Some(1_778_038_289));
    assert_eq!(snapshot.renews_at, Some(1_776_655_889));
    assert_eq!(recorded.path, "/accounts/check/v4-2023-04-27");
    assert_eq!(recorded.authorization.as_deref(), Some("Bearer token_123"));
    assert_eq!(recorded.chatgpt_account_id, None);
    assert_eq!(recorded.originator, None);
    assert_eq!(recorded.residency, None);
    assert_eq!(recorded.origin.as_deref(), Some("https://chatgpt.com"));
    assert_eq!(recorded.referer.as_deref(), Some("https://chatgpt.com/"));
    assert_eq!(recorded.accept.as_deref(), Some("application/json"));
}

/// 函数 `refresh_token_url_uses_official_default_for_openai_issuer`
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
fn refresh_token_url_uses_official_default_for_openai_issuer() {
    let _lock = crate::test_env_guard();
    std::env::remove_var("CODEX_REFRESH_TOKEN_URL_OVERRIDE");

    assert_eq!(
        super::resolve_refresh_token_url("https://auth.openai.com"),
        "https://auth.openai.com/oauth/token"
    );
    assert_eq!(
        super::resolve_refresh_token_url("https://auth.openai.com/"),
        "https://auth.openai.com/oauth/token"
    );
}

/// 函数 `refresh_token_url_preserves_custom_issuer_and_override`
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
fn refresh_token_url_preserves_custom_issuer_and_override() {
    let _lock = crate::test_env_guard();
    let previous = std::env::var("CODEX_REFRESH_TOKEN_URL_OVERRIDE").ok();

    std::env::remove_var("CODEX_REFRESH_TOKEN_URL_OVERRIDE");
    assert_eq!(
        super::resolve_refresh_token_url("https://auth.example.com"),
        "https://auth.example.com/oauth/token"
    );

    std::env::set_var(
        "CODEX_REFRESH_TOKEN_URL_OVERRIDE",
        "https://override.example.com/custom/token",
    );
    assert_eq!(
        super::resolve_refresh_token_url("https://auth.example.com"),
        "https://override.example.com/custom/token"
    );

    match previous {
        Some(value) => std::env::set_var("CODEX_REFRESH_TOKEN_URL_OVERRIDE", value),
        None => std::env::remove_var("CODEX_REFRESH_TOKEN_URL_OVERRIDE"),
    }
}

#[test]
fn refresh_access_token_mock_region_blocked_response_surfaces_marker() {
    let _lock = crate::test_env_guard();
    let previous = std::env::var("CODEX_REFRESH_TOKEN_URL_OVERRIDE").ok();
    let _ = super::usage_http_client();
    let server = Server::http("127.0.0.1:0").expect("start mock refresh server");
    let url = format!("http://{}/oauth/token", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let mut request = server
            .recv_timeout(Duration::from_secs(5))
            .expect("refresh server timeout")
            .expect("receive refresh request");
        let mut body = String::new();
        request
            .as_reader()
            .read_to_string(&mut body)
            .expect("read refresh request body");
        tx.send(body).expect("send refresh request body");
        let response = Response::from_string("")
            .with_status_code(TinyStatusCode(403))
            .with_header(
                Header::from_bytes(
                    "x-openai-authorization-error",
                    "unsupported_country_region_territory",
                )
                .expect("auth error header"),
            )
            .with_header(Header::from_bytes("cf-ray", "ray-hkg").expect("cf-ray header"));
        request.respond(response).expect("respond refresh request");
    });
    std::env::set_var("CODEX_REFRESH_TOKEN_URL_OVERRIDE", &url);

    let err =
        match super::refresh_access_token("https://auth.openai.com", "client-id", "refresh-old") {
            Ok(_) => panic!("region blocked refresh should fail"),
            Err(err) => err,
        };
    let body = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("receive refresh request body");
    handle.join().expect("join refresh server");
    match previous {
        Some(value) => std::env::set_var("CODEX_REFRESH_TOKEN_URL_OVERRIDE", value),
        None => std::env::remove_var("CODEX_REFRESH_TOKEN_URL_OVERRIDE"),
    }

    assert!(body.contains("grant_type=refresh_token"));
    assert!(body.contains("refresh_token=refresh-old"));
    assert!(err.contains("refresh token failed with status 403 Forbidden"));
    assert!(err.contains("auth_error=unsupported_country_region_territory"));
    assert!(super::is_refresh_token_region_blocked_error_message(&err));
}

/// 函数 `summarize_usage_error_response_stabilizes_html_and_debug_headers`
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
fn summarize_usage_error_response_stabilizes_html_and_debug_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-request-id", HeaderValue::from_static("req_usage_123"));
    headers.insert("cf-ray", HeaderValue::from_static("cf_usage_123"));
    headers.insert(
        "x-openai-authorization-error",
        HeaderValue::from_static("missing_authorization_header"),
    );
    headers.insert(
        "x-error-json",
        HeaderValue::from_static("eyJlcnJvciI6eyJjb2RlIjoidG9rZW5fZXhwaXJlZCJ9fQ=="),
    );

    let summary = summarize_usage_error_response(
        StatusCode::FORBIDDEN,
        &headers,
        "<html><head><title>Just a moment...</title></head><body>challenge</body></html>",
        true,
    );

    assert!(summary.contains("usage endpoint failed: status=403 Forbidden"));
    assert!(summary.contains("Cloudflare 安全验证页"));
    assert!(summary.contains("request id: req_usage_123"));
    assert!(summary.contains("cf-ray: cf_usage_123"));
    assert!(summary.contains("auth error: missing_authorization_header"));
    assert!(summary.contains("identity error code: token_expired"));
}

/// 函数 `summarize_usage_error_response_accepts_raw_error_json_header`
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
fn summarize_usage_error_response_accepts_raw_error_json_header() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-request-id",
        HeaderValue::from_static("req_usage_raw_123"),
    );
    headers.insert(
        "x-error-json",
        HeaderValue::from_static("{\"details\":{\"identity_error_code\":\"proxy_auth_required\"}}"),
    );

    let summary = summarize_usage_error_response(
        StatusCode::BAD_GATEWAY,
        &headers,
        "<html><head><title>502 Bad Gateway</title></head></html>",
        false,
    );

    assert!(summary.contains("request id: req_usage_raw_123"));
    assert!(summary.contains("identity error code: proxy_auth_required"));
    assert!(summary.contains("<html><head><title>502 Bad Gateway</title></head></html>"));
}
