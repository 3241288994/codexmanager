use super::{parse_request_log_query, RequestLogQuery};

/// 函数 `prefixed_field_query_supports_exact_mode`
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
fn prefixed_field_query_supports_exact_mode() {
    let query = parse_request_log_query(Some("method:=POST"));
    assert!(matches!(
        query,
        RequestLogQuery::FieldExact {
            column: "method",
            value
        } if value == "POST"
    ));
}

/// 函数 `prefixed_field_query_keeps_like_mode_by_default`
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
fn prefixed_field_query_keeps_like_mode_by_default() {
    let query = parse_request_log_query(Some("key:key-alpha"));
    assert!(matches!(
        query,
        RequestLogQuery::FieldLike {
            column: "key_id",
            pattern
        } if pattern == "%key-alpha%"
    ));
}

/// 函数 `prefixed_account_query_supports_alias`
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
fn prefixed_account_query_supports_alias() {
    let query = parse_request_log_query(Some("account:acc-1"));
    assert!(matches!(
        query,
        RequestLogQuery::AccountLike(pattern) if pattern == "%acc-1%"
    ));
}

#[test]
fn prefixed_request_type_and_service_tier_queries_are_supported() {
    let request_type_query = parse_request_log_query(Some("type:=ws"));
    assert!(matches!(
        request_type_query,
        RequestLogQuery::FieldExact {
            column: "request_type",
            value
        } if value == "ws"
    ));

    let service_tier_query = parse_request_log_query(Some("tier:fast"));
    assert!(matches!(
        service_tier_query,
        RequestLogQuery::FieldLike {
            column: "service_tier",
            pattern
        } if pattern == "%fast%"
    ));

    let effective_service_tier_query = parse_request_log_query(Some("effective_tier:=priority"));
    assert!(matches!(
        effective_service_tier_query,
        RequestLogQuery::FieldExact {
            column: "effective_service_tier",
            value
        } if value == "priority"
    ));

    let service_tier_source_query = parse_request_log_query(Some("tier_source:=gateway_override"));
    assert!(matches!(
        service_tier_source_query,
        RequestLogQuery::FieldExact {
            column: "service_tier_source",
            value
        } if value == "gateway_override"
    ));
}

#[test]
fn prefixed_model_and_reasoning_source_queries_are_supported() {
    let client_model_query = parse_request_log_query(Some("client_model:gpt-client"));
    assert!(matches!(
        client_model_query,
        RequestLogQuery::FieldLike {
            column: "client_model",
            pattern
        } if pattern == "%gpt-client%"
    ));

    let model_source_query = parse_request_log_query(Some("model_source:=gateway_override"));
    assert!(matches!(
        model_source_query,
        RequestLogQuery::FieldExact {
            column: "model_source",
            value
        } if value == "gateway_override"
    ));

    let client_reasoning_query = parse_request_log_query(Some("client_reasoning:minimal"));
    assert!(matches!(
        client_reasoning_query,
        RequestLogQuery::FieldLike {
            column: "client_reasoning_effort",
            pattern
        } if pattern == "%minimal%"
    ));

    let reasoning_source_query = parse_request_log_query(Some("reasoning_source:=api_key_profile"));
    assert!(matches!(
        reasoning_source_query,
        RequestLogQuery::FieldExact {
            column: "reasoning_source",
            value
        } if value == "api_key_profile"
    ));
}

#[test]
fn prefixed_route_strategy_queries_are_supported() {
    let route_strategy_query = parse_request_log_query(Some("route_strategy:=balanced"));
    assert!(matches!(
        route_strategy_query,
        RequestLogQuery::FieldExact {
            column: "route_strategy",
            value
        } if value == "balanced"
    ));

    let route_source_query = parse_request_log_query(Some("route_source:conversation"));
    assert!(matches!(
        route_source_query,
        RequestLogQuery::FieldLike {
            column: "route_source",
            pattern
        } if pattern == "%conversation%"
    ));
}

#[test]
fn prefixed_route_detail_queries_are_supported() {
    let upstream_model_query = parse_request_log_query(Some("upstream_model:gpt-real"));
    assert!(matches!(
        upstream_model_query,
        RequestLogQuery::FieldLike {
            column: "upstream_model",
            pattern
        } if pattern == "%gpt-real%"
    ));

    let source_kind_query = parse_request_log_query(Some("source_kind:=aggregate_api"));
    assert!(matches!(
        source_kind_query,
        RequestLogQuery::FieldExact {
            column: "actual_source_kind",
            value
        } if value == "aggregate_api"
    ));

    let source_id_query = parse_request_log_query(Some("source_id:ag_123"));
    assert!(matches!(
        source_id_query,
        RequestLogQuery::FieldLike {
            column: "actual_source_id",
            pattern
        } if pattern == "%ag_123%"
    ));
}
