use super::{resolve_backend_route, BackendRoute};

/// 函数 `resolves_rpc_route`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn resolves_rpc_route() {
    assert_eq!(resolve_backend_route("POST", "/rpc"), BackendRoute::Rpc);
}

/// 函数 `resolves_auth_callback_route`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn resolves_auth_callback_route() {
    assert_eq!(
        resolve_backend_route("GET", "/auth/callback?code=123"),
        BackendRoute::AuthCallback
    );
}

#[test]
fn rejects_removed_usage_refresh_events_route() {
    assert_eq!(
        resolve_backend_route("GET", "/events/usage-refresh"),
        BackendRoute::NotFound
    );
}

/// 函数 `resolves_metrics_route`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn rejects_removed_metrics_route() {
    assert_eq!(
        resolve_backend_route("GET", "/metrics"),
        BackendRoute::NotFound
    );
}

/// 函数 `falls_back_to_gateway_route`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn rejects_removed_gateway_route() {
    assert_eq!(
        resolve_backend_route("POST", "/v1/responses"),
        BackendRoute::NotFound
    );
}
