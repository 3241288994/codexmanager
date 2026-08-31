use codexmanager_core::storage::now_ts;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const DEFAULT_ROUTE_HEALTH_SCORE: i32 = 100;
const MIN_ROUTE_HEALTH_SCORE: i32 = 0;
const MAX_ROUTE_HEALTH_SCORE: i32 = 200;

#[derive(Debug, Clone, Default)]
struct RouteQualityRecord {
    success_2xx: u32,
    challenge_403: u32,
    throttle_429: u32,
    upstream_5xx: u32,
    upstream_4xx: u32,
    health_score: i32,
    updated_at: i64,
}

static ROUTE_QUALITY: OnceLock<Mutex<RouteQualityState>> = OnceLock::new();
const ROUTE_QUALITY_TTL_SECS: i64 = 24 * 60 * 60;
const ROUTE_QUALITY_CLEANUP_INTERVAL_SECS: i64 = 60;

#[derive(Default)]
struct RouteQualityState {
    entries: HashMap<String, RouteQualityRecord>,
    last_cleanup_at: i64,
}

/// 函数 `with_map_mut`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - mutator: 参数 mutator
///
/// # 返回
/// 无
fn with_map_mut<F>(mutator: F)
where
    F: FnOnce(&mut HashMap<String, RouteQualityRecord>, i64),
{
    let lock = ROUTE_QUALITY.get_or_init(|| Mutex::new(RouteQualityState::default()));
    let mut state = crate::lock_utils::lock_recover(lock, "route_quality_state");
    let now = now_ts();
    maybe_cleanup_route_quality(&mut state, now);
    mutator(&mut state.entries, now);
}

/// 函数 `record_route_quality`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 无
pub(crate) fn record_route_quality(account_id: &str, status_code: u16) {
    with_map_mut(|map, now| {
        let record = map.entry(account_id.to_string()).or_default();
        if record.updated_at == 0 {
            record.health_score = DEFAULT_ROUTE_HEALTH_SCORE;
        }
        record.updated_at = now;
        let delta = route_health_delta(status_code);
        record.health_score =
            (record.health_score + delta).clamp(MIN_ROUTE_HEALTH_SCORE, MAX_ROUTE_HEALTH_SCORE);
        match status_code {
            200..=299 => {
                record.success_2xx = record.success_2xx.saturating_add(1);
            }
            403 => {
                record.challenge_403 = record.challenge_403.saturating_add(1);
            }
            429 => {
                record.throttle_429 = record.throttle_429.saturating_add(1);
            }
            500..=599 => {
                record.upstream_5xx = record.upstream_5xx.saturating_add(1);
            }
            400..=499 => {
                record.upstream_4xx = record.upstream_4xx.saturating_add(1);
            }
            _ => {}
        }
    });
}

/// 函数 `route_health_score`
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
pub(crate) fn route_health_score(account_id: &str) -> i32 {
    let lock = ROUTE_QUALITY.get_or_init(|| Mutex::new(RouteQualityState::default()));
    let mut state = crate::lock_utils::lock_recover(lock, "route_quality_state");
    let now = now_ts();
    let Some(record) = state.entries.get(account_id).cloned() else {
        return DEFAULT_ROUTE_HEALTH_SCORE;
    };
    if route_quality_record_expired(&record, now) {
        state.entries.remove(account_id);
        return DEFAULT_ROUTE_HEALTH_SCORE;
    }
    record
        .health_score
        .clamp(MIN_ROUTE_HEALTH_SCORE, MAX_ROUTE_HEALTH_SCORE)
}

/// 函数 `route_quality_penalty`
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
#[allow(dead_code)]
pub(crate) fn route_quality_penalty(account_id: &str) -> i64 {
    let lock = ROUTE_QUALITY.get_or_init(|| Mutex::new(RouteQualityState::default()));
    let mut state = crate::lock_utils::lock_recover(lock, "route_quality_state");
    let now = now_ts();
    let Some(record) = state.entries.get(account_id).cloned() else {
        return 0;
    };
    if route_quality_record_expired(&record, now) {
        state.entries.remove(account_id);
        return 0;
    }
    i64::from(record.challenge_403) * 6 + i64::from(record.throttle_429) * 3
        - i64::from(record.success_2xx) * 2
}

/// 函数 `clear_runtime_state`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 无
pub(super) fn clear_runtime_state() {
    let lock = ROUTE_QUALITY.get_or_init(|| Mutex::new(RouteQualityState::default()));
    let mut state = crate::lock_utils::lock_recover(lock, "route_quality_state");
    state.entries.clear();
    state.last_cleanup_at = 0;
}

/// 函数 `clear_route_quality_for_tests`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 无
#[cfg(test)]
pub(crate) fn clear_route_quality_for_tests() {
    clear_runtime_state();
}

/// 函数 `route_quality_tests_guard`
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
#[cfg(test)]
pub(crate) fn route_quality_tests_guard() -> std::sync::MutexGuard<'static, ()> {
    static ROUTE_QUALITY_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    ROUTE_QUALITY_TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 函数 `maybe_cleanup_route_quality`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - state: 参数 state
/// - now: 参数 now
///
/// # 返回
/// 无
fn maybe_cleanup_route_quality(state: &mut RouteQualityState, now: i64) {
    if state.last_cleanup_at != 0
        && now.saturating_sub(state.last_cleanup_at) < ROUTE_QUALITY_CLEANUP_INTERVAL_SECS
    {
        return;
    }
    state.last_cleanup_at = now;
    state
        .entries
        .retain(|_, value| !route_quality_record_expired(value, now));
}

/// 函数 `route_quality_record_expired`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - record: 参数 record
/// - now: 参数 now
///
/// # 返回
/// 返回函数执行结果
fn route_quality_record_expired(record: &RouteQualityRecord, now: i64) -> bool {
    record.updated_at + ROUTE_QUALITY_TTL_SECS <= now
}

/// 函数 `route_health_delta`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - status_code: 参数 status_code
///
/// # 返回
/// 返回函数执行结果
fn route_health_delta(status_code: u16) -> i32 {
    match status_code {
        200..=299 => 4,
        429 => -15,
        500..=599 => -10,
        401 | 403 => -18,
        400..=499 => -8,
        _ => -2,
    }
}

#[cfg(test)]
#[path = "tests/route_quality_tests.rs"]
mod tests;
