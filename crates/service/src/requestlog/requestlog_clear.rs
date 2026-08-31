use crate::storage_helpers::open_storage;

/// 函数 `clear_request_logs`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn clear_request_logs() -> Result<(), String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage.clear_request_logs().map_err(|e| e.to_string())
}
