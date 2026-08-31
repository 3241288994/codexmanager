use std::sync::{OnceLock, RwLock};

const ENV_UPSTREAM_BASE_URL: &str = "CODEXMANAGER_UPSTREAM_BASE_URL";
const DEFAULT_UPSTREAM_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";

static CONFIG_LOADED: OnceLock<()> = OnceLock::new();
static UPSTREAM_BASE_URL: OnceLock<RwLock<String>> = OnceLock::new();
static UPSTREAM_FALLBACK_BASE_URL: OnceLock<RwLock<Option<String>>> = OnceLock::new();

/// 函数 `normalize_upstream_base_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn normalize_upstream_base_url(base: &str) -> String {
    let mut normalized = base.trim().trim_end_matches('/').to_string();
    let lower = normalized.to_ascii_lowercase();
    if (lower.starts_with("https://chatgpt.com") || lower.starts_with("https://chat.openai.com"))
        && !lower.contains("/backend-api")
    {
        // 中文注释：对齐官方客户端的主机归一化，避免仅填域名时落到错误路径。
        normalized = format!("{normalized}/backend-api/codex");
    }
    normalized
}

/// 函数 `resolve_upstream_base_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn resolve_upstream_base_url() -> String {
    ensure_config_loaded();
    crate::lock_utils::read_recover(upstream_base_url_cell(), "upstream_base_url").clone()
}

/// 函数 `resolve_upstream_fallback_base_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn resolve_upstream_fallback_base_url(primary_base: &str) -> Option<String> {
    let _ = primary_base;
    None
}

/// 函数 `is_openai_api_base`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn is_openai_api_base(base: &str) -> bool {
    let normalized = base.trim().to_ascii_lowercase();
    normalized.contains("api.openai.com/v1")
}

/// 函数 `is_chatgpt_backend_base`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn is_chatgpt_backend_base(base: &str) -> bool {
    let normalized = base.trim().to_ascii_lowercase();
    normalized.contains("chatgpt.com/backend-api")
        || normalized.contains("chat.openai.com/backend-api")
}

/// 函数 `should_send_chatgpt_account_header`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn should_send_chatgpt_account_header(base: &str) -> bool {
    base.trim()
        .to_ascii_lowercase()
        .contains("/backend-api/codex")
}

/// 函数 `is_official_openai_target`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn is_official_openai_target(base: &str) -> bool {
    is_chatgpt_backend_base(base) || is_openai_api_base(base)
}

/// 函数 `should_try_openai_fallback`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn should_try_openai_fallback(
    base: &str,
    request_path: &str,
    content_type: Option<&reqwest::header::HeaderValue>,
) -> bool {
    let _ = (base, request_path, content_type);
    false
}

/// 函数 `should_try_openai_fallback_by_status`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn should_try_openai_fallback_by_status(
    base: &str,
    request_path: &str,
    status_code: u16,
) -> bool {
    let _ = (base, request_path, status_code);
    false
}

/// 函数 `reload_from_env`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 无
pub(in super::super) fn reload_from_env() {
    let base = env_non_empty(ENV_UPSTREAM_BASE_URL)
        .map(|value| normalize_upstream_base_url(&value))
        .unwrap_or_else(|| DEFAULT_UPSTREAM_BASE_URL.to_string());
    let mut cached_base =
        crate::lock_utils::write_recover(upstream_base_url_cell(), "upstream_base_url");
    *cached_base = base;
    let mut cached_fallback_base = crate::lock_utils::write_recover(
        upstream_fallback_base_url_cell(),
        "upstream_fallback_base_url",
    );
    *cached_fallback_base = None;
}

/// 函数 `ensure_config_loaded`
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
fn ensure_config_loaded() {
    let _ = CONFIG_LOADED.get_or_init(|| reload_from_env());
}

/// 函数 `upstream_base_url_cell`
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
fn upstream_base_url_cell() -> &'static RwLock<String> {
    UPSTREAM_BASE_URL.get_or_init(|| RwLock::new(DEFAULT_UPSTREAM_BASE_URL.to_string()))
}

/// 函数 `upstream_fallback_base_url_cell`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-03
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
fn upstream_fallback_base_url_cell() -> &'static RwLock<Option<String>> {
    UPSTREAM_FALLBACK_BASE_URL.get_or_init(|| RwLock::new(None))
}

/// 函数 `env_non_empty`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - name: 参数 name
///
/// # 返回
/// 返回函数执行结果
fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
#[path = "tests/config_tests.rs"]
mod tests;
