use std::time::{Duration, Instant};

/// 函数 `effective_request_timeout`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - total_timeout: 参数 total_timeout
/// - stream_timeout: 参数 stream_timeout
/// - is_stream: 参数 is_stream
///
/// # 返回
/// 返回函数执行结果
fn effective_request_timeout(
    total_timeout: Option<Duration>,
    stream_timeout: Option<Duration>,
    is_stream: bool,
) -> Option<Duration> {
    if !is_stream {
        return total_timeout;
    }
    match (total_timeout, stream_timeout) {
        // 中文注释：流式链路要兼容 0.1.18 的行为，避免较短 total timeout 抢先截断更长的 stream timeout。
        (Some(total_timeout), Some(stream_timeout)) => Some(total_timeout.max(stream_timeout)),
        (Some(total_timeout), None) => Some(total_timeout),
        (None, Some(stream_timeout)) => Some(stream_timeout),
        (None, None) => None,
    }
}

/// 函数 `request_deadline`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn request_deadline(started_at: Instant, is_stream: bool) -> Option<Instant> {
    let total_timeout = super::super::super::upstream_total_timeout();
    let stream_timeout = super::super::super::upstream_stream_timeout();
    effective_request_timeout(total_timeout, stream_timeout, is_stream)
        .map(|timeout| started_at + timeout)
}

/// 函数 `remaining`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn remaining(deadline: Option<Instant>) -> Option<Duration> {
    deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()))
}

/// 函数 `is_expired`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn is_expired(deadline: Option<Instant>) -> bool {
    remaining(deadline).is_some_and(|remaining| remaining.is_zero())
}

/// 函数 `cap_wait`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn cap_wait(wait: Duration, deadline: Option<Instant>) -> Option<Duration> {
    match remaining(deadline) {
        Some(remaining) if remaining.is_zero() => None,
        Some(remaining) => Some(wait.min(remaining)),
        None => Some(wait),
    }
}

/// 函数 `send_timeout`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn send_timeout(
    deadline: Option<Instant>,
    is_stream: bool,
) -> Option<Duration> {
    if is_stream {
        let configured = super::super::super::upstream_stream_timeout();
        return match (configured, remaining(deadline)) {
            (Some(configured), Some(remaining)) => Some(configured.min(remaining)),
            (Some(configured), None) => Some(configured),
            (None, Some(remaining)) => Some(remaining),
            (None, None) => None,
        }
        .map(|timeout| timeout.max(Duration::from_millis(1)));
    }
    remaining(deadline).map(|remaining| remaining.max(Duration::from_millis(1)))
}

#[cfg(test)]
#[path = "../tests/support/deadline_tests.rs"]
mod tests;
