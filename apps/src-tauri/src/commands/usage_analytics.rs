use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_usage_analytics_pricing_refresh(addr: Option<String>, force: Option<bool>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("account/analytics/pricingRefresh", addr, Some(serde_json::json!({"force": force.unwrap_or(false)}))).await
}

#[tauri::command]
pub async fn service_usage_analytics_read(
    addr: Option<String>,
    account_id: String,
    start_date: String,
    end_date: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "account/analytics/read",
        addr,
        Some(serde_json::json!({"accountId":account_id,"startDate":start_date,"endDate":end_date})),
    )
    .await
}
#[tauri::command]
pub async fn service_usage_analytics_refresh(
    addr: Option<String>,
    account_id: String,
    start_date: String,
    end_date: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "account/analytics/refresh",
        addr,
        Some(serde_json::json!({"accountId":account_id,"startDate":start_date,"endDate":end_date})),
    )
    .await
}
#[tauri::command]
pub async fn service_usage_analytics_set_rate(
    addr: Option<String>,
    usd_per_credit: Option<f64>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "account/analytics/setRate",
        addr,
        Some(serde_json::json!({"usdPerCredit":usd_per_credit})),
    )
    .await
}
