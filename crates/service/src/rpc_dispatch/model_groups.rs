use codexmanager_core::rpc::types::{
    JsonRpcRequest, JsonRpcResponse, ModelGroupModelsSetParams, ModelGroupUpsertParams,
    ModelGroupUsersSetParams,
};

use crate::RpcActor;

pub(super) fn try_handle(req: &JsonRpcRequest, _actor: &RpcActor) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "modelGroups/list" => super::value_or_error(crate::read_model_groups()),
        "modelGroups/save" => {
            let params = req
                .params
                .clone()
                .map(serde_json::from_value::<ModelGroupUpsertParams>)
                .transpose()
                .map_err(|err| format!("invalid model group payload: {err}"));
            super::value_or_error(
                params
                    .and_then(|params| {
                        params.ok_or_else(|| "missing model group payload".to_string())
                    })
                    .and_then(crate::upsert_model_group),
            )
        }
        "modelGroups/delete" => {
            let id = super::str_param(req, "id").unwrap_or("");
            super::value_or_error(crate::delete_model_group(id))
        }
        "modelGroups/setModels" => {
            let params = req
                .params
                .clone()
                .map(serde_json::from_value::<ModelGroupModelsSetParams>)
                .transpose()
                .map_err(|err| format!("invalid model group models payload: {err}"));
            super::value_or_error(
                params
                    .and_then(|params| {
                        params.ok_or_else(|| "missing model group models payload".to_string())
                    })
                    .and_then(crate::set_model_group_models),
            )
        }
        "modelGroups/setUsers" => {
            let params = req
                .params
                .clone()
                .map(serde_json::from_value::<ModelGroupUsersSetParams>)
                .transpose()
                .map_err(|err| format!("invalid model group users payload: {err}"));
            super::value_or_error(
                params
                    .and_then(|params| {
                        params.ok_or_else(|| "missing model group users payload".to_string())
                    })
                    .and_then(crate::set_model_group_users),
            )
        }
        _ => return None,
    };

    Some(super::response(req, result))
}
