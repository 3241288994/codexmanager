use super::{should_skip_request_header, should_skip_response_header};
use axum::http::{HeaderName, HeaderValue};

/// 函数 `request_header_filters_hop_by_hop_and_non_ascii`
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
fn request_header_filters_hop_by_hop_and_non_ascii() {
    let connection = HeaderName::from_static("connection");
    let keep = HeaderValue::from_static("keep-alive");
    assert!(should_skip_request_header(&connection, &keep));

    let metadata = HeaderName::from_static("x-codex-turn-metadata");
    let bad_value = HeaderValue::from_bytes(&[0xE4, 0xB8, 0xAD]).expect("non-ascii bytes");
    assert!(should_skip_request_header(&metadata, &bad_value));
}

/// 函数 `request_header_keeps_ascii_turn_metadata`
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
fn request_header_keeps_ascii_turn_metadata() {
    let metadata = HeaderName::from_static("x-codex-turn-metadata");
    let ascii_value = HeaderValue::from_static(
        "{\"workspaces\":{\"D:/repo\":{\"latest_git_commit_hash\":\"abc123\"}}}",
    );
    assert!(!should_skip_request_header(&metadata, &ascii_value));
}

/// 函数 `request_header_keeps_normal_content_type`
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
fn request_header_keeps_normal_content_type() {
    let content_type = HeaderName::from_static("content-type");
    let json = HeaderValue::from_static("application/json");
    assert!(!should_skip_request_header(&content_type, &json));
}

/// 函数 `response_header_filters_content_length_and_connection`
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
fn response_header_filters_content_length_and_connection() {
    let content_length = HeaderName::from_static("content-length");
    assert!(should_skip_response_header(&content_length));

    let connection = HeaderName::from_static("connection");
    assert!(should_skip_response_header(&connection));

    let content_type = HeaderName::from_static("content-type");
    assert!(!should_skip_response_header(&content_type));
}
