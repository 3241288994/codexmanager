use serde::Serialize;
use std::thread;
use sysinfo::System;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayConcurrencyRecommendation {
    pub(crate) cpu_cores: usize,
    pub(crate) memory_mib: u64,
    pub(crate) usage_refresh_workers: usize,
    pub(crate) http_worker_factor: usize,
    pub(crate) http_worker_min: usize,
    pub(crate) http_stream_worker_factor: usize,
    pub(crate) http_stream_worker_min: usize,
    pub(crate) account_max_inflight: usize,
    pub(crate) queue_wait_timeout_ms: u64,
}

/// 函数 `current_gateway_concurrency_recommendation`
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
pub(crate) fn current_gateway_concurrency_recommendation() -> GatewayConcurrencyRecommendation {
    let cpu_cores = thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(4)
        .max(1);
    let system = System::new_all();
    let memory_mib = system.total_memory().saturating_div(1024).max(1);
    recommend_gateway_concurrency(cpu_cores, memory_mib)
}

/// 函数 `recommend_gateway_concurrency`
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
pub(crate) fn recommend_gateway_concurrency(
    cpu_cores: usize,
    memory_mib: u64,
) -> GatewayConcurrencyRecommendation {
    let cpu_cores = cpu_cores.max(1);
    let memory_blocks = ((memory_mib / 2_048).max(1)) as usize;
    let resource_score = cpu_cores.min(memory_blocks).max(1);

    let (
        usage_refresh_workers,
        http_worker_factor,
        http_worker_min,
        http_stream_worker_factor,
        http_stream_worker_min,
        account_max_inflight,
    ) = match resource_score {
        1 => (2, 2, 4, 1, 1, 1),
        2..=4 => (3, 3, 6, 1, 2, 1),
        5..=8 => (4, 4, 8, 1, 2, 2),
        9..=16 => (6, 5, 12, 2, 4, 2),
        _ => (8, 6, 16, 2, 4, 4),
    };

    GatewayConcurrencyRecommendation {
        cpu_cores,
        memory_mib,
        usage_refresh_workers,
        http_worker_factor,
        http_worker_min,
        http_stream_worker_factor,
        http_stream_worker_min,
        account_max_inflight,
        queue_wait_timeout_ms: 100,
    }
}

#[cfg(test)]
mod tests {
    use super::recommend_gateway_concurrency;

    /// 函数 `small_machine_prefers_conservative_values`
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
    fn small_machine_prefers_conservative_values() {
        let recommendation = recommend_gateway_concurrency(2, 2_048);
        assert_eq!(recommendation.usage_refresh_workers, 2);
        assert_eq!(recommendation.http_worker_factor, 2);
        assert_eq!(recommendation.http_worker_min, 4);
        assert_eq!(recommendation.http_stream_worker_factor, 1);
        assert_eq!(recommendation.http_stream_worker_min, 1);
        assert_eq!(recommendation.account_max_inflight, 1);
    }

    /// 函数 `larger_machine_scales_up_gradually`
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
    fn larger_machine_scales_up_gradually() {
        let recommendation = recommend_gateway_concurrency(16, 32_768);
        assert_eq!(recommendation.usage_refresh_workers, 6);
        assert_eq!(recommendation.http_worker_factor, 5);
        assert_eq!(recommendation.http_worker_min, 12);
        assert_eq!(recommendation.http_stream_worker_factor, 2);
        assert_eq!(recommendation.http_stream_worker_min, 4);
        assert_eq!(recommendation.account_max_inflight, 2);
    }
}
