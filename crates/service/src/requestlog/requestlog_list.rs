use codexmanager_core::rpc::types::{
    RequestLogListParams, RequestLogListResult, RequestLogSummary,
};
use codexmanager_core::storage::RequestLog;

use crate::storage_helpers::open_storage;

const DEFAULT_REQUEST_LOG_PAGE_SIZE: i64 = 20;
const MAX_REQUEST_LOG_PAGE_SIZE: i64 = 500;

/// 函数 `normalize_upstream_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - raw: 参数 raw
///
/// # 返回
/// 返回函数执行结果
fn normalize_upstream_url(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn derive_canonical_source(
    response_adapter: Option<&str>,
    aggregate_api_supplier_name: Option<&str>,
    aggregate_api_url: Option<&str>,
    attempted_aggregate_api_ids: &[String],
) -> String {
    if aggregate_api_supplier_name
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        || aggregate_api_url
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        || !attempted_aggregate_api_ids.is_empty()
    {
        return "aggregate_passthrough".to_string();
    }

    let adapter = response_adapter
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Passthrough");
    if adapter.starts_with("Anthropic") {
        "anthropic_adapter".to_string()
    } else if adapter.starts_with("Gemini") {
        "gemini_adapter".to_string()
    } else if adapter.starts_with("OpenAI") {
        "openai_compat".to_string()
    } else {
        "native_codex".to_string()
    }
}

fn derive_size_reject_stage(status_code: Option<i64>, error: Option<&str>) -> String {
    let Some(error) = error.map(str::trim).filter(|value| !value.is_empty()) else {
        return if status_code == Some(413) {
            "upstream".to_string()
        } else {
            "-".to_string()
        };
    };
    let code = crate::error_codes::code_for_message(error);
    if !matches!(code, "input_too_large" | "request_body_too_large") {
        return if status_code == Some(413) {
            "upstream".to_string()
        } else {
            "-".to_string()
        };
    }

    if error.to_ascii_lowercase().contains("upstream") {
        "upstream".to_string()
    } else {
        "local".to_string()
    }
}

/// 函数 `read_request_logs`
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
pub(crate) fn read_request_logs(
    query: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<RequestLogSummary>, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let logs = storage
        .list_request_logs(query.as_deref(), limit.unwrap_or(200))
        .map_err(|err| format!("list request logs failed: {err}"))?;
    Ok(logs
        .into_iter()
        .map(|item| to_request_log_summary(item, true))
        .collect())
}

pub(crate) fn read_request_logs_for_key_ids(
    query: Option<String>,
    limit: Option<i64>,
    key_ids: &[String],
) -> Result<Vec<RequestLogSummary>, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let logs = storage
        .list_request_logs_for_keys(query.as_deref(), limit.unwrap_or(200), key_ids)
        .map_err(|err| format!("list request logs failed: {err}"))?;
    Ok(logs
        .into_iter()
        .map(|item| to_request_log_summary(item, false))
        .collect())
}

/// 函数 `read_request_log_page`
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
pub(crate) fn read_request_log_page(
    params: RequestLogListParams,
) -> Result<RequestLogListResult, String> {
    let params = params.normalized();
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let query = normalize_optional_text(params.query);
    let status_filter = normalize_status_filter(params.status_filter);
    let (start_ts, end_ts) = normalize_time_range(params.start_ts, params.end_ts);
    let page_size = normalize_page_size(params.page_size);
    let total = storage
        .count_request_logs(query.as_deref(), status_filter.as_deref(), start_ts, end_ts)
        .map_err(|err| format!("count request logs failed: {err}"))?;
    let page = clamp_page(params.page, total, page_size);
    let offset = (page - 1) * page_size;
    let logs = storage
        .list_request_logs_paginated(
            query.as_deref(),
            status_filter.as_deref(),
            start_ts,
            end_ts,
            offset,
            page_size,
        )
        .map_err(|err| format!("list request logs failed: {err}"))?;

    Ok(RequestLogListResult {
        items: logs
            .into_iter()
            .map(|item| to_request_log_summary(item, true))
            .collect(),
        total,
        page,
        page_size,
    })
}

pub(crate) fn read_request_log_page_for_key_ids(
    params: RequestLogListParams,
    key_ids: &[String],
) -> Result<RequestLogListResult, String> {
    let params = params.normalized();
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let query = normalize_optional_text(params.query);
    let status_filter = normalize_status_filter(params.status_filter);
    let (start_ts, end_ts) = normalize_time_range(params.start_ts, params.end_ts);
    let page_size = normalize_page_size(params.page_size);
    let total = storage
        .count_request_logs_for_keys(
            query.as_deref(),
            status_filter.as_deref(),
            start_ts,
            end_ts,
            key_ids,
        )
        .map_err(|err| format!("count request logs failed: {err}"))?;
    let page = clamp_page(params.page, total, page_size);
    let offset = (page - 1) * page_size;
    let logs = storage
        .list_request_logs_paginated_for_keys(
            query.as_deref(),
            status_filter.as_deref(),
            start_ts,
            end_ts,
            offset,
            page_size,
            key_ids,
        )
        .map_err(|err| format!("list request logs failed: {err}"))?;

    Ok(RequestLogListResult {
        items: logs
            .into_iter()
            .map(|item| to_request_log_summary(item, false))
            .collect(),
        total,
        page,
        page_size,
    })
}

