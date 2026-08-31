use super::{
    get_persisted_app_setting, normalize_optional_text, save_persisted_app_setting,
    APP_SETTING_SERVICE_ADDR_KEY,
};

pub const DEFAULT_ADDR: &str = "localhost:48760";
pub const DEFAULT_BIND_ADDR: &str = "0.0.0.0:48760";
pub const DEFAULT_WEB_ADDR: &str = "localhost:48761";
pub const DEFAULT_WEB_BIND_ADDR: &str = "0.0.0.0:48761";
pub const SERVICE_BIND_MODE_SETTING_KEY: &str = "service.bind_mode";
pub const SERVICE_BIND_MODE_LOOPBACK: &str = "loopback";
pub const SERVICE_BIND_MODE_ALL_INTERFACES: &str = "all_interfaces";

/// 函数 `normalize_service_bind_mode`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - raw: 参数 raw
///
/// # 返回
/// 返回函数执行结果
fn normalize_service_bind_mode(raw: Option<&str>) -> &'static str {
    let Some(value) = raw else {
        return SERVICE_BIND_MODE_LOOPBACK;
    };
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "all_interfaces" | "all-interfaces" | "all" | "0.0.0.0" => SERVICE_BIND_MODE_ALL_INTERFACES,
        _ => SERVICE_BIND_MODE_LOOPBACK,
    }
}

/// 函数 `normalize_saved_service_addr`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - raw: 参数 raw
///
/// # 返回
/// 返回函数执行结果
fn normalize_saved_service_addr(raw: Option<&str>) -> Result<String, String> {
    let Some(value) = normalize_optional_text(raw) else {
        return Ok(DEFAULT_ADDR.to_string());
    };
    let value = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .unwrap_or(&value);
    let value = value.split('/').next().unwrap_or(value).trim();
    if value.is_empty() {
        return Err("service address is empty".to_string());
    }
    if value.contains(':') {
        return Ok(value.to_string());
    }
    Ok(format!("localhost:{value}"))
}

/// 函数 `current_env_service_addr`
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
fn current_env_service_addr() -> Option<String> {
    let raw = std::env::var("CODEXMANAGER_SERVICE_ADDR").ok()?;
    let normalized = normalize_saved_service_addr(Some(&raw)).ok()?;
    let Some((host, port)) = normalized.rsplit_once(':') else {
        return Some(normalized);
    };
    match host {
        "0.0.0.0" | "::" | "[::]" => Some(format!("localhost:{port}")),
        _ => Some(normalized),
    }
}

/// 函数 `current_env_service_bind_mode`
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
fn current_env_service_bind_mode() -> Option<String> {
    let raw = std::env::var("CODEXMANAGER_SERVICE_ADDR").ok()?;
    let normalized = normalize_saved_service_addr(Some(&raw)).ok()?;
    let host = normalized
        .rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(normalized.as_str());
    let mode = match host {
        "0.0.0.0" | "::" | "[::]" => SERVICE_BIND_MODE_ALL_INTERFACES,
        "localhost" | "127.0.0.1" | "::1" | "[::1]" => SERVICE_BIND_MODE_LOOPBACK,
        _ => return None,
    };
    Some(mode.to_string())
}

/// 函数 `current_persisted_service_bind_mode`
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
fn current_persisted_service_bind_mode() -> Option<String> {
    get_persisted_app_setting(SERVICE_BIND_MODE_SETTING_KEY)
        .map(|value| normalize_service_bind_mode(Some(&value)).to_string())
}

/// 函数 `current_effective_service_bind_mode`
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
fn current_effective_service_bind_mode() -> String {
    current_persisted_service_bind_mode()
        .or_else(current_env_service_bind_mode)
        .unwrap_or_else(|| SERVICE_BIND_MODE_LOOPBACK.to_string())
}

/// 函数 `current_service_bind_mode`
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
pub fn current_service_bind_mode() -> String {
    current_env_service_bind_mode()
        .or_else(|| {
            get_persisted_app_setting(SERVICE_BIND_MODE_SETTING_KEY)
                .map(|value| normalize_service_bind_mode(Some(&value)).to_string())
        })
        .unwrap_or_else(|| SERVICE_BIND_MODE_LOOPBACK.to_string())
}

/// 函数 `set_service_bind_mode`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - mode: 参数 mode
///
/// # 返回
/// 返回函数执行结果
pub fn set_service_bind_mode(mode: &str) -> Result<String, String> {
    let normalized = normalize_service_bind_mode(Some(mode)).to_string();
    save_persisted_app_setting(SERVICE_BIND_MODE_SETTING_KEY, Some(&normalized))?;
    let current_addr = current_saved_service_addr();
    let synced_addr = listener_bind_addr_for_mode(&current_addr, &normalized);
    save_persisted_app_setting(APP_SETTING_SERVICE_ADDR_KEY, Some(&synced_addr))?;
    std::env::set_var("CODEXMANAGER_SERVICE_ADDR", &synced_addr);
    Ok(normalized)
}

