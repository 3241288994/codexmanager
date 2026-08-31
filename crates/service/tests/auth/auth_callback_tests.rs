use super::{
    build_callback_error_page, build_callback_success_page, ensure_login_server_with_addr,
    html_response, oauth_callback_error_message, resolve_redirect_uri, LOGIN_SERVER_STATE,
};
use std::net::TcpListener;
use url::Url;

#[path = "../support.rs"]
mod support;
use support::test_env_guard;

/// 函数 `reset_login_server_state`
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
fn reset_login_server_state() {
    if let Some(cell) = LOGIN_SERVER_STATE.get() {
        if let Ok(mut guard) = cell.lock() {
            *guard = None;
        }
    }
}

/// 函数 `resolve_redirect_uri_prefers_login_server`
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
fn resolve_redirect_uri_prefers_login_server() {
    let _guard = test_env_guard();
    reset_login_server_state();
    let prev_redirect = std::env::var("CODEXMANAGER_REDIRECT_URI").ok();
    let prev_login = std::env::var("CODEXMANAGER_LOGIN_ADDR").ok();
    let prev_service = std::env::var("CODEXMANAGER_SERVICE_ADDR").ok();

    std::env::remove_var("CODEXMANAGER_REDIRECT_URI");
    std::env::set_var("CODEXMANAGER_LOGIN_ADDR", "127.0.0.1:0");
    std::env::set_var("CODEXMANAGER_SERVICE_ADDR", "localhost:48760");

    let uri = resolve_redirect_uri().expect("redirect uri");
    let url = Url::parse(&uri).expect("parse redirect uri");
    assert_eq!(url.host_str(), Some("localhost"));
    let port = url.port_or_known_default().expect("port");
    assert_ne!(port, 48760);
    assert_eq!(url.path(), "/auth/callback");

    match prev_redirect {
        Some(value) => std::env::set_var("CODEXMANAGER_REDIRECT_URI", value),
        None => std::env::remove_var("CODEXMANAGER_REDIRECT_URI"),
    }
    match prev_login {
        Some(value) => std::env::set_var("CODEXMANAGER_LOGIN_ADDR", value),
        None => std::env::remove_var("CODEXMANAGER_LOGIN_ADDR"),
    }
    match prev_service {
        Some(value) => std::env::set_var("CODEXMANAGER_SERVICE_ADDR", value),
        None => std::env::remove_var("CODEXMANAGER_SERVICE_ADDR"),
    }
    reset_login_server_state();
}

/// 函数 `login_server_reports_port_in_use`
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
fn login_server_reports_port_in_use() {
    let _guard = test_env_guard();
    reset_login_server_state();
    let prev_login = std::env::var("CODEXMANAGER_LOGIN_ADDR").ok();

    let listener_v6 = TcpListener::bind("[::1]:0").expect("bind v6 port");
    let port = listener_v6.local_addr().expect("addr").port();
    let listener_v4 = TcpListener::bind(format!("127.0.0.1:{port}")).ok();
    let err = match ensure_login_server_with_addr(&format!("localhost:{port}")) {
        Ok(_) => panic!("expected port in use error"),
        Err(err) => err,
    };
    assert!(err.contains("占用") || err.contains("in use"));

    drop(listener_v4);
    drop(listener_v6);
    match prev_login {
        Some(value) => std::env::set_var("CODEXMANAGER_LOGIN_ADDR", value),
        None => std::env::remove_var("CODEXMANAGER_LOGIN_ADDR"),
    }
    reset_login_server_state();
}

/// 函数 `login_server_rejects_non_loopback_by_default`
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
fn login_server_rejects_non_loopback_by_default() {
    let _guard = test_env_guard();
    reset_login_server_state();
    let prev_allow = std::env::var("CODEXMANAGER_ALLOW_NON_LOOPBACK_LOGIN_ADDR").ok();

    std::env::remove_var("CODEXMANAGER_ALLOW_NON_LOOPBACK_LOGIN_ADDR");
    let err = ensure_login_server_with_addr("0.0.0.0:1455").expect_err("should reject");
    assert!(err.contains("loopback"));

    match prev_allow {
        Some(value) => std::env::set_var("CODEXMANAGER_ALLOW_NON_LOOPBACK_LOGIN_ADDR", value),
        None => std::env::remove_var("CODEXMANAGER_ALLOW_NON_LOOPBACK_LOGIN_ADDR"),
    }
    reset_login_server_state();
}

/// 函数 `callback_success_page_contains_auto_close_script`
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
fn callback_success_page_contains_auto_close_script() {
    let html = build_callback_success_page();
    assert!(html.contains("window.close()"));
    assert!(html.contains("Login Success"));
    assert!(html.contains("Close Window"));
}

/// 函数 `callback_error_page_escapes_message`
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
fn callback_error_page_escapes_message() {
    let html = build_callback_error_page("bad <script>alert(1)</script>");
    assert!(html.contains("Login Failed"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
}

/// 函数 `callback_html_response_forces_connection_close`
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
fn callback_html_response_forces_connection_close() {
    let response = html_response(build_callback_success_page());
    let headers = response.headers();

    let content_type = headers
        .iter()
        .find(|header| header.field.equiv("Content-Type"))
        .map(|header| header.value.as_str());
    assert_eq!(content_type, Some("text/html; charset=utf-8"));
    assert!(
        headers
            .iter()
            .all(|header| !header.field.equiv("Transfer-Encoding")),
        "html response should stay simple for login callback pages"
    );
}

/// 函数 `oauth_callback_error_message_maps_missing_entitlement`
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
fn oauth_callback_error_message_maps_missing_entitlement() {
    let message = oauth_callback_error_message(
        "access_denied",
        Some("missing_codex_entitlement for workspace"),
    );
    assert!(message.contains("Codex is not enabled"));
}

/// 函数 `login_start_fails_when_login_server_cannot_bind`
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
fn login_start_fails_when_login_server_cannot_bind() {
    let _guard = test_env_guard();
    reset_login_server_state();
    let prev_login = std::env::var("CODEXMANAGER_LOGIN_ADDR").ok();

    std::env::set_var("CODEXMANAGER_LOGIN_ADDR", "0.0.0.0:1455");

    let err = ensure_login_server_with_addr("0.0.0.0:1455").expect_err("should fail");
    assert!(err.contains("loopback"));

    match prev_login {
        Some(value) => std::env::set_var("CODEXMANAGER_LOGIN_ADDR", value),
        None => std::env::remove_var("CODEXMANAGER_LOGIN_ADDR"),
    }
    reset_login_server_state();
}
