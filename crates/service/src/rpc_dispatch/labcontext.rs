use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let params = req.params.clone().unwrap_or_else(|| serde_json::json!({}));
    let result = match req.method.as_str() {
        "labcontext/overview" => super::value_or_error(crate::labcontext::get("overview")),
        "labcontext/activity" => super::value_or_error(crate::labcontext::get("activity")),
        "labcontext/jobs" => super::value_or_error(crate::labcontext::get("jobs")),
        "labcontext/setDefaultWorkspace" => {
            super::value_or_error(crate::labcontext::post("default-workspace", params))
        }
        "labcontext/upsertWorkspace" => {
            super::value_or_error(crate::labcontext::post("workspaces", params))
        }
        "labcontext/deleteWorkspace" => {
            super::value_or_error(crate::labcontext::post("delete-workspace", params))
        }
        "labcontext/setWorkspaceOverview" => {
            super::value_or_error(crate::labcontext::post("workspace-overview", params))
        }
        "labcontext/generateWorkspaceOverview" => super::value_or_error(crate::labcontext::post(
            "generate-workspace-overview",
            params,
        )),
        "labcontext/setWorkerConfig" => {
            super::value_or_error(crate::labcontext::post("worker-config", params))
        }
        "labcontext/setToolPolicy" => {
            super::value_or_error(crate::labcontext::post("tool-policy", params))
        }
        "labcontext/refreshWorkspace" => {
            super::value_or_error(crate::labcontext::post("refresh-workspace", params))
        }
        "labcontext/testTool" => {
            super::value_or_error(crate::labcontext::post("test-tool", params))
        }
        "labcontext/getResearchMap" => {
            let workspace_id = params
                .get("workspaceId")
                .and_then(Value::as_str)
                .unwrap_or("");
            super::value_or_error(crate::labcontext::get_with_query(
                "research-map",
                &[("workspace_id", workspace_id)],
            ))
        }
        "labcontext/initializeResearchMap" => {
            super::value_or_error(crate::labcontext::post("research-map/initialize", params))
        }
        "labcontext/saveResearchMapLayout" => {
            super::value_or_error(crate::labcontext::post("research-map/layout", params))
        }
        "labcontext/applyResearchMapPatch" => {
            super::value_or_error(crate::labcontext::post("research-map/patch", params))
        }
        "labcontext/reviewResearchMap" => {
            super::value_or_error(crate::labcontext::post("research-map/review", params))
        }
        "labcontext/researchMapProposalAction" => {
            super::value_or_error(crate::labcontext::post("research-map/proposal", params))
        }
        _ => return None,
    };
    Some(super::response(req, result))
}
