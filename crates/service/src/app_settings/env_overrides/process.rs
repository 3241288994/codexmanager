use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Mutex, OnceLock};

static ENV_OVERRIDE_BASELINE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

/// 函数 `env_override_baseline`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
fn env_override_baseline() -> &'static Mutex<HashMap<String, Option<String>>> {
    ENV_OVERRIDE_BASELINE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 函数 `env_override_original_process_value`
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
pub(super) fn env_override_original_process_value(key: &str) -> Option<String> {
    let baseline =
        crate::lock_utils::lock_recover(env_override_baseline(), "env_override_baseline");
    if let Some(value) = baseline.get(key) {
        return value.clone();
    }
    drop(baseline);
    super::normalize_optional_text(std::env::var(key).ok().as_deref())
}

/// 函数 `apply_env_overrides_to_process`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 无
pub(crate) fn apply_env_overrides_to_process(
    previous: &BTreeMap<String, String>,
    next: &BTreeMap<String, String>,
) {
    let mut all_keys = BTreeSet::new();
    all_keys.extend(previous.keys().cloned());
    all_keys.extend(next.keys().cloned());
    if all_keys.is_empty() {
        return;
    }

    let mut baseline =
        crate::lock_utils::lock_recover(env_override_baseline(), "env_override_baseline");
    for key in &all_keys {
        baseline
            .entry(key.clone())
            .or_insert_with(|| super::normalize_optional_text(std::env::var(key).ok().as_deref()));
    }

    for key in all_keys {
        if let Some(value) = next.get(&key) {
            if value.trim().is_empty() {
                if let Some(original) = baseline.get(&key).and_then(|value| value.clone()) {
                    std::env::set_var(&key, original);
                } else {
                    std::env::remove_var(&key);
                }
            } else {
                std::env::set_var(&key, value);
            }
            continue;
        }
        if let Some(original) = baseline.get(&key).and_then(|value| value.clone()) {
            std::env::set_var(&key, original);
        } else {
            std::env::remove_var(&key);
        }
    }
}

/// 函数 `reload_runtime_after_env_override_apply`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 无
pub(crate) fn reload_runtime_after_env_override_apply() {
    crate::gateway::reload_runtime_config_from_env();
    crate::usage_refresh::reload_background_tasks_runtime_from_env();
    crate::usage_http::reload_usage_http_client_from_env();
}