/// 函数 `normalize_optional_text`
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
pub(crate) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    let trimmed = value.unwrap_or_default().trim().to_string();
    if trimmed.is_empty() || trimmed == "all" {
        return None;
    }
    Some(trimmed)
}

/// 函数 `normalize_status_filter`
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
pub(crate) fn normalize_status_filter(value: Option<String>) -> Option<String> {
    let normalized = value.unwrap_or_default().trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "all" => None,
        "2xx" | "4xx" | "5xx" => Some(normalized),
        _ => None,
    }
}

pub(crate) fn normalize_optional_timestamp(value: Option<i64>) -> Option<i64> {
    value.filter(|timestamp| *timestamp > 0)
}

pub(crate) fn normalize_time_range(
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> (Option<i64>, Option<i64>) {
    let start_ts = normalize_optional_timestamp(start_ts);
    let end_ts = normalize_optional_timestamp(end_ts);
    match (start_ts, end_ts) {
        (Some(start_ts), Some(end_ts)) if start_ts > end_ts => (Some(end_ts), Some(start_ts)),
        _ => (start_ts, end_ts),
    }
}

/// 函数 `normalize_page_size`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
fn normalize_page_size(value: i64) -> i64 {
    if value < 1 {
        DEFAULT_REQUEST_LOG_PAGE_SIZE
    } else {
        value.min(MAX_REQUEST_LOG_PAGE_SIZE)
    }
}

/// 函数 `clamp_page`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - page: 参数 page
/// - total: 参数 total
/// - page_size: 参数 page_size
///
/// # 返回
/// 返回函数执行结果
fn clamp_page(page: i64, total: i64, page_size: i64) -> i64 {
    let normalized_page = page.max(1);
    let total_pages = if total <= 0 {
        1
    } else {
        ((total + page_size - 1) / page_size).max(1)
    };
    normalized_page.min(total_pages)
}

/// 函数 `to_request_log_summary`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - item: 参数 item
///
/// # 返回
/// 返回函数执行结果
fn to_request_log_summary(item: RequestLog, include_route_details: bool) -> RequestLogSummary {
    let attempted_account_ids = item
        .attempted_account_ids_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .unwrap_or_default();
    let attempted_aggregate_api_ids = item
        .attempted_aggregate_api_ids_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .unwrap_or_default();
    let canonical_source = derive_canonical_source(
        item.response_adapter.as_deref(),
        item.aggregate_api_supplier_name.as_deref(),
        item.aggregate_api_url.as_deref(),
        &attempted_aggregate_api_ids,
    );
    let size_reject_stage = derive_size_reject_stage(item.status_code, item.error.as_deref());
    RequestLogSummary {
        trace_id: item.trace_id,
        key_id: item.key_id,
        account_id: item.account_id,
        initial_account_id: item.initial_account_id,
        attempted_account_ids,
        initial_aggregate_api_id: item.initial_aggregate_api_id,
        attempted_aggregate_api_ids,
        request_path: item.request_path,
        original_path: item.original_path,
        adapted_path: item.adapted_path,
        method: item.method,
        request_type: item.request_type,
        gateway_mode: item.gateway_mode,
        route_strategy: item.route_strategy,
        route_source: item.route_source,
        transparent_mode: item.transparent_mode,
        enhanced_mode: item.enhanced_mode,
        client_model: item.client_model,
        model: item.model,
        model_source: item.model_source,
        upstream_model: include_route_details
            .then_some(item.upstream_model)
            .flatten(),
        actual_source_kind: include_route_details
            .then_some(item.actual_source_kind)
            .flatten(),
        actual_source_id: include_route_details
            .then_some(item.actual_source_id)
            .flatten(),
        client_reasoning_effort: item.client_reasoning_effort,
        reasoning_effort: item.reasoning_effort,
        reasoning_source: item.reasoning_source,
        service_tier: item.service_tier,
        effective_service_tier: item.effective_service_tier,
        service_tier_source: item.service_tier_source,
        response_adapter: item.response_adapter,
        canonical_source: Some(canonical_source),
        size_reject_stage: Some(size_reject_stage),
        upstream_url: normalize_upstream_url(item.upstream_url.as_deref()),
        aggregate_api_supplier_name: item.aggregate_api_supplier_name,
        aggregate_api_url: normalize_upstream_url(item.aggregate_api_url.as_deref()),
        status_code: item.status_code,
        duration_ms: item.duration_ms,
        first_response_ms: item.first_response_ms,
        input_tokens: item.input_tokens,
        cached_input_tokens: item.cached_input_tokens,
        output_tokens: item.output_tokens,
        total_tokens: item.total_tokens,
        reasoning_output_tokens: item.reasoning_output_tokens,
        estimated_cost_usd: item.estimated_cost_usd,
        error: item.error,
        created_at: item.created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        derive_canonical_source, derive_size_reject_stage, normalize_optional_text,
        normalize_status_filter, normalize_upstream_url, RequestLogListParams,
        DEFAULT_REQUEST_LOG_PAGE_SIZE,
    };

    /// 函数 `normalize_upstream_url_keeps_official_domains`
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
    fn normalize_upstream_url_keeps_official_domains() {
        assert_eq!(
            normalize_upstream_url(Some("https://chatgpt.com/backend-api/codex/responses"))
                .as_deref(),
            Some("https://chatgpt.com/backend-api/codex/responses")
        );
        assert_eq!(
            normalize_upstream_url(Some("https://api.openai.com/v1/responses")).as_deref(),
            Some("https://api.openai.com/v1/responses")
        );
    }

    /// 函数 `normalize_upstream_url_keeps_local_addresses`
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
    fn normalize_upstream_url_keeps_local_addresses() {
        assert_eq!(
            normalize_upstream_url(Some("http://127.0.0.1:3000/relay")).as_deref(),
            Some("http://127.0.0.1:3000/relay")
        );
        assert_eq!(
            normalize_upstream_url(Some("http://localhost:3000/relay")).as_deref(),
            Some("http://localhost:3000/relay")
        );
    }

    /// 函数 `normalize_upstream_url_keeps_custom_addresses`
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
    fn normalize_upstream_url_keeps_custom_addresses() {
        assert_eq!(
            normalize_upstream_url(Some("https://gateway.example.com/v1")).as_deref(),
            Some("https://gateway.example.com/v1")
        );
    }

    /// 函数 `normalize_upstream_url_trims_empty_values`
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
    fn normalize_upstream_url_trims_empty_values() {
        assert_eq!(normalize_upstream_url(None), None);
        assert_eq!(normalize_upstream_url(Some("   ")), None);
        assert_eq!(
            normalize_upstream_url(Some(" https://api.openai.com/v1/responses ")).as_deref(),
            Some("https://api.openai.com/v1/responses")
        );
    }

    #[test]
    fn derive_canonical_source_uses_adapter_and_aggregate_context() {
        assert_eq!(
            derive_canonical_source(Some("Passthrough"), None, None, &[]),
            "native_codex"
        );
        assert_eq!(
            derive_canonical_source(Some("OpenAIChatCompletionsSse"), None, None, &[]),
            "openai_compat"
        );
        assert_eq!(
            derive_canonical_source(Some("AnthropicSse"), None, None, &[]),
            "anthropic_adapter"
        );
        assert_eq!(
            derive_canonical_source(Some("GeminiJson"), None, None, &[]),
            "gemini_adapter"
        );
        assert_eq!(
            derive_canonical_source(
                Some("Passthrough"),
                Some("supplier"),
                None,
                &["agg-1".to_string()],
            ),
            "aggregate_passthrough"
        );
    }

    #[test]
    fn derive_size_reject_stage_distinguishes_local_and_upstream() {
        assert_eq!(
            derive_size_reject_stage(
                Some(400),
                Some("Input exceeds the maximum length of 1048576 characters."),
            ),
            "local"
        );
        assert_eq!(
            derive_size_reject_stage(Some(413), Some("upstream request body too large")),
            "upstream"
        );
        assert_eq!(derive_size_reject_stage(Some(413), None), "upstream");
        assert_eq!(derive_size_reject_stage(Some(200), None), "-");
    }

    /// 函数 `request_log_list_params_default_to_first_page_with_twenty_items`
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
    fn request_log_list_params_default_to_first_page_with_twenty_items() {
        let params: RequestLogListParams =
            serde_json::from_value(serde_json::json!({})).expect("deserialize params");
        let normalized = params.normalized();

        assert_eq!(normalized.page, 1);
        assert_eq!(normalized.page_size, DEFAULT_REQUEST_LOG_PAGE_SIZE);
    }

    /// 函数 `normalize_status_filter_accepts_known_values`
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
    fn normalize_status_filter_accepts_known_values() {
        assert_eq!(
            normalize_status_filter(Some("2xx".to_string())).as_deref(),
            Some("2xx")
        );
        assert_eq!(normalize_status_filter(Some("ALL".to_string())), None);
        assert_eq!(normalize_status_filter(Some("unknown".to_string())), None);
    }

    /// 函数 `normalize_optional_text_trims_blank_values`
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
    fn normalize_optional_text_trims_blank_values() {
        assert_eq!(normalize_optional_text(Some("  ".to_string())), None);
        assert_eq!(
            normalize_optional_text(Some(" trace:=abc ".to_string())).as_deref(),
            Some("trace:=abc")
        );
    }
}