/// 函数 `bind_all_interfaces_enabled`
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
pub fn bind_all_interfaces_enabled() -> bool {
    current_effective_service_bind_mode() == SERVICE_BIND_MODE_ALL_INTERFACES
}

/// 函数 `bind_all_interfaces_enabled_for_mode`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - mode: 参数 mode
///
/// # 返回
/// 返回函数执行结果
pub fn bind_all_interfaces_enabled_for_mode(mode: &str) -> bool {
    normalize_service_bind_mode(Some(mode)) == SERVICE_BIND_MODE_ALL_INTERFACES
}

/// 函数 `default_listener_bind_addr`
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
pub fn default_listener_bind_addr() -> String {
    if bind_all_interfaces_enabled() {
        DEFAULT_BIND_ADDR.to_string()
    } else {
        DEFAULT_ADDR.to_string()
    }
}

/// 函数 `default_web_listener_addr`
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
pub fn default_web_listener_addr() -> String {
    let service_addr = current_saved_service_addr();
    let Some((host, port_text)) = service_addr.rsplit_once(':') else {
        return if bind_all_interfaces_enabled() {
            DEFAULT_WEB_BIND_ADDR.to_string()
        } else {
            DEFAULT_WEB_ADDR.to_string()
        };
    };
    let Ok(port) = port_text.parse::<u16>() else {
        return if bind_all_interfaces_enabled() {
            DEFAULT_WEB_BIND_ADDR.to_string()
        } else {
            DEFAULT_WEB_ADDR.to_string()
        };
    };
    let web_port = port.saturating_add(1);

    match host {
        "0.0.0.0" | "::" | "[::]" => format!("0.0.0.0:{web_port}"),
        "localhost" | "127.0.0.1" | "::1" | "[::1]" => {
            if bind_all_interfaces_enabled() {
                format!("0.0.0.0:{web_port}")
            } else {
                format!("localhost:{web_port}")
            }
        }
        _ => format!("{host}:{web_port}"),
    }
}

/// 函数 `listener_bind_addr`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
///
/// # 返回
/// 返回函数执行结果
pub fn listener_bind_addr(addr: &str) -> String {
    listener_bind_addr_for_mode(addr, &current_effective_service_bind_mode())
}

/// 函数 `listener_bind_addr_for_mode`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - bind_mode: 参数 bind_mode
///
/// # 返回
/// 返回函数执行结果
pub fn listener_bind_addr_for_mode(addr: &str, bind_mode: &str) -> String {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return if bind_all_interfaces_enabled_for_mode(bind_mode) {
            DEFAULT_BIND_ADDR.to_string()
        } else {
            DEFAULT_ADDR.to_string()
        };
    }

    let addr = trimmed.strip_prefix("http://").unwrap_or(trimmed);
    let addr = addr.strip_prefix("https://").unwrap_or(addr);
    let addr = addr.split('/').next().unwrap_or(addr);
    let bind_all = bind_all_interfaces_enabled_for_mode(bind_mode);

    if !addr.contains(':') {
        return if bind_all {
            format!("0.0.0.0:{addr}")
        } else {
            format!("localhost:{addr}")
        };
    }

    let Some((host, port)) = addr.rsplit_once(':') else {
        return addr.to_string();
    };
    if host == "0.0.0.0" {
        return format!("0.0.0.0:{port}");
    }
    if host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host == "[::1]"
    {
        return if bind_all {
            format!("0.0.0.0:{port}")
        } else {
            format!("localhost:{port}")
        };
    }

    addr.to_string()
}

/// 函数 `current_saved_service_addr`
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
pub fn current_saved_service_addr() -> String {
    current_env_service_addr()
        .or_else(|| {
            get_persisted_app_setting(APP_SETTING_SERVICE_ADDR_KEY)
                .and_then(|value| normalize_saved_service_addr(Some(&value)).ok())
        })
        .unwrap_or_else(|| DEFAULT_ADDR.to_string())
}

/// 函数 `set_saved_service_addr`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
///
/// # 返回
/// 返回函数执行结果
pub fn set_saved_service_addr(addr: Option<&str>) -> Result<String, String> {
    let normalized = normalize_saved_service_addr(addr)?;
    save_persisted_app_setting(APP_SETTING_SERVICE_ADDR_KEY, Some(&normalized))?;
    Ok(normalized)
}
