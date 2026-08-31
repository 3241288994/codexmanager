use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};

use crate::RpcActor;

/// 函数 `try_handle`
///
/// 作者: gaohongshun
///
/// 时间: 2026-05-11
///
/// # 参数
/// - req: 参数 req
///
/// # 返回
/// 返回函数执行结果
pub(super) fn try_handle(req: &JsonRpcRequest, actor: &RpcActor) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "accountManager/status" => super::value_or_error(crate::app_auth_status_value()),
        "accountManager/session/current" => super::value_or_error(crate::app_session_result(actor)),
        "accountManager/profile/update" => {
            let display_name = super::str_param(req, "displayName");
            super::value_or_error(crate::update_app_user_profile(actor, display_name))
        }
        "accountManager/password/change" => {
            let current_password = super::str_param(req, "currentPassword").unwrap_or("");
            let new_password = super::str_param(req, "newPassword").unwrap_or("");
            super::ok_or_error(crate::change_app_user_password(
                actor,
                current_password,
                new_password,
            ))
        }
        "accountManager/users/list" => super::value_or_error(crate::list_app_users()),
        "accountManager/users/create" => {
            let input = req
                .params
                .clone()
                .map(serde_json::from_value::<crate::AppUserCreateInput>)
                .transpose()
                .map_err(|err| format!("invalid user payload: {err}"));
            super::value_or_error(
                input
                    .and_then(|input| input.ok_or_else(|| "missing user payload".to_string()))
                    .and_then(crate::create_app_user),
            )
        }
        "accountManager/users/update" => {
            let input = req
                .params
                .clone()
                .map(serde_json::from_value::<crate::AppUserUpdateInput>)
                .transpose()
                .map_err(|err| format!("invalid user payload: {err}"));
            super::value_or_error(
                input
                    .and_then(|input| input.ok_or_else(|| "missing user payload".to_string()))
                    .and_then(crate::update_app_user),
            )
        }
        "accountManager/users/delete" => {
            let user_id = super::str_param(req, "id").unwrap_or("");
            super::ok_or_error(crate::delete_app_user(user_id))
        }
        "accountManager/wallet/topUp" => {
            let owner_kind = super::str_param(req, "ownerKind").unwrap_or("user");
            let owner_id = super::str_param(req, "ownerId").unwrap_or("");
            let amount = super::i64_param(req, "amountCreditMicros").unwrap_or(0);
            let note = super::str_param(req, "note");
            let created_by = super::str_param(req, "createdByUserId");
            super::value_or_error(crate::wallet_top_up(
                owner_kind, owner_id, amount, note, created_by,
            ))
        }
        "accountManager/wallet/setAvailable" => {
            let owner_kind = super::str_param(req, "ownerKind").unwrap_or("user");
            let owner_id = super::str_param(req, "ownerId").unwrap_or("");
            let amount = super::i64_param(req, "availableCreditMicros").unwrap_or(0);
            let note = super::str_param(req, "note");
            let created_by = super::str_param(req, "createdByUserId");
            super::value_or_error(crate::wallet_set_available_credit(
                owner_kind, owner_id, amount, note, created_by,
            ))
        }
        "accountManager/apiKeyOwners/list" => super::value_or_error(crate::list_api_key_owners()),
        "accountManager/apiKeyOwners/set" => {
            let key_id = super::str_param(req, "keyId").unwrap_or("");
            let owner_kind = super::str_param(req, "ownerKind").unwrap_or("user");
            let owner_user_id = super::str_param(req, "ownerUserId");
            let project_id = super::str_param(req, "projectId");
            super::value_or_error(crate::set_api_key_owner(
                key_id,
                owner_kind,
                owner_user_id,
                project_id,
            ))
        }
        "accountManager/webAuthMode/set" => {
            let mode = super::str_param(req, "mode").unwrap_or("none");
            super::value_or_error(
                crate::set_web_auth_mode(mode).map(|mode| serde_json::json!({ "mode": mode })),
            )
        }
        "accountManager/distribution/set" => {
            let enabled = super::bool_param(req, "enabled").unwrap_or(false);
            super::value_or_error(
                crate::set_distribution_enabled(enabled)
                    .map(|enabled| serde_json::json!({ "distributionEnabled": enabled })),
            )
        }
        _ => return None,
    };

    Some(super::response(req, result))
}
