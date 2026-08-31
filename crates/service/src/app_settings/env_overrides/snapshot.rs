use serde_json::Value;
use std::collections::BTreeMap;

/// 函数 `env_override_default_value`
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
pub(super) fn env_override_default_value(key: &str) -> String {
    super::process::env_override_original_process_value(key).unwrap_or_else(|| {
        super::catalog::env_override_catalog_item(key)
            .map(|item| item.default_value.to_string())
            .unwrap_or_default()
    })
}

/// 函数 `env_override_default_snapshot`
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
pub(super) fn env_override_default_snapshot() -> BTreeMap<String, String> {
    let mut snapshot = BTreeMap::new();
    for item in super::catalog::editable_env_override_catalog() {
        snapshot.insert(item.key.to_string(), env_override_default_value(item.key));
    }
    snapshot
}

/// 函数 `persisted_env_overrides`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - normalized: 参数 normalized
///
/// # 返回
/// 返回函数执行结果
fn persisted_env_overrides(mut normalized: BTreeMap<String, String>) -> BTreeMap<String, String> {
    let Some(raw) = super::get_persisted_app_setting(super::APP_SETTING_ENV_OVERRIDES_KEY) else {
        return normalized;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Map<String, Value>>(&raw) else {
        log::warn!("parse persisted env overrides failed: invalid json");
        return normalized;
    };

    for (raw_key, raw_value) in parsed {
        let normalized_key = raw_key.trim().to_ascii_uppercase();
        if super::catalog::is_env_override_reserved_key(&normalized_key) {
            continue;
        }
        let Ok(key) = super::normalize::normalize_env_override_key(&raw_key) else {
            log::warn!(
                "skip persisted env override: key={} invalid",
                raw_key.trim()
            );
            continue;
        };
        if let Some(value) = super::normalize::parse_saved_env_override_value(&raw_value) {
            if super::catalog::is_env_override_catalog_key(&key) || !value.is_empty() {
                normalized.insert(key, value);
            } else {
                normalized.remove(&key);
            }
        }
    }
    normalized
}

/// 函数 `persisted_env_overrides_only`
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
pub(crate) fn persisted_env_overrides_only() -> BTreeMap<String, String> {
    persisted_env_overrides(BTreeMap::new())
}

/// 函数 `persisted_env_overrides_missing_process_env`
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
pub(crate) fn persisted_env_overrides_missing_process_env() -> BTreeMap<String, String> {
    persisted_env_overrides_only()
        .into_iter()
        .filter(|(key, _)| std::env::var_os(key).is_none())
        .collect()
}

/// 函数 `current_env_overrides`
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
pub(crate) fn current_env_overrides() -> BTreeMap<String, String> {
    let mut current = env_override_default_snapshot();
    for (key, value) in persisted_env_overrides_only() {
        current.insert(key, value);
    }
    current
}

/// 函数 `save_env_overrides_value`
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
pub(crate) fn save_env_overrides_value(overrides: &BTreeMap<String, String>) -> Result<(), String> {
    let sanitized = overrides
        .iter()
        .filter(|(key, _)| !super::catalog::is_env_override_reserved_key(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    let raw = serde_json::to_string(&sanitized)
        .map_err(|err| format!("serialize env overrides failed: {err}"))?;
    super::save_persisted_app_setting(super::APP_SETTING_ENV_OVERRIDES_KEY, Some(&raw))
}
