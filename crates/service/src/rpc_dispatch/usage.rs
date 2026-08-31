use codexmanager_core::rpc::types::{
    JsonRpcRequest, JsonRpcResponse, UsageListResult, UsageReadResult,
};

use crate::{usage_aggregate, usage_list, usage_read, usage_refresh};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "account/usage/read" => {
            let account_id =
                super::str_param(req, "accountId").or_else(|| super::str_param(req, "account_id"));
            super::as_json(UsageReadResult {
                snapshot: usage_read::read_usage_snapshot(account_id),
            })
        }
        "account/usage/list" => super::value_or_error(
            usage_list::read_usage_snapshots().map(|items| UsageListResult { items }),
        ),
        "account/usage/aggregate" => {
            super::value_or_error(usage_aggregate::read_usage_aggregate_summary())
        }
        "account/usage/refresh" => {
            let account_id =
                super::str_param(req, "accountId").or_else(|| super::str_param(req, "account_id"));
            let refreshed = match account_id {
                Some(account_id) => usage_refresh::refresh_usage_for_account(account_id)
                    .and_then(serialize_refresh_result),
                None => usage_refresh::refresh_usage_for_all_accounts_report()
                    .and_then(serialize_refresh_result),
            };
            super::value_or_error(refreshed)
        }
        "account/usage/pollingStatus" => {
            let settings = usage_refresh::background_tasks_settings();
            super::as_json(settings)
        }
        _ => return None,
    };
    Some(super::response(req, result))
}

fn serialize_refresh_result<T: serde::Serialize>(value: T) -> Result<serde_json::Value, String> {
    serde_json::to_value(value)
        .map_err(|err| format!("serialize usage refresh result failed: {err}"))
}
