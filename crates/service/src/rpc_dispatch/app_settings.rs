use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};

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
        "appSettings/get" => super::value_or_error(crate::app_settings_get()),
        // This dispatcher is administrator-only in accounts mode. Preserve the
        // complete typed patch so desktop and Web settings stay equivalent.
        "appSettings/set" => super::value_or_error(crate::app_settings_set(req.params.as_ref())),
        _ => return None,
    };

    Some(super::response(req, result))
}
