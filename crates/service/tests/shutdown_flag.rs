/// 函数 `shutdown_flag_can_toggle`
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
fn shutdown_flag_can_toggle() {
    assert_eq!(codexmanager_service::shutdown_requested(), false);
    codexmanager_service::request_shutdown("localhost:0");
    assert_eq!(codexmanager_service::shutdown_requested(), true);
    codexmanager_service::clear_shutdown_flag();
    assert_eq!(codexmanager_service::shutdown_requested(), false);
}
