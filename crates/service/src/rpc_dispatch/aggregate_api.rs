use codexmanager_core::rpc::types::{
    AggregateApiListResult, AggregateApiSupplierModelDeleteParams,
    AggregateApiSupplierModelImportParams, AggregateApiSupplierModelListResult,
    AggregateApiSupplierModelUpsertParams, JsonRpcRequest, JsonRpcResponse,
};

use crate::{
    create_aggregate_api, delete_aggregate_api, delete_aggregate_api_supplier_model,
    import_aggregate_api_supplier_models, list_aggregate_api_supplier_models, list_aggregate_apis,
    read_aggregate_api_secret, refresh_aggregate_api_balance, save_aggregate_api_supplier_model,
    test_aggregate_api_connection, update_aggregate_api,
};

/// 函数 `api_id_param`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - req: 参数 req
///
/// # 返回
/// 返回函数执行结果
fn api_id_param(req: &JsonRpcRequest) -> Option<&str> {
    super::str_param(req, "id").or_else(|| super::str_param(req, "apiId"))
}

/// 函数 `try_handle`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 返回函数执行结果
pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "aggregateApi/list" => super::value_or_error(
            list_aggregate_apis().map(|items| AggregateApiListResult { items }),
        ),
        "aggregateApi/create" => {
            let provider_type = super::string_param(req, "providerType");
            let supplier_name = super::string_param(req, "supplierName");
            let sort = super::i64_param(req, "sort");
            let url = super::string_param(req, "url");
            let key = super::string_param(req, "key");
            let auth_type = super::string_param(req, "authType");
            let auth_custom_enabled = super::bool_param(req, "authCustomEnabled");
            let auth_params = req
                .params
                .as_ref()
                .and_then(|v| v.get("authParams"))
                .cloned();
            let action_custom_enabled = super::bool_param(req, "actionCustomEnabled");
            let action = super::string_param(req, "action");
            let model_override = super::string_param(req, "modelOverride");
            let username = super::string_param(req, "username");
            let password = super::string_param(req, "password");
            let balance_query_enabled = super::bool_param(req, "balanceQueryEnabled");
            let balance_query_template = super::string_param(req, "balanceQueryTemplate");
            let balance_query_base_url = super::string_param(req, "balanceQueryBaseUrl");
            let balance_query_access_token = super::string_param(req, "balanceQueryAccessToken");
            let balance_query_user_id = super::string_param(req, "balanceQueryUserId");
            let balance_query_config_json = super::string_param(req, "balanceQueryConfigJson");
            let model_slugs = string_array_param(req, "modelSlugs");
            super::value_or_error(create_aggregate_api(
                url,
                key,
                provider_type,
                supplier_name,
                sort,
                auth_type,
                auth_custom_enabled,
                auth_params,
                action_custom_enabled,
                action,
                model_override,
                username,
                password,
                balance_query_enabled,
                balance_query_template,
                balance_query_base_url,
                balance_query_access_token,
                balance_query_user_id,
                balance_query_config_json,
                model_slugs,
            ))
        }
        "aggregateApi/update" => {
            let api_id = api_id_param(req).unwrap_or("");
            let provider_type = super::string_param(req, "providerType");
            let supplier_name = super::string_param(req, "supplierName");
            let sort = super::i64_param(req, "sort");
            let status = super::string_param(req, "status");
            let url = super::string_param(req, "url");
            let key = super::string_param(req, "key");
            let auth_type = super::string_param(req, "authType");
            let auth_custom_enabled = super::bool_param(req, "authCustomEnabled");
            let auth_params = req
                .params
                .as_ref()
                .and_then(|v| v.get("authParams"))
                .cloned();
            let action_custom_enabled = super::bool_param(req, "actionCustomEnabled");
            let action = super::string_param(req, "action");
            let model_override = super::string_param(req, "modelOverride");
            let username = super::string_param(req, "username");
            let password = super::string_param(req, "password");
            let balance_query_enabled = super::bool_param(req, "balanceQueryEnabled");
            let balance_query_template = super::string_param(req, "balanceQueryTemplate");
            let balance_query_base_url = super::string_param(req, "balanceQueryBaseUrl");
            let balance_query_access_token = super::string_param(req, "balanceQueryAccessToken");
            let balance_query_user_id = super::string_param(req, "balanceQueryUserId");
            let balance_query_config_json = super::string_param(req, "balanceQueryConfigJson");
            let model_slugs = string_array_param(req, "modelSlugs");
            super::ok_or_error(update_aggregate_api(
                api_id,
                url,
                key,
                provider_type,
                supplier_name,
                sort,
                status,
                auth_type,
                auth_custom_enabled,
                auth_params,
                action_custom_enabled,
                action,
                model_override,
                username,
                password,
                balance_query_enabled,
                balance_query_template,
                balance_query_base_url,
                balance_query_access_token,
                balance_query_user_id,
                balance_query_config_json,
                model_slugs,
            ))
        }
        "aggregateApi/readSecret" => {
            let api_id = api_id_param(req).unwrap_or("");
            super::value_or_error(read_aggregate_api_secret(api_id))
        }
        "aggregateApi/delete" => {
            let api_id = api_id_param(req).unwrap_or("");
            super::ok_or_error(delete_aggregate_api(api_id))
        }
        "aggregateApi/testConnection" => {
            let api_id = api_id_param(req).unwrap_or("");
            super::value_or_error(test_aggregate_api_connection(api_id))
        }
        "aggregateApi/refreshBalance" => {
            let api_id = api_id_param(req).unwrap_or("");
            super::value_or_error(refresh_aggregate_api_balance(api_id))
        }
        "aggregateApi/supplierModels/list" => {
            let supplier_key = super::string_param(req, "supplierKey");
            let provider_type = super::string_param(req, "providerType");
            super::value_or_error(
                list_aggregate_api_supplier_models(supplier_key, provider_type)
                    .map(|items| AggregateApiSupplierModelListResult { items }),
            )
        }
        "aggregateApi/supplierModels/save" => {
            let params = req
                .params
                .clone()
                .ok_or_else(|| "缺少供应商模型参数".to_string())
                .and_then(|value| {
                    serde_json::from_value::<AggregateApiSupplierModelUpsertParams>(value)
                        .map_err(|err| format!("解析供应商模型参数失败: {err}"))
                });
            super::value_or_error(params.and_then(save_aggregate_api_supplier_model))
        }
        "aggregateApi/supplierModels/delete" => {
            let params = req
                .params
                .clone()
                .ok_or_else(|| "缺少供应商模型参数".to_string())
                .and_then(|value| {
                    serde_json::from_value::<AggregateApiSupplierModelDeleteParams>(value)
                        .map_err(|err| format!("解析供应商模型参数失败: {err}"))
                });
            super::ok_or_error(params.and_then(delete_aggregate_api_supplier_model))
        }
        "aggregateApi/sourceModels/importSupplier" => {
            let params = req
                .params
                .clone()
                .ok_or_else(|| "缺少供应商模型导入参数".to_string())
                .and_then(|value| {
                    serde_json::from_value::<AggregateApiSupplierModelImportParams>(value)
                        .map_err(|err| format!("解析供应商模型导入参数失败: {err}"))
                });
            super::value_or_error(params.and_then(import_aggregate_api_supplier_models))
        }
        _ => return None,
    };

    Some(super::response(req, result))
}

