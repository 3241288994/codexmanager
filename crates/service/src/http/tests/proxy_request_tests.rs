use super::{build_target_url, filter_request_headers};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Uri};

/// 函数 `build_target_url_keeps_path_and_query`
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
fn build_target_url_keeps_path_and_query() {
    let uri: Uri = "/v1/models?limit=20".parse().expect("valid uri");
    assert_eq!(
        build_target_url("http://127.0.0.1:1234", &uri),
        "http://127.0.0.1:1234/v1/models?limit=20"
    );
}

/// 函数 `filter_request_headers_drops_forbidden_headers`
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
fn filter_request_headers_drops_forbidden_headers() {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        HeaderName::from_static("host"),
        HeaderValue::from_static("localhost:8080"),
    );
    headers.insert(
        HeaderName::from_static("connection"),
        HeaderValue::from_static("keep-alive"),
    );

    let filtered = filter_request_headers(&headers);
    assert!(filtered.contains_key("content-type"));
    assert!(!filtered.contains_key("host"));
    assert!(!filtered.contains_key("connection"));
}
