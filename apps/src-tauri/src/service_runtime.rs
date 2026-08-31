use std::sync::Mutex;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use crate::app_storage::apply_runtime_storage_env;
use crate::rpc_client::rpc_call;

const ENV_SERVICE_ADDR: &str = "CODEXMANAGER_SERVICE_ADDR";

/// 函数 `validate_initialize_response`
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
pub(super) fn validate_initialize_response(v: &serde_json::Value) -> Result<(), String> {
    // 连接探测必须确认对端确实是 CodexManager 服务，避免端口被其他服务占用时误判“已连接”。
    let result = v.get("result").and_then(|r| r.as_object());
    let user_agent = result
        .and_then(|r| r.get("userAgent").or_else(|| r.get("user_agent")))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let codex_home = result
        .and_then(|r| r.get("codexHome").or_else(|| r.get("codex_home")))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    if user_agent.contains("codex_cli_rs/") && !codex_home.is_empty() {
        return Ok(());
    }

    let hint = if user_agent.is_empty() {
        "missing userAgent"
    } else {
        user_agent
    };
    Err(format!(
        "Port is in use or unexpected service responded ({hint})"
    ))
}

/// 函数 `spawn_service_with_addr`
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
pub(super) fn spawn_service_with_addr(
    app: &tauri::AppHandle,
    bind_addr: &str,
    connect_addr: &str,
) -> Result<(), String> {
    if std::env::var("CODEXMANAGER_NO_SERVICE").is_ok() {
        return Ok(());
    }

    apply_runtime_storage_env(app);

    std::env::set_var(ENV_SERVICE_ADDR, bind_addr);
    codexmanager_service::clear_shutdown_flag();

    let bind_addr = bind_addr.to_string();
    let connect_addr = connect_addr.to_string();
    let thread_addr = bind_addr.clone();
    log::info!(
        "service starting at {} (local rpc {})",
        bind_addr,
        connect_addr
    );
    let handle = thread::spawn(move || {
        if let Err(err) = codexmanager_service::start_server(&thread_addr) {
            log::error!("service stopped: {}", err);
        }
    });
    set_service_runtime(ServiceRuntime {
        addr: connect_addr,
        join: handle,
    });
    Ok(())
}

struct ServiceRuntime {
    addr: String,
    join: thread::JoinHandle<()>,
}

static SERVICE_RUNTIME: OnceLock<Mutex<Option<ServiceRuntime>>> = OnceLock::new();

/// 函数 `set_service_runtime`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - runtime: 参数 runtime
///
/// # 返回
/// 无
fn set_service_runtime(runtime: ServiceRuntime) {
    let slot = SERVICE_RUNTIME.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(runtime);
    }
}

/// 函数 `take_service_runtime`
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
fn take_service_runtime() -> Option<ServiceRuntime> {
    let slot = SERVICE_RUNTIME.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        guard.take()
    } else {
        None
    }
}

/// 函数 `stop_service`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 无
pub(super) fn stop_service() {
    if let Some(runtime) = take_service_runtime() {
        log::info!("service stopping at {}", runtime.addr);
        codexmanager_service::request_shutdown(&runtime.addr);
        thread::spawn(move || {
            let _ = runtime.join.join();
        });
    }
}

/// 函数 `wait_for_service_ready`
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
pub(super) fn wait_for_service_ready(
    addr: &str,
    retries: usize,
    delay: Duration,
) -> Result<(), String> {
    let mut last_err = "service bootstrap check failed".to_string();
    for attempt in 0..=retries {
        match rpc_call("initialize", Some(addr.to_string()), None) {
            Ok(v) => match validate_initialize_response(&v) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            },
            Err(err) => {
                last_err = err;
            }
        }
        if attempt < retries {
            std::thread::sleep(delay);
        }
    }
    Err(last_err)
}
