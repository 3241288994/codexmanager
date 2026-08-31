use std::collections::HashMap;

/// 函数 `set_env_overrides`
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
pub(crate) fn set_env_overrides(
    overrides: HashMap<String, String>,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let previous = super::snapshot::current_env_overrides();
    let patch = super::normalize::normalize_env_overrides_patch(overrides)?;
    let mut next = if patch.is_empty() {
        super::snapshot::env_override_default_snapshot()
    } else {
        previous.clone()
    };

    for (key, value) in patch {
        if let Some(value) = value {
            next.insert(key, value);
        } else if super::catalog::is_env_override_catalog_key(&key) {
            next.insert(
                key.clone(),
                super::snapshot::env_override_default_value(&key),
            );
        } else {
            next.remove(&key);
        }
    }

    for item in super::catalog::editable_env_override_catalog() {
        next.entry(item.key.to_string())
            .or_insert_with(|| super::snapshot::env_override_default_value(item.key));
    }

    super::snapshot::save_env_overrides_value(&next)?;
    super::process::apply_env_overrides_to_process(&previous, &next);
    super::process::reload_runtime_after_env_override_apply();
    Ok(next)
}
