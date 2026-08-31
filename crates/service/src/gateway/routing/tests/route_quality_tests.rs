use super::*;

/// 函数 `route_quality_penalty_prefers_successful_accounts`
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
fn route_quality_penalty_prefers_successful_accounts() {
    let _guard = crate::test_env_guard();
    clear_route_quality_for_tests();
    record_route_quality("acc_a", 403);
    record_route_quality("acc_a", 403);
    record_route_quality("acc_b", 200);
    assert!(route_quality_penalty("acc_a") > route_quality_penalty("acc_b"));
    assert!(route_health_score("acc_b") > route_health_score("acc_a"));
}

/// 函数 `route_quality_penalty_evicts_expired_record`
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
fn route_quality_penalty_evicts_expired_record() {
    let _guard = crate::test_env_guard();
    clear_route_quality_for_tests();
    let lock = ROUTE_QUALITY.get_or_init(|| Mutex::new(RouteQualityState::default()));
    let mut state = lock.lock().expect("route quality state lock");
    let now = now_ts();
    state.entries.insert(
        "acc_old".to_string(),
        RouteQualityRecord {
            success_2xx: 0,
            challenge_403: 1,
            throttle_429: 0,
            upstream_5xx: 0,
            upstream_4xx: 0,
            health_score: DEFAULT_ROUTE_HEALTH_SCORE,
            updated_at: now - ROUTE_QUALITY_TTL_SECS - 1,
        },
    );
    drop(state);

    assert_eq!(route_quality_penalty("acc_old"), 0);
    let state = lock.lock().expect("route quality state lock");
    assert!(!state.entries.contains_key("acc_old"));
}

/// 函数 `record_path_cleanup_prunes_expired_records`
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
fn record_path_cleanup_prunes_expired_records() {
    let _guard = crate::test_env_guard();
    clear_route_quality_for_tests();
    let lock = ROUTE_QUALITY.get_or_init(|| Mutex::new(RouteQualityState::default()));
    let mut state = lock.lock().expect("route quality state lock");
    let now = now_ts();
    state.entries.insert(
        "acc_stale".to_string(),
        RouteQualityRecord {
            success_2xx: 0,
            challenge_403: 1,
            throttle_429: 0,
            upstream_5xx: 0,
            upstream_4xx: 0,
            health_score: DEFAULT_ROUTE_HEALTH_SCORE,
            updated_at: now - ROUTE_QUALITY_TTL_SECS - 1,
        },
    );
    state.last_cleanup_at = now - ROUTE_QUALITY_CLEANUP_INTERVAL_SECS - 1;
    drop(state);

    record_route_quality("acc_fresh", 200);
    let state = lock.lock().expect("route quality state lock");
    assert!(!state.entries.contains_key("acc_stale"));
    assert!(state.entries.contains_key("acc_fresh"));
}
