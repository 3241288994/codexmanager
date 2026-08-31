/// 函数 `core_version_is_set`
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
fn core_version_is_set() {
    assert!(!codexmanager_core::core_version().is_empty());
}
