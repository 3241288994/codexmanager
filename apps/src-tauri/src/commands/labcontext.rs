use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_labcontext_overview(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("labcontext/overview", addr, None).await
}

#[tauri::command]
pub async fn service_labcontext_set_default_workspace(
    addr: Option<String>,
    workspace_id: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/setDefaultWorkspace",
        addr,
        Some(serde_json::json!({"workspaceId": workspace_id})),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_refresh_workspace(
    addr: Option<String>,
    workspace_id: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/refreshWorkspace",
        addr,
        Some(serde_json::json!({"workspaceId": workspace_id})),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_test_tool(
    addr: Option<String>,
    tool: String,
    workspace_id: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/testTool",
        addr,
        Some(serde_json::json!({"tool": tool, "workspaceId": workspace_id})),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_set_tool_policy(
    addr: Option<String>,
    profile: String,
    disabled_tools: Vec<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/setToolPolicy",
        addr,
        Some(serde_json::json!({"profile": profile, "disabledTools": disabled_tools})),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_upsert_workspace(
    addr: Option<String>,
    name: String,
    root: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/upsertWorkspace",
        addr,
        Some(serde_json::json!({
            "name": name, "root": root,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_delete_workspace(
    addr: Option<String>,
    workspace_id: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/deleteWorkspace",
        addr,
        Some(serde_json::json!({
            "workspaceId": workspace_id,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_set_workspace_overview(
    addr: Option<String>,
    workspace_id: String,
    overview: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/setWorkspaceOverview",
        addr,
        Some(serde_json::json!({
            "workspaceId": workspace_id, "overview": overview,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_generate_workspace_overview(
    addr: Option<String>,
    workspace_id: String,
    refresh: bool,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/generateWorkspaceOverview",
        addr,
        Some(serde_json::json!({
            "workspaceId": workspace_id, "refresh": refresh,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_set_worker_config(
    addr: Option<String>,
    model: String,
    reasoning_effort: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/setWorkerConfig",
        addr,
        Some(serde_json::json!({
            "model": model, "reasoningEffort": reasoning_effort,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_get_research_map(
    addr: Option<String>,
    workspace_id: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/getResearchMap",
        addr,
        Some(serde_json::json!({"workspaceId": workspace_id})),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_initialize_research_map(
    addr: Option<String>,
    workspace_id: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/initializeResearchMap",
        addr,
        Some(serde_json::json!({"workspaceId": workspace_id})),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_save_research_map_layout(
    addr: Option<String>,
    workspace_id: String,
    layout: serde_json::Value,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/saveResearchMapLayout",
        addr,
        Some(serde_json::json!({
            "workspaceId": workspace_id, "layout": layout,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_apply_research_map_patch(
    addr: Option<String>,
    workspace_id: String,
    patch: serde_json::Value,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/applyResearchMapPatch",
        addr,
        Some(serde_json::json!({
            "workspaceId": workspace_id, "patch": patch,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_review_research_map(
    addr: Option<String>,
    workspace_id: String,
    prefer_queue: bool,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/reviewResearchMap",
        addr,
        Some(serde_json::json!({
            "workspaceId": workspace_id, "preferQueue": prefer_queue,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_labcontext_research_map_proposal_action(
    addr: Option<String>,
    workspace_id: String,
    proposal_id: String,
    action: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "labcontext/researchMapProposalAction",
        addr,
        Some(serde_json::json!({
            "workspaceId": workspace_id, "proposalId": proposal_id, "action": action,
        })),
    )
    .await
}
