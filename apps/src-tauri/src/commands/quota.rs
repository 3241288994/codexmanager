use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_quota_overview(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("quota/overview", addr, None).await
}

#[tauri::command]
pub async fn service_quota_model_usage(
    addr: Option<String>,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "quota/modelUsage",
        addr,
        Some(serde_json::json!({
            "startTs": start_ts,
            "endTs": end_ts,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_quota_api_key_usage(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("quota/apiKeyUsage", addr, None).await
}

#[tauri::command]
pub async fn service_quota_source_list(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("quota/sourceList", addr, None).await
}

#[tauri::command]
pub async fn service_quota_model_pools(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("quota/modelPools", addr, None).await
}

#[tauri::command]
pub async fn service_quota_system_pool(
    addr: Option<String>,
    reference_model: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "quota/systemPool",
        addr,
        Some(serde_json::json!({
            "referenceModel": reference_model,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_quota_capacity_config(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("quota/capacityConfig", addr, None).await
}

#[tauri::command]
pub async fn service_quota_billing_rules(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("quota/billingRules", addr, None).await
}

#[tauri::command]
pub async fn service_quota_billing_rule_upsert(
    addr: Option<String>,
    id: Option<String>,
    name: String,
    status: Option<String>,
    priority: Option<i64>,
    multiplier_millis: i64,
    model_pattern: Option<String>,
    service_tier: Option<String>,
    user_id: Option<String>,
    api_key_id: Option<String>,
    starts_at: Option<i64>,
    ends_at: Option<i64>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "quota/billingRule/upsert",
        addr,
        Some(serde_json::json!({
            "id": id,
            "name": name,
            "status": status,
            "priority": priority,
            "multiplierMillis": multiplier_millis,
            "modelPattern": model_pattern,
            "serviceTier": service_tier,
            "userId": user_id,
            "apiKeyId": api_key_id,
            "startsAt": starts_at,
            "endsAt": ends_at,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_quota_billing_rule_delete(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "quota/billingRule/delete",
        addr,
        Some(serde_json::json!({ "id": id })),
    )
    .await
}

#[tauri::command]
pub async fn service_quota_source_models_set(
    addr: Option<String>,
    source_kind: String,
    source_id: String,
    model_slugs: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "quota/sourceModels/set",
        addr,
        Some(serde_json::json!({
            "sourceKind": source_kind,
            "sourceId": source_id,
            "modelSlugs": model_slugs.unwrap_or_default(),
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_quota_capacity_template_update(
    addr: Option<String>,
    plan_type: String,
    primary_window_tokens: Option<i64>,
    secondary_window_tokens: Option<i64>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "quota/capacityTemplate/update",
        addr,
        Some(serde_json::json!({
            "planType": plan_type,
            "primaryWindowTokens": primary_window_tokens,
            "secondaryWindowTokens": secondary_window_tokens,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_quota_account_capacity_override_update(
    addr: Option<String>,
    account_id: String,
    primary_window_tokens: Option<i64>,
    secondary_window_tokens: Option<i64>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "quota/accountCapacityOverride/update",
        addr,
        Some(serde_json::json!({
            "accountId": account_id,
            "primaryWindowTokens": primary_window_tokens,
            "secondaryWindowTokens": secondary_window_tokens,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_quota_refresh_sources(
    addr: Option<String>,
    kinds: Option<Vec<String>>,
    source_ids: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "quota/refreshSources",
        addr,
        Some(serde_json::json!({
            "kinds": kinds.unwrap_or_default(),
            "sourceIds": source_ids.unwrap_or_default(),
        })),
    )
    .await
}
