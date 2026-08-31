use axum::http::{HeaderName, HeaderValue};

/// 函数 `is_hop_by_hop_header`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - name: 参数 name
///
/// # 返回
/// 返回函数执行结果
fn is_hop_by_hop_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("connection")
        || name.eq_ignore_ascii_case("keep-alive")
        || name.eq_ignore_ascii_case("proxy-authenticate")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("te")
        || name.eq_ignore_ascii_case("trailer")
        || name.eq_ignore_ascii_case("transfer-encoding")
        || name.eq_ignore_ascii_case("upgrade")
}

/// 函数 `should_skip_request_header`
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
pub(crate) fn should_skip_request_header(name: &HeaderName, value: &HeaderValue) -> bool {
    let lower = name.as_str();
    if is_hop_by_hop_header(lower)
        || lower.eq_ignore_ascii_case("host")
        || lower.eq_ignore_ascii_case("content-length")
    {
        return true;
    }
    // 中文注释：tiny_http 仅支持 ASCII 头值；像 x-codex-turn-metadata 这类可能携带中文路径的头，
    // 只在值可安全转成 ASCII 时透传，非 ASCII 一律在入口层过滤，避免请求还没进业务层就断流。
    value.to_str().is_err()
}

/// 函数 `should_skip_response_header`
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
pub(crate) fn should_skip_response_header(name: &HeaderName) -> bool {
    let lower = name.as_str();
    is_hop_by_hop_header(lower) || lower.eq_ignore_ascii_case("content-length")
}

#[cfg(test)]
#[path = "tests/header_filter_tests.rs"]
mod tests;
