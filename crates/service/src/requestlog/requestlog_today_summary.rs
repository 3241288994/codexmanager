use chrono::{Duration, Local, LocalResult, TimeZone};
use codexmanager_core::rpc::types::RequestLogTodaySummaryResult;

use crate::storage_helpers::open_storage;

const MAX_REQUESTED_DAY_RANGE_SECS: i64 = 48 * 60 * 60;

/// 函数 `local_day_bounds_ts`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
fn local_day_bounds_ts() -> Result<(i64, i64), String> {
    let now = Local::now();
    let today = now.date_naive();
    let start_naive = today
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| "build local start-of-day failed".to_string())?;
    let tomorrow_naive = (today + Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| "build local end-of-day failed".to_string())?;

    let start = match Local.from_local_datetime(&start_naive) {
        LocalResult::Single(value) => value.timestamp(),
        LocalResult::Ambiguous(a, b) => a.timestamp().min(b.timestamp()),
        LocalResult::None => now.timestamp(),
    };
    let end = match Local.from_local_datetime(&tomorrow_naive) {
        LocalResult::Single(value) => value.timestamp(),
        LocalResult::Ambiguous(a, b) => a.timestamp().max(b.timestamp()),
        LocalResult::None => start + 24 * 60 * 60,
    };
    Ok((start, end.max(start)))
}

/// 函数 `resolve_day_bounds_ts`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-13
///
/// # 参数
/// - day_start_ts: 参数 day_start_ts
/// - day_end_ts: 参数 day_end_ts
///
/// # 返回
/// 返回函数执行结果
fn resolve_day_bounds_ts(
    day_start_ts: Option<i64>,
    day_end_ts: Option<i64>,
) -> Result<(i64, i64), String> {
    match (day_start_ts, day_end_ts) {
        (Some(start), Some(end)) => {
            if end <= start {
                return Err("dayEndTs must be greater than dayStartTs".to_string());
            }
            if end - start > MAX_REQUESTED_DAY_RANGE_SECS {
                return Err("requested day range is too large".to_string());
            }
            Ok((start, end))
        }
        (None, None) => local_day_bounds_ts(),
        _ => Err("dayStartTs and dayEndTs must be provided together".to_string()),
    }
}

/// 函数 `read_requestlog_today_summary`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - day_start_ts: 参数 day_start_ts
/// - day_end_ts: 参数 day_end_ts
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn read_requestlog_today_summary(
    day_start_ts: Option<i64>,
    day_end_ts: Option<i64>,
) -> Result<RequestLogTodaySummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let (start_ts, end_ts) = resolve_day_bounds_ts(day_start_ts, day_end_ts)?;
    let summary = storage
        .summarize_request_logs_between(start_ts, end_ts)
        .map_err(|err| format!("summarize request logs failed: {err}"))?;
    let input_tokens = summary.input_tokens.max(0);
    let cached_input_tokens = summary.cached_input_tokens.max(0);
    let output_tokens = summary.output_tokens.max(0);
    let reasoning_output_tokens = summary.reasoning_output_tokens.max(0);
    let non_cached_input_tokens = input_tokens.saturating_sub(cached_input_tokens);
    Ok(RequestLogTodaySummaryResult {
        input_tokens,
        cached_input_tokens,
        output_tokens,
        reasoning_output_tokens,
        today_tokens: non_cached_input_tokens.saturating_add(output_tokens),
        estimated_cost: summary.estimated_cost_usd.max(0.0),
    })
}

pub(crate) fn read_requestlog_today_summary_for_key_ids(
    day_start_ts: Option<i64>,
    day_end_ts: Option<i64>,
    key_ids: &[String],
) -> Result<RequestLogTodaySummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let (start_ts, end_ts) = resolve_day_bounds_ts(day_start_ts, day_end_ts)?;
    let summary = storage
        .summarize_request_logs_between_for_keys(start_ts, end_ts, key_ids)
        .map_err(|err| format!("summarize request logs failed: {err}"))?;
    let input_tokens = summary.input_tokens.max(0);
    let cached_input_tokens = summary.cached_input_tokens.max(0);
    let output_tokens = summary.output_tokens.max(0);
    let reasoning_output_tokens = summary.reasoning_output_tokens.max(0);
    let non_cached_input_tokens = input_tokens.saturating_sub(cached_input_tokens);
    Ok(RequestLogTodaySummaryResult {
        input_tokens,
        cached_input_tokens,
        output_tokens,
        reasoning_output_tokens,
        today_tokens: non_cached_input_tokens.saturating_add(output_tokens),
        estimated_cost: summary.estimated_cost_usd.max(0.0),
    })
}

#[cfg(test)]
mod tests {
    use super::{resolve_day_bounds_ts, MAX_REQUESTED_DAY_RANGE_SECS};

    #[test]
    fn resolve_day_bounds_uses_requested_range_when_complete() {
        assert_eq!(
            resolve_day_bounds_ts(Some(1_700_000_000), Some(1_700_086_400)).unwrap(),
            (1_700_000_000, 1_700_086_400)
        );
    }

    #[test]
    fn resolve_day_bounds_rejects_partial_range() {
        let error = resolve_day_bounds_ts(Some(1_700_000_000), None).unwrap_err();
        assert!(error.contains("provided together"));
    }

    #[test]
    fn resolve_day_bounds_rejects_oversized_range() {
        let error =
            resolve_day_bounds_ts(Some(0), Some(MAX_REQUESTED_DAY_RANGE_SECS + 1)).unwrap_err();
        assert!(error.contains("too large"));
    }
}
