use crate::commands::shared::rpc_call_in_background;

/// 函数 `service_aggregate_api_list`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_list(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("aggregateApi/list", addr, None).await
}

/// 函数 `service_aggregate_api_create`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - provider_type: 参数 provider_type
/// - supplier_name: 参数 supplier_name
/// - sort: 参数 sort
/// - url: 参数 url
/// - key: 参数 key
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_create(
    addr: Option<String>,
    provider_type: Option<String>,
    supplier_name: Option<String>,
    sort: Option<i64>,
    url: Option<String>,
    key: Option<String>,
    auth_type: Option<String>,
    auth_custom_enabled: Option<bool>,
    auth_params: Option<serde_json::Value>,
    action_custom_enabled: Option<bool>,
    action: Option<String>,
    model_override: Option<String>,
    username: Option<String>,
    password: Option<String>,
    balance_query_enabled: Option<bool>,
    balance_query_template: Option<String>,
    balance_query_base_url: Option<String>,
    balance_query_access_token: Option<String>,
    balance_query_user_id: Option<String>,
    balance_query_config_json: Option<String>,
    model_slugs: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "providerType": provider_type,
        "supplierName": supplier_name,
        "sort": sort,
        "url": url,
        "key": key,
        "authType": auth_type,
        "authCustomEnabled": auth_custom_enabled,
        "authParams": auth_params,
        "actionCustomEnabled": action_custom_enabled,
        "action": action,
        "modelOverride": model_override,
        "username": username,
        "password": password,
        "balanceQueryEnabled": balance_query_enabled,
        "balanceQueryTemplate": balance_query_template,
        "balanceQueryBaseUrl": balance_query_base_url,
        "balanceQueryAccessToken": balance_query_access_token,
        "balanceQueryUserId": balance_query_user_id,
        "balanceQueryConfigJson": balance_query_config_json,
        "modelSlugs": model_slugs,
    });
    rpc_call_in_background("aggregateApi/create", addr, Some(params)).await
}

/// 函数 `service_aggregate_api_update`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - id: 参数 id
/// - provider_type: 参数 provider_type
/// - supplier_name: 参数 supplier_name
/// - sort: 参数 sort
/// - url: 参数 url
/// - key: 参数 key
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_update(
    addr: Option<String>,
    id: String,
    provider_type: Option<String>,
    supplier_name: Option<String>,
    sort: Option<i64>,
    status: Option<String>,
    url: Option<String>,
    key: Option<String>,
    auth_type: Option<String>,
    auth_custom_enabled: Option<bool>,
    auth_params: Option<serde_json::Value>,
    action_custom_enabled: Option<bool>,
    action: Option<String>,
    model_override: Option<String>,
    username: Option<String>,
    password: Option<String>,
    balance_query_enabled: Option<bool>,
    balance_query_template: Option<String>,
    balance_query_base_url: Option<String>,
    balance_query_access_token: Option<String>,
    balance_query_user_id: Option<String>,
    balance_query_config_json: Option<String>,
    model_slugs: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "id": id,
        "providerType": provider_type,
        "supplierName": supplier_name,
        "sort": sort,
        "status": status,
        "url": url,
        "key": key,
        "authType": auth_type,
        "authCustomEnabled": auth_custom_enabled,
        "authParams": auth_params,
        "actionCustomEnabled": action_custom_enabled,
        "action": action,
        "modelOverride": model_override,
        "username": username,
        "password": password,
        "balanceQueryEnabled": balance_query_enabled,
        "balanceQueryTemplate": balance_query_template,
        "balanceQueryBaseUrl": balance_query_base_url,
        "balanceQueryAccessToken": balance_query_access_token,
        "balanceQueryUserId": balance_query_user_id,
        "balanceQueryConfigJson": balance_query_config_json,
        "modelSlugs": model_slugs,
    });
    rpc_call_in_background("aggregateApi/update", addr, Some(params)).await
}

/// 函数 `service_aggregate_api_read_secret`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - id: 参数 id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_read_secret(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("aggregateApi/readSecret", addr, Some(params)).await
}

/// 函数 `service_aggregate_api_delete`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - id: 参数 id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_delete(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("aggregateApi/delete", addr, Some(params)).await
}

/// 函数 `service_aggregate_api_test_connection`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - id: 参数 id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_test_connection(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("aggregateApi/testConnection", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_aggregate_api_refresh_balance(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("aggregateApi/refreshBalance", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_aggregate_api_supplier_models_list(
    addr: Option<String>,
    supplier_key: Option<String>,
    provider_type: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "supplierKey": supplier_key,
        "providerType": provider_type,
    });
    rpc_call_in_background("aggregateApi/supplierModels/list", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_aggregate_api_supplier_model_save(
    addr: Option<String>,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("aggregateApi/supplierModels/save", addr, Some(payload)).await
}

#[tauri::command]
pub async fn service_aggregate_api_supplier_model_delete(
    addr: Option<String>,
    supplier_key: String,
    provider_type: String,
    upstream_model: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "supplierKey": supplier_key,
        "providerType": provider_type,
        "upstreamModel": upstream_model,
    });
    rpc_call_in_background("aggregateApi/supplierModels/delete", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_aggregate_api_supplier_models_import(
    addr: Option<String>,
    api_id: String,
    supplier_key: Option<String>,
    provider_type: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "apiId": api_id,
        "supplierKey": supplier_key,
        "providerType": provider_type,
    });
    rpc_call_in_background(
        "aggregateApi/sourceModels/importSupplier",
        addr,
        Some(params),
    )
    .await
}
