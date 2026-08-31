use super::should_trigger_stateless_retry;

/// 函数 `stateless_retry_disables_403_when_challenge_retry_is_disabled`
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
fn stateless_retry_disables_403_when_challenge_retry_is_disabled() {
    assert!(!should_trigger_stateless_retry(403, false, true));
    assert!(!should_trigger_stateless_retry(429, false, true));
    assert!(!should_trigger_stateless_retry(401, false, true));
    assert!(should_trigger_stateless_retry(404, false, true));
}

/// 函数 `stateless_retry_keeps_403_when_challenge_retry_is_enabled`
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
fn stateless_retry_keeps_403_when_challenge_retry_is_enabled() {
    assert!(should_trigger_stateless_retry(403, false, false));
    assert!(should_trigger_stateless_retry(429, false, false));
    assert!(!should_trigger_stateless_retry(401, false, false));
}

/// 函数 `stateless_retry_respects_session_affinity_guard`
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
fn stateless_retry_respects_session_affinity_guard() {
    assert!(!should_trigger_stateless_retry(401, true, false));
    assert!(should_trigger_stateless_retry(403, true, false));
    assert!(should_trigger_stateless_retry(429, true, false));
    assert!(!should_trigger_stateless_retry(403, true, true));
    assert!(!should_trigger_stateless_retry(429, true, true));
}