fn string_array_param(req: &JsonRpcRequest, key: &str) -> Option<Vec<String>> {
    req.params
        .as_ref()
        .and_then(|params| params.get(key))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
}

#[cfg(test)]
mod tests {
    use super::try_handle;
    use codexmanager_core::rpc::types::JsonRpcRequest;

    /// 函数 `rpc_request`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - method: 参数 method
    /// - params: 参数 params
    ///
    /// # 返回
    /// 返回函数执行结果
    fn rpc_request(method: &str, params: serde_json::Value) -> JsonRpcRequest {
        JsonRpcRequest {
            id: 1.into(),
            method: method.to_string(),
            params: Some(params),
            trace: None,
        }
    }

    /// 函数 `error_message`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - resp: 参数 resp
    ///
    /// # 返回
    /// 返回函数执行结果
    fn error_message(resp: &codexmanager_core::rpc::types::JsonRpcResponse) -> String {
        resp.result
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string()
    }

    /// 函数 `aggregate_api_update_accepts_id_and_api_id`
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
    fn aggregate_api_update_accepts_id_and_api_id() {
        let missing = try_handle(&rpc_request(
            "aggregateApi/update",
            serde_json::json!({ "supplierName": "codex" }),
        ))
        .expect("response");
        assert_eq!(error_message(&missing), "aggregate api id required");

        let with_id = try_handle(&rpc_request(
            "aggregateApi/update",
            serde_json::json!({ "id": "ag_test", "supplierName": "codex" }),
        ))
        .expect("response");
        assert_ne!(error_message(&with_id), "aggregate api id required");

        let with_api_id = try_handle(&rpc_request(
            "aggregateApi/update",
            serde_json::json!({ "apiId": "ag_test", "supplierName": "codex" }),
        ))
        .expect("response");
        assert_ne!(error_message(&with_api_id), "aggregate api id required");
    }

    /// 函数 `aggregate_api_test_connection_accepts_id_and_api_id`
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
    fn aggregate_api_test_connection_accepts_id_and_api_id() {
        let missing = try_handle(&rpc_request(
            "aggregateApi/testConnection",
            serde_json::json!({}),
        ))
        .expect("response");
        assert_eq!(error_message(&missing), "aggregate api id required");

        let with_id = try_handle(&rpc_request(
            "aggregateApi/testConnection",
            serde_json::json!({ "id": "ag_test" }),
        ))
        .expect("response");
        assert_ne!(error_message(&with_id), "aggregate api id required");

        let with_api_id = try_handle(&rpc_request(
            "aggregateApi/testConnection",
            serde_json::json!({ "apiId": "ag_test" }),
        ))
        .expect("response");
        assert_ne!(error_message(&with_api_id), "aggregate api id required");
    }
}
