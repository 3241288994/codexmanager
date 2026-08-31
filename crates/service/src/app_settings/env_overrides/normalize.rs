use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// 函数 `normalize_env_override_key`
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
pub(super) fn normalize_env_override_key(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return Err("environment variable key is empty".to_string());
    }
    if !normalized.starts_with("CODEXMANAGER_") {
        return Err(format!("{normalized} must start with CODEXMANAGER_"));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(format!("{normalized} contains unsupported characters"));
    }
    if super::catalog::is_env_override_unsupported_key(&normalized) {
        return Err(format!(
            "{normalized} must stay in process/.env because it is required before app_settings can be loaded"
        ));
    }
    if super::catalog::is_env_override_reserved_key(&normalized) {
        return Err(format!(
            "{normalized} is already managed by an existing settings card; update it there instead"
        ));
    }
    Ok(normalized)
}

/// 函数 `normalize_env_override_patch_value`
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
pub(super) fn normalize_env_override_patch_value(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

/// 函数 `normalize_saved_env_override_text`
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
pub(super) fn normalize_saved_env_override_text(raw: &str) -> String {
    raw.trim().to_string()
}

/// 函数 `normalize_env_overrides_patch`
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
pub(super) fn normalize_env_overrides_patch(
    overrides: HashMap<String, String>,
) -> Result<BTreeMap<String, Option<String>>, String> {
    let mut normalized = BTreeMap::new();
    for (raw_key, raw_value) in overrides {
        let key = normalize_env_override_key(&raw_key)?;
        normalized.insert(key, normalize_env_override_patch_value(Some(&raw_value)));
    }
    Ok(normalized)
}

/// 函数 `parse_saved_env_override_value`
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
pub(super) fn parse_saved_env_override_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(normalize_saved_env_override_text(text)),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(if *flag { "1" } else { "0" }.to_string()),
        Value::Null => None,
        _ => None,
    }
}
