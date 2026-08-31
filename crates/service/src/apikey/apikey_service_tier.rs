/// 函数 `normalize_service_tier`
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
pub(crate) fn normalize_service_tier(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "auto" => None,
        "fast" => Some("fast"),
        _ => None,
    }
}

/// 函数 `normalize_service_tier_for_log`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-05
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn normalize_service_tier_for_log(value: &str) -> Option<&'static str> {
    if value.trim().eq_ignore_ascii_case("priority") {
        return Some("fast");
    }
    normalize_service_tier(value)
}

/// 函数 `normalize_service_tier_owned`
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
pub(crate) fn normalize_service_tier_owned(
    value: Option<String>,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "auto" => Ok(None),
        "fast" => Ok(Some("fast".to_string())),
        _ => Err(format!("unsupported service tier: {value}")),
    }
}
