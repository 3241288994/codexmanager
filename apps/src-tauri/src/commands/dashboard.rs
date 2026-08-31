use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_dashboard_admin_usage_summary(
    addr: Option<String>,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "dashboard/adminUsageSummary",
        addr,
        Some(serde_json::json!({
            "startTs": start_ts,
            "endTs": end_ts,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_dashboard_member_summary(
    addr: Option<String>,
    user_id: Option<String>,
    day_start_ts: Option<i64>,
    day_end_ts: Option<i64>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "dashboard/memberSummary",
        addr,
        Some(serde_json::json!({
            "userId": user_id,
            "dayStartTs": day_start_ts,
            "dayEndTs": day_end_ts,
        })),
    )
    .await
}
