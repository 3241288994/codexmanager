use super::*;

/// 函数 `effective_request_timeout_non_stream_uses_total_only`
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
fn effective_request_timeout_non_stream_uses_total_only() {
    assert_eq!(
        effective_request_timeout(
            Some(Duration::from_secs(120)),
            Some(Duration::from_secs(300)),
            false
        ),
        Some(Duration::from_secs(120))
    );
    assert_eq!(
        effective_request_timeout(None, Some(Duration::from_secs(300)), false),
        None
    );
}

/// 函数 `effective_request_timeout_stream_uses_max_total_and_stream`
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
fn effective_request_timeout_stream_uses_max_total_and_stream() {
    assert_eq!(
        effective_request_timeout(
            Some(Duration::from_secs(120)),
            Some(Duration::from_secs(300)),
            true
        ),
        Some(Duration::from_secs(300))
    );
    assert_eq!(
        effective_request_timeout(
            Some(Duration::from_secs(300)),
            Some(Duration::from_secs(120)),
            true
        ),
        Some(Duration::from_secs(300))
    );
}

/// 函数 `effective_request_timeout_stream_falls_back_when_one_side_missing`
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
fn effective_request_timeout_stream_falls_back_when_one_side_missing() {
    assert_eq!(
        effective_request_timeout(Some(Duration::from_secs(120)), None, true),
        Some(Duration::from_secs(120))
    );
    assert_eq!(
        effective_request_timeout(None, Some(Duration::from_secs(300)), true),
        Some(Duration::from_secs(300))
    );
    assert_eq!(effective_request_timeout(None, None, true), None);
}

/// 函数 `send_timeout_stream_caps_by_configured_stream_timeout`
///
/// 作者: gaohongshun
///
/// 时间: 2026-05-10
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn send_timeout_stream_caps_by_configured_stream_timeout() {
    let _guard = crate::test_env_guard();
    let previous_total = std::env::var_os("CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS");
    let previous_stream = std::env::var_os("CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS");
    std::env::set_var("CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS", "120000");
    std::env::set_var("CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS", "300000");

    let deadline = Some(Instant::now() + Duration::from_secs(120));
    let timeout = send_timeout(deadline, true).expect("stream timeout");

    if let Some(value) = previous_total {
        std::env::set_var("CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS", value);
    } else {
        std::env::remove_var("CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS");
    }
    if let Some(value) = previous_stream {
        std::env::set_var("CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS", value);
    } else {
        std::env::remove_var("CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS");
    }

    assert!(timeout > Duration::from_secs(100));
    assert!(timeout <= Duration::from_secs(120));
}
