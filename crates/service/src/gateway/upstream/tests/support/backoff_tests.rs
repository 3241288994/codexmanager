use super::exponential_jitter_delay;
use std::time::Duration;

/// 函数 `jitter_delay_stays_within_cap`
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
fn jitter_delay_stays_within_cap() {
    let base = Duration::from_millis(80);
    let cap = Duration::from_millis(600);
    for _ in 0..32 {
        let delay = exponential_jitter_delay(base, cap, 4);
        assert!(delay <= cap);
    }
}
