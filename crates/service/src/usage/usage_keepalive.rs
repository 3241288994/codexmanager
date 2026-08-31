/// 函数 `run_gateway_keepalive_once`
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
pub(crate) fn run_gateway_keepalive_once() -> Result<(), String> {
    // 中文注释：定期探活 models 路径可预热上游连接与 token exchange，减少服务空闲后首个请求的冷启动失败概率。
    let _ = crate::gateway::fetch_models_for_picker()?;
    Ok(())
}

/// 函数 `is_keepalive_error_ignorable`
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
pub(crate) fn is_keepalive_error_ignorable(err: &str) -> bool {
    let normalized = err.trim().to_ascii_lowercase();
    normalized.contains("no available account") || normalized.contains("storage unavailable")
}

#[cfg(test)]
#[path = "tests/usage_keepalive_tests.rs"]
mod tests;
