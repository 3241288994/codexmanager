use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "sessionCatalog/list" => super::value_or_error(crate::session_catalog::list_sessions(
            super::str_param(req, "codexHome"),
            super::str_param(req, "query"),
            super::bool_param(req, "includeArchived").unwrap_or(false),
            super::i64_param(req, "limit"),
        )),
        "sessionCatalog/repairProviderIndex" => {
            let session_id = super::str_param(req, "sessionId").unwrap_or_default();
            let confirmation = super::str_param(req, "confirmSessionId").unwrap_or_default();
            super::value_or_error(crate::session_catalog::repair_provider_index(
                super::str_param(req, "codexHome"),
                session_id,
                confirmation,
            ))
        }
        _ => return None,
    };
    Some(super::response(req, result))
}
