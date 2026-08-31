use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_session_catalog_list(
    addr: Option<String>,
    codex_home: Option<String>,
    query: Option<String>,
    include_archived: bool,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "sessionCatalog/list",
        addr,
        Some(serde_json::json!({
            "codexHome": codex_home,
            "query": query,
            "includeArchived": include_archived,
            "limit": limit,
        })),
    )
    .await
}

#[tauri::command]
pub async fn service_session_catalog_repair_provider_index(
    addr: Option<String>,
    codex_home: Option<String>,
    session_id: String,
    confirm_session_id: String,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background(
        "sessionCatalog/repairProviderIndex",
        addr,
        Some(serde_json::json!({
            "codexHome": codex_home,
            "sessionId": session_id,
            "confirmSessionId": confirm_session_id,
        })),
    )
    .await
}
