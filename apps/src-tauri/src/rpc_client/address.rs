use std::net::{SocketAddr, ToSocketAddrs};

/// 函数 `normalize_addr`
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
pub(crate) fn normalize_addr(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("addr is empty".to_string());
    }
    let mut value = trimmed;
    if let Some(rest) = value.strip_prefix("http://") {
        value = rest;
    }
    if let Some(rest) = value.strip_prefix("https://") {
        value = rest;
    }
    let value = value.split('/').next().unwrap_or(value);
    if value.is_empty() {
        return Err("addr is empty".to_string());
    }
    if value.contains(':') {
        Ok(normalize_host(value))
    } else if value.parse::<u16>().is_ok() {
        Ok(format!("localhost:{value}"))
    } else {
        Ok(normalize_host(value))
    }
}

/// 函数 `resolve_service_addr`
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
pub(crate) fn resolve_service_addr(addr: Option<String>) -> Result<String, String> {
    if let Some(addr) = addr {
        return normalize_addr(&addr);
    }
    if let Ok(env_addr) = std::env::var("CODEXMANAGER_SERVICE_ADDR") {
        if let Ok(addr) = normalize_addr(&env_addr) {
            return Ok(addr);
        }
    }
    Ok(codexmanager_service::DEFAULT_ADDR.to_string())
}

/// 函数 `resolve_socket_addrs`
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
pub(crate) fn resolve_socket_addrs(addr: &str) -> Result<Vec<SocketAddr>, String> {
    let addrs = addr
        .to_socket_addrs()
        .map_err(|err| format!("Invalid service address {addr}: {err}"))?;
    let mut out = Vec::new();
    for sock in addrs {
        if !out.iter().any(|item| item == &sock) {
            out.push(sock);
        }
    }
    if out.is_empty() {
        return Err(format!(
            "Invalid service address {addr}: no address resolved"
        ));
    }
    if addr
        .rsplit_once(':')
        .map(|(host, _)| host.eq_ignore_ascii_case("localhost"))
        .unwrap_or(false)
    {
        out.sort_by_key(|sock| if sock.is_ipv4() { 0 } else { 1 });
    }
    Ok(out)
}

/// 函数 `normalize_host`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
fn normalize_host(value: &str) -> String {
    if let Some((host, port)) = value.rsplit_once(':') {
        let mapped = match host {
            "127.0.0.1" | "0.0.0.0" | "::1" | "[::1]" => "localhost",
            _ => host,
        };
        format!("{mapped}:{port}")
    } else {
        value.to_string()
    }
}
