use reqwest::header::HeaderValue;

use super::{
    reload_from_env, resolve_upstream_fallback_base_url, should_send_chatgpt_account_header,
    should_try_openai_fallback, should_try_openai_fallback_by_status,
};

/// 函数 `fallback_status_trigger_is_limited_to_responses_path`
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
fn fallback_status_trigger_is_limited_to_responses_path() {
    assert!(!should_try_openai_fallback_by_status(
        "https://chatgpt.com/backend-api/codex",
        "/v1/responses",
        429
    ));
    assert!(!should_try_openai_fallback_by_status(
        "https://chatgpt.com/backend-api/codex",
        "/v1/responses/compact",
        429
    ));
    assert!(!should_try_openai_fallback_by_status(
        "https://chatgpt.com/backend-api/codex",
        "/v1/chat/completions",
        429
    ));
}

/// 函数 `fallback_content_type_trigger_is_limited_to_responses_path`
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
fn fallback_content_type_trigger_is_limited_to_responses_path() {
    let html = HeaderValue::from_static("text/html; charset=utf-8");
    assert!(!should_try_openai_fallback(
        "https://chatgpt.com/backend-api/codex",
        "/v1/responses",
        Some(&html)
    ));
    assert!(!should_try_openai_fallback(
        "https://chatgpt.com/backend-api/codex",
        "/v1/responses/compact",
        Some(&html)
    ));
    assert!(!should_try_openai_fallback(
        "https://chatgpt.com/backend-api/codex",
        "/v1/chat/completions",
        Some(&html)
    ));
}

/// 函数 `fallback_base_is_disabled_by_default_for_chatgpt_backend`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-03
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn fallback_base_is_disabled_by_default_for_chatgpt_backend() {
    std::env::remove_var("CODEXMANAGER_UPSTREAM_FALLBACK_BASE_URL");
    reload_from_env();
    assert_eq!(
        resolve_upstream_fallback_base_url("https://chatgpt.com/backend-api/codex").as_deref(),
        None
    );
}

/// 函数 `fallback_base_ignores_env_override_for_chatgpt_backend`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-03
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn fallback_base_ignores_env_override_for_chatgpt_backend() {
    std::env::set_var(
        "CODEXMANAGER_UPSTREAM_FALLBACK_BASE_URL",
        "https://api.openai.com/v1",
    );
    reload_from_env();
    assert_eq!(
        resolve_upstream_fallback_base_url("https://chatgpt.com/backend-api/codex").as_deref(),
        None
    );

    std::env::remove_var("CODEXMANAGER_UPSTREAM_FALLBACK_BASE_URL");
    reload_from_env();
}

/// 函数 `fallback_base_is_disabled_for_non_chatgpt_primary`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-03
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn fallback_base_is_disabled_for_non_chatgpt_primary() {
    std::env::remove_var("CODEXMANAGER_UPSTREAM_FALLBACK_BASE_URL");
    reload_from_env();
    assert_eq!(
        resolve_upstream_fallback_base_url("https://api.openai.com/v1").as_deref(),
        None
    );
}

/// 函数 `chatgpt_account_header_is_limited_to_codex_backend_shape`
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
fn chatgpt_account_header_is_limited_to_codex_backend_shape() {
    assert!(should_send_chatgpt_account_header(
        "https://chatgpt.com/backend-api/codex"
    ));
    assert!(should_send_chatgpt_account_header(
        "http://127.0.0.1:8787/backend-api/codex/responses"
    ));
    assert!(!should_send_chatgpt_account_header(
        "https://api.openai.com/v1"
    ));
    assert!(!should_send_chatgpt_account_header(
        "https://api.anthropic.com/v1/messages"
    ));
    assert!(!should_send_chatgpt_account_header(
        "https://example.com/v1/responses"
    ));
}
