use codexmanager_core::rpc::types::{RequestLogFilterSummaryResult, RequestLogListParams};

use crate::storage_helpers::open_storage;

use super::list::{normalize_optional_text, normalize_status_filter, normalize_time_range};

/// 函数 `read_request_log_filter_summary`
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
pub(crate) fn read_request_log_filter_summary(
    params: RequestLogListParams,
) -> Result<RequestLogFilterSummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let query = normalize_optional_text(params.query);
    let status_filter = normalize_status_filter(params.status_filter);
    let (start_ts, end_ts) = normalize_time_range(params.start_ts, params.end_ts);
    let total_count = storage
        .count_request_logs(query.as_deref(), None, start_ts, end_ts)
        .map_err(|err| format!("count request logs failed: {err}"))?;
    let filtered = storage
        .summarize_request_logs_filtered(
            query.as_deref(),
            status_filter.as_deref(),
            start_ts,
            end_ts,
        )
        .map_err(|err| format!("summarize request logs failed: {err}"))?;

    Ok(RequestLogFilterSummaryResult {
        total_count,
        filtered_count: filtered.count,
        success_count: filtered.success_count,
        error_count: filtered.error_count,
        total_tokens: filtered.total_tokens,
        total_cost_usd: filtered.estimated_cost_usd,
    })
}

pub(crate) fn read_request_log_filter_summary_for_key_ids(
    params: RequestLogListParams,
    key_ids: &[String],
) -> Result<RequestLogFilterSummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let query = normalize_optional_text(params.query);
    let status_filter = normalize_status_filter(params.status_filter);
    let (start_ts, end_ts) = normalize_time_range(params.start_ts, params.end_ts);
    let total_count = storage
        .count_request_logs_for_keys(query.as_deref(), None, start_ts, end_ts, key_ids)
        .map_err(|err| format!("count request logs failed: {err}"))?;
    let filtered = storage
        .summarize_request_logs_filtered_for_keys(
            query.as_deref(),
            status_filter.as_deref(),
            start_ts,
            end_ts,
            key_ids,
        )
        .map_err(|err| format!("summarize request logs failed: {err}"))?;

    Ok(RequestLogFilterSummaryResult {
        total_count,
        filtered_count: filtered.count,
        success_count: filtered.success_count,
        error_count: filtered.error_count,
        total_tokens: filtered.total_tokens,
        total_cost_usd: filtered.estimated_cost_usd,
    })
}
