use codexmanager_core::rpc::types::JsonRpcRequest;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use super::address::{resolve_service_addr, resolve_socket_addrs};
use super::http::parse_http_body;

const RPC_CONNECT_TIMEOUT: Duration = Duration::from_millis(400);
const RPC_DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(10);
const RPC_BULK_USAGE_REFRESH_IO_TIMEOUT: Duration = Duration::from_secs(600);

/// 函数 `rpc_io_timeout`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - method: 参数 method
/// - params: 参数 params
///
/// # 返回
/// 返回函数执行结果
fn rpc_io_timeout(method: &str, params: Option<&serde_json::Value>) -> Duration {
    if method == "account/usage/refresh"
        && params
            .and_then(|value| value.get("accountId"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return RPC_BULK_USAGE_REFRESH_IO_TIMEOUT;
    }

    RPC_DEFAULT_IO_TIMEOUT
}

/// 函数 `rpc_call_on_socket`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - method: 参数 method
/// - addr: 参数 addr
/// - sock: 参数 sock
/// - params: 参数 params
///
/// # 返回
/// 返回函数执行结果
fn rpc_call_on_socket(
    method: &str,
    addr: &str,
    sock: SocketAddr,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let io_timeout = rpc_io_timeout(method, params.as_ref());
    let mut stream = TcpStream::connect_timeout(&sock, RPC_CONNECT_TIMEOUT).map_err(|e| {
        let msg = format!("Failed to connect to service at {addr}: {e}");
        log::warn!(
            "rpc connect failed ({} -> {} via {}): {}",
            method,
            addr,
            sock,
            e
        );
        msg
    })?;
    let _ = stream.set_read_timeout(Some(io_timeout));
    let _ = stream.set_write_timeout(Some(io_timeout));

    let req = JsonRpcRequest {
        id: 1.into(),
        method: method.to_string(),
        params,
        trace: None,
    };
    let json = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    let rpc_token = codexmanager_service::rpc_auth_token();
    let http = format!(
        "POST /rpc HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nX-CodexManager-Rpc-Token: {rpc_token}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        json.len(),
        json
    );
    stream.write_all(http.as_bytes()).map_err(|e| {
        let msg = e.to_string();
        log::warn!(
            "rpc write failed ({} -> {} via {}): {}",
            method,
            addr,
            sock,
            msg
        );
        msg
    })?;

    let mut buf = String::new();
    stream.read_to_string(&mut buf).map_err(|e| {
        let msg = e.to_string();
        log::warn!(
            "rpc read failed ({} -> {} via {}): {}",
            method,
            addr,
            sock,
            msg
        );
        msg
    })?;
    let body = parse_http_body(&buf).map_err(|msg| {
        log::warn!(
            "rpc parse failed ({} -> {} via {}): {}",
            method,
            addr,
            sock,
            msg
        );
        msg
    })?;
    if body.trim().is_empty() {
        log::warn!("rpc empty response ({} -> {} via {})", method, addr, sock);
        return Err(
            "Empty response from service (service not ready, exited, or port occupied)".to_string(),
        );
    }

    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        let msg = format!("Unexpected RPC response (non-JSON body): {e}");
        log::warn!(
            "rpc json parse failed ({} -> {} via {}): {}",
            method,
            addr,
            sock,
            msg
        );
        msg
    })?;
    if let Some(err) = v.get("error") {
        log::warn!("rpc error ({} -> {} via {}): {}", method, addr, sock, err);
    }
    Ok(v)
}

/// 函数 `rpc_call_with_sockets`
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
pub(crate) fn rpc_call_with_sockets(
    method: &str,
    addr: &str,
    socket_addrs: &[SocketAddr],
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if socket_addrs.is_empty() {
        return Err(format!(
            "Invalid service address {addr}: no address resolved"
        ));
    }
    let mut last_err =
        "Empty response from service (service not ready, exited, or port occupied)".to_string();
    for attempt in 0..=1 {
        for sock in socket_addrs {
            match rpc_call_on_socket(method, addr, *sock, params.clone()) {
                Ok(v) => return Ok(v),
                Err(err) => last_err = err,
            }
        }
        if attempt == 0 {
            std::thread::sleep(Duration::from_millis(120));
        }
    }
    Err(last_err)
}

/// 函数 `rpc_call`
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
pub(crate) fn rpc_call(
    method: &str,
    addr: Option<String>,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let addr = resolve_service_addr(addr)?;
    let socket_addrs = resolve_socket_addrs(&addr)?;
    rpc_call_with_sockets(method, &addr, &socket_addrs, params)
}

#[cfg(test)]
mod tests {
    use super::{rpc_io_timeout, RPC_BULK_USAGE_REFRESH_IO_TIMEOUT, RPC_DEFAULT_IO_TIMEOUT};

    /// 函数 `bulk_usage_refresh_uses_extended_timeout`
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
    #[test]
    fn bulk_usage_refresh_uses_extended_timeout() {
        let timeout = rpc_io_timeout("account/usage/refresh", None);
        assert_eq!(timeout, RPC_BULK_USAGE_REFRESH_IO_TIMEOUT);
    }

    /// 函数 `single_usage_refresh_keeps_default_timeout`
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
    #[test]
    fn single_usage_refresh_keeps_default_timeout() {
        let timeout = rpc_io_timeout(
            "account/usage/refresh",
            Some(&serde_json::json!({ "accountId": "acc-1" })),
        );
        assert_eq!(timeout, RPC_DEFAULT_IO_TIMEOUT);
    }

    /// 函数 `unrelated_rpc_keeps_default_timeout`
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
    #[test]
    fn unrelated_rpc_keeps_default_timeout() {
        let timeout = rpc_io_timeout("account/list", None);
        assert_eq!(timeout, RPC_DEFAULT_IO_TIMEOUT);
    }
}
