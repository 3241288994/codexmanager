use serde_json::Value;

/// 函数 `retain_fields_with_allowlist`
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
pub(super) fn retain_fields_with_allowlist(
    obj: &mut serde_json::Map<String, Value>,
    allow: fn(&str) -> bool,
) -> Vec<String> {
    let dropped = obj
        .keys()
        .filter(|key| !allow(key.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if dropped.is_empty() {
        return dropped;
    }
    obj.retain(|key, _| allow(key.as_str()));
    dropped
}

/// 函数 `normalize_path`
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
pub(super) fn normalize_path(path: &str) -> &str {
    path.split('?').next().unwrap_or(path)
}

/// 函数 `path_matches_template`
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
pub(super) fn path_matches_template(path: &str, template: &str) -> bool {
    let normalized_path = normalize_path(path);
    let mut path_segments = normalized_path
        .trim_end_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty());
    let mut template_segments = template
        .trim_end_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty());

    loop {
        match (template_segments.next(), path_segments.next()) {
            (None, None) => return true,
            (Some(_), None) | (None, Some(_)) => return false,
            (Some(template_segment), Some(path_segment)) => {
                if template_segment.starts_with('{') && template_segment.ends_with('}') {
                    if path_segment.is_empty() {
                        return false;
                    }
                    continue;
                }
                if template_segment != path_segment {
                    return false;
                }
            }
        }
    }
}

pub(super) struct TemplateAllowlist {
    pub(super) template: &'static str,
    pub(super) allow: fn(&str) -> bool,
}

/// 函数 `retain_fields_by_templates`
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
pub(super) fn retain_fields_by_templates(
    path: &str,
    obj: &mut serde_json::Map<String, Value>,
    templates: &[TemplateAllowlist],
) -> Vec<String> {
    for template in templates {
        if path_matches_template(path, template.template) {
            return retain_fields_with_allowlist(obj, template.allow);
        }
    }
    Vec::new()
}
