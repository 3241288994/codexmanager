use super::is_keepalive_error_ignorable;

/// 函数 `keepalive_ignores_expected_idle_errors`
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
fn keepalive_ignores_expected_idle_errors() {
    assert!(is_keepalive_error_ignorable("no available account"));
    assert!(is_keepalive_error_ignorable("storage unavailable"));
    assert!(!is_keepalive_error_ignorable("upstream timeout"));
}
