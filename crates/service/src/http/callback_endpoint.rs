use tiny_http::Request;

/// 函数 `handle_callback`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - request: 参数 request
///
/// # 返回
/// 无
pub fn handle_callback(request: Request) {
    if let Err(err) = crate::auth_callback::handle_login_request(request) {
        log::warn!("callback request error: {err}");
    }
}
