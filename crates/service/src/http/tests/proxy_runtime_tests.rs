use super::{
    build_backend_base_url, build_local_backend_client, front_proxy_max_blocking_threads,
    front_proxy_worker_threads, proxy_handler, ProxyState,
};
use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Request as HttpRequest, StatusCode};
use codexmanager_core::storage::{Account, ApiKey, Storage, Token, UsageSnapshotRecord};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

impl EnvGuard {
    /// 函数 `set`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - key: 参数 key
    /// - value: 参数 value
    ///
    /// # 返回
    /// 返回函数执行结果
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, original }
    }

    fn clear(key: &'static str) -> Self {
        let original = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    /// 函数 `drop`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    ///
    /// # 返回
    /// 无
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

/// 函数 `backend_base_url_uses_http_scheme`
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
fn backend_base_url_uses_http_scheme() {
    assert_eq!(
        build_backend_base_url("127.0.0.1:18080"),
        "http://127.0.0.1:18080"
    );
}

/// 函数 `local_backend_client_builds_without_system_proxy`
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
fn local_backend_client_builds_without_system_proxy() {
    build_local_backend_client().expect("local backend client");
}

#[test]
fn front_proxy_blocking_threads_follow_storage_pool_default() {
    let _guard = crate::test_env_guard();
    let _front_guard = EnvGuard::clear("CODEXMANAGER_FRONT_PROXY_MAX_BLOCKING_THREADS");
    let _storage_guard = EnvGuard::set("CODEXMANAGER_STORAGE_MAX_CONNECTIONS", "7");

    assert_eq!(front_proxy_max_blocking_threads(), 7);
}

#[test]
fn front_proxy_blocking_threads_allow_explicit_override() {
    let _guard = crate::test_env_guard();
    let _storage_guard = EnvGuard::set("CODEXMANAGER_STORAGE_MAX_CONNECTIONS", "7");
    let _front_guard = EnvGuard::set("CODEXMANAGER_FRONT_PROXY_MAX_BLOCKING_THREADS", "5");

    assert_eq!(front_proxy_max_blocking_threads(), 5);
}

#[test]
fn front_proxy_worker_threads_default_to_small_runtime() {
    let _guard = crate::test_env_guard();
    let _worker_guard = EnvGuard::clear("CODEXMANAGER_FRONT_PROXY_WORKER_THREADS");

    assert_eq!(front_proxy_worker_threads(), 2);
}

#[test]
fn front_proxy_worker_threads_allow_explicit_override() {
    let _guard = crate::test_env_guard();
    let _worker_guard = EnvGuard::set("CODEXMANAGER_FRONT_PROXY_WORKER_THREADS", "3");

    assert_eq!(front_proxy_worker_threads(), 3);
}

/// 函数 `request_without_content_length_over_limit_returns_413`
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
fn request_without_content_length_over_limit_returns_413() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set("CODEXMANAGER_FRONT_PROXY_MAX_BODY_BYTES", "8");
    crate::gateway::reload_runtime_config_from_env();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let state = ProxyState {
        backend_base_url: "http://127.0.0.1:1".to_string(),
        client: build_local_backend_client().expect("client"),
    };
    let request = HttpRequest::builder()
        .method("POST")
        .uri("/rpc")
        .body(Body::from(vec![b'x'; 9]))
        .expect("request");

    let response = runtime.block_on(proxy_handler(State(state), request));
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body = runtime
        .block_on(to_bytes(response.into_body(), usize::MAX))
        .expect("read body");
    let text = String::from_utf8(body.to_vec()).expect("utf8");
    assert_eq!(text, "request body too large: content-length>8");
}

#[test]
fn zero_front_proxy_limit_disables_body_rejection() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set("CODEXMANAGER_FRONT_PROXY_MAX_BODY_BYTES", "0");
    crate::gateway::reload_runtime_config_from_env();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let state = ProxyState {
        backend_base_url: "http://127.0.0.1:1".to_string(),
        client: Client::new(),
    };
    let request = HttpRequest::builder()
        .method("POST")
        .uri("/rpc")
        .body(Body::from(vec![b'x'; 64]))
        .expect("request");

    let response = runtime.block_on(proxy_handler(State(state), request));
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

/// 函数 `backend_send_failure_returns_502`
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
fn backend_send_failure_returns_502() {
    let _ = crate::gateway::front_proxy_max_body_bytes();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let state = ProxyState {
        backend_base_url: "http://127.0.0.1:1".to_string(),
        client: Client::new(),
    };
    let request = HttpRequest::builder()
        .method("GET")
        .uri("/backend-proxy-health")
        .body(Body::empty())
        .expect("request");

    let response = runtime.block_on(proxy_handler(State(state), request));
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let error_code = response
        .headers()
        .get(crate::error_codes::ERROR_CODE_HEADER_NAME)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = runtime
        .block_on(to_bytes(response.into_body(), usize::MAX))
        .expect("read body");
    let text = String::from_utf8(body.to_vec()).expect("utf8");
    let _ = error_code;
    let _ = text;
}

fn new_test_db_path(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unix ts")
        .as_nanos();
    let counter = TEST_DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "{prefix}-{}-{counter}-{nonce}.db",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
    path
}

fn init_test_storage(db_path: &PathBuf) -> Storage {
    let storage = Storage::open(db_path).expect("open storage");
    storage.init().expect("init storage");
    storage
}

fn insert_api_key_record(
    storage: &Storage,
    platform_key: &str,
    rotation_strategy: &str,
    upstream_base_url: Option<String>,
) {
    let now = chrono::Utc::now().timestamp();
    storage
        .insert_api_key(&ApiKey {
            id: "gk_proxy_runtime_ws".to_string(),
            name: Some("proxy-runtime-ws".to_string()),
            model_slug: Some("gpt-5.4-mini".to_string()),
            reasoning_effort: Some("high".to_string()),
            service_tier: Some("fast".to_string()),
            rotation_strategy: rotation_strategy.to_string(),
            aggregate_api_id: None,
            aggregate_api_url: None,
            account_plan_filter: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url,
            static_headers_json: None,
            key_hash: crate::storage_helpers::hash_platform_key(platform_key),
            status: "active".to_string(),
            created_at: now,
            last_used_at: None,
        })
        .expect("insert api key");
}

fn insert_account_and_token(storage: &Storage) {
    let now = chrono::Utc::now().timestamp();
    storage
        .insert_account(&Account {
            id: "acc_proxy_runtime_ws".to_string(),
            label: "proxy-runtime-ws".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("chatgpt_proxy_runtime_ws".to_string()),
            workspace_id: None,
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&Token {
            account_id: "acc_proxy_runtime_ws".to_string(),
            id_token: "id_token_ws".to_string(),
            access_token: "access_token_ws".to_string(),
            refresh_token: "refresh_token_ws".to_string(),
            api_key_access_token: Some("access_token_ws".to_string()),
            last_refresh: now,
        })
        .expect("insert token");
    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc_proxy_runtime_ws".to_string(),
            used_percent: Some(8.0),
            window_minutes: Some(180),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert usage snapshot");
}

fn insert_account_and_token_with_id(
    storage: &Storage,
    account_id: &str,
    label: &str,
    chatgpt_account_id: &str,
    access_token: &str,
    sort: i64,
) {
    let now = chrono::Utc::now().timestamp();
    storage
        .insert_account(&Account {
            id: account_id.to_string(),
            label: label.to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some(chatgpt_account_id.to_string()),
            workspace_id: None,
            group_name: None,
            sort,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&Token {
            account_id: account_id.to_string(),
            id_token: format!("id_token_{account_id}"),
            access_token: access_token.to_string(),
            refresh_token: format!("refresh_token_{account_id}"),
            api_key_access_token: Some(access_token.to_string()),
            last_refresh: now,
        })
        .expect("insert token");
    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: account_id.to_string(),
            used_percent: Some(8.0),
            window_minutes: Some(180),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert usage snapshot");
}

async fn start_front_proxy_test_server(
    state: ProxyState,
) -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let app = super::build_front_proxy_app(state);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        server.await.expect("serve front proxy");
    });
    (addr.to_string(), shutdown_tx, handle)
}

#[derive(Debug)]
struct UpstreamWsCapture {
    path: String,
    headers: HashMap<String, String>,
    frames: Vec<String>,
}

async fn start_mock_upstream_ws() -> (
    String,
    tokio::sync::mpsc::UnboundedReceiver<String>,
    oneshot::Receiver<UpstreamWsCapture>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let addr = listener.local_addr().expect("mock upstream addr");
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    let (capture_tx, capture_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept mock upstream");
        let captured_headers = std::sync::Arc::new(std::sync::Mutex::new(
            None::<(String, HashMap<String, String>)>,
        ));
        let captured_headers_clone = captured_headers.clone();
        let mut websocket =
            accept_hdr_async(stream, move |request: &Request, response: Response| {
                let mut headers = HashMap::new();
                for (name, value) in request.headers() {
                    if let Ok(text) = value.to_str() {
                        headers.insert(name.as_str().to_ascii_lowercase(), text.to_string());
                    }
                }
                let mut guard = captured_headers_clone
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *guard = Some((request.uri().path().to_string(), headers));
                Ok(response)
            })
            .await
            .expect("accept websocket handshake");

        let mut frames = Vec::new();
        if let Some(Ok(Message::Text(text))) = websocket.next().await {
            frames.push(text.to_string());
            let _ = event_tx.send(text.to_string());
            websocket
                .send(Message::Text(
                    "{\"type\":\"response.created\",\"response\":{\"id\":\"resp_ws_1\"}}"
                        .to_string()
                        .into(),
                ))
                .await
                .expect("send response.created");
        }
        if let Some(Ok(Message::Text(text))) = websocket.next().await {
            frames.push(text.to_string());
            let _ = event_tx.send(text.to_string());
            websocket
                .send(Message::Text(
                    "{\"type\":\"response.completed\",\"response\":{\"id\":\"resp_ws_2\"}}"
                        .to_string()
                        .into(),
                ))
                .await
                .expect("send response.completed");
        }
        let (path, headers) = captured_headers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("captured handshake");
        let _ = capture_tx.send(UpstreamWsCapture {
            path,
            headers,
            frames,
        });
    });
    (addr.to_string(), event_rx, capture_rx, handle)
}

async fn start_mock_upstream_ws_fail_then_success() -> (
    String,
    tokio::sync::mpsc::UnboundedReceiver<String>,
    oneshot::Receiver<Vec<UpstreamWsCapture>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let addr = listener.local_addr().expect("mock upstream addr");
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    let (capture_tx, capture_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let mut captures = Vec::new();

        for round in 0..2 {
            let (stream, _) = listener.accept().await.expect("accept mock upstream");
            let captured_headers = std::sync::Arc::new(std::sync::Mutex::new(
                None::<(String, HashMap<String, String>)>,
            ));
            let captured_headers_clone = captured_headers.clone();
            let mut websocket =
                accept_hdr_async(stream, move |request: &Request, response: Response| {
                    let mut headers = HashMap::new();
                    for (name, value) in request.headers() {
                        if let Ok(text) = value.to_str() {
                            headers.insert(name.as_str().to_ascii_lowercase(), text.to_string());
                        }
                    }
                    let mut guard = captured_headers_clone
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    *guard = Some((request.uri().path().to_string(), headers));
                    Ok(response)
                })
                .await
                .expect("accept websocket handshake");

            let mut frames = Vec::new();
            if let Some(Ok(Message::Text(text))) = websocket.next().await {
                frames.push(text.to_string());
                let _ = event_tx.send(text.to_string());
                let response_payload = if round == 0 {
                    serde_json::json!({
                        "type": "response.failed",
                        "status": 429,
                        "error": {
                            "message": "rate limited on first account"
                        }
                    })
                } else {
                    serde_json::json!({
                        "type": "response.completed",
                        "response": { "id": "resp_ws_failover_ok" }
                    })
                };
                websocket
                    .send(Message::Text(response_payload.to_string().into()))
                    .await
                    .expect("send upstream response");
            }
            let (path, headers) = captured_headers
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .take()
                .expect("captured handshake");
            captures.push(UpstreamWsCapture {
                path,
                headers,
                frames,
            });
        }

        let _ = capture_tx.send(captures);
    });
    (addr.to_string(), event_rx, capture_rx, handle)
}

fn build_ws_request(
    url: &str,
    platform_key: &str,
    extra_headers: &[(&str, &str)],
) -> tokio_tungstenite::tungstenite::handshake::client::Request {
    let mut request = url.into_client_request().expect("build ws request");
    request.headers_mut().insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_str(&format!("Bearer {platform_key}"))
            .expect("authorization header"),
    );
    for (name, value) in extra_headers {
        request.headers_mut().insert(
            axum::http::header::HeaderName::from_bytes(name.as_bytes()).expect("header name"),
            axum::http::HeaderValue::from_str(value).expect("header value"),
        );
    }
    request
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_responses_websocket_returns_426() {
    let _guard = crate::test_env_guard();
    let db_path = new_test_db_path("codexmanager-proxy-runtime-ws-unsupported");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = init_test_storage(&db_path);
    insert_api_key_record(
        &storage,
        "platform_key_ws_unsupported",
        crate::apikey_profile::ROTATION_AGGREGATE_API,
        None,
    );
    tokio::task::spawn_blocking(|| {
        crate::gateway::reload_runtime_config_from_env();
        let _ = crate::gateway::front_proxy_max_body_bytes();
    })
    .await
    .expect("reload runtime config");

    let state = ProxyState {
        backend_base_url: "http://127.0.0.1:1".to_string(),
        client: Client::new(),
    };
    let (front_addr, shutdown_tx, server_handle) = start_front_proxy_test_server(state).await;
    let request = build_ws_request(
        &format!("ws://{front_addr}/v1/responses"),
        "platform_key_ws_unsupported",
        &[("OpenAI-Beta", "responses_websockets=2026-02-06")],
    );

    let err = connect_async(request)
        .await
        .expect_err("websocket should fail");
    match err {
        tokio_tungstenite::tungstenite::Error::Http(response) => {
            assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
        }
        other => panic!("unexpected websocket error: {other}"),
    }

    let _ = shutdown_tx.send(());
    server_handle.await.expect("join front proxy");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hybrid_responses_websocket_returns_426() {
    let _guard = crate::test_env_guard();
    let db_path = new_test_db_path("codexmanager-proxy-runtime-ws-hybrid-unsupported");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = init_test_storage(&db_path);
    insert_api_key_record(
        &storage,
        "platform_key_ws_hybrid_unsupported",
        crate::apikey_profile::ROTATION_HYBRID,
        None,
    );
    tokio::task::spawn_blocking(|| {
        crate::gateway::reload_runtime_config_from_env();
        let _ = crate::gateway::front_proxy_max_body_bytes();
    })
    .await
    .expect("reload runtime config");

    let state = ProxyState {
        backend_base_url: "http://127.0.0.1:1".to_string(),
        client: Client::new(),
    };
    let (front_addr, shutdown_tx, server_handle) = start_front_proxy_test_server(state).await;
    let request = build_ws_request(
        &format!("ws://{front_addr}/v1/responses"),
        "platform_key_ws_hybrid_unsupported",
        &[("OpenAI-Beta", "responses_websockets=2026-02-06")],
    );

    let err = connect_async(request)
        .await
        .expect_err("websocket should fail");
    match err {
        tokio_tungstenite::tungstenite::Error::Http(response) => {
            assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
        }
        other => panic!("unexpected websocket error: {other}"),
    }

    let _ = shutdown_tx.send(());
    server_handle.await.expect("join front proxy");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn official_responses_websocket_proxies_frames_and_headers() {
    let _guard = crate::test_env_guard();
    let _org_guard = EnvGuard::set("OPENAI_ORGANIZATION", "org_ws_test");
    let _project_guard = EnvGuard::set("OPENAI_PROJECT", "proj_ws_test");
    let db_path = new_test_db_path("codexmanager-proxy-runtime-ws-supported");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = init_test_storage(&db_path);
    let (upstream_addr, mut upstream_events, capture_rx, upstream_handle) =
        start_mock_upstream_ws().await;
    insert_api_key_record(
        &storage,
        "platform_key_ws_supported",
        crate::apikey_profile::ROTATION_ACCOUNT,
        Some(format!(
            "http://{upstream_addr}/chatgpt.com/backend-api/codex"
        )),
    );
    insert_account_and_token(&storage);
    tokio::task::spawn_blocking(|| {
        crate::gateway::reload_runtime_config_from_env();
        let _ = crate::gateway::front_proxy_max_body_bytes();
    })
    .await
    .expect("reload runtime config");

    let state = ProxyState {
        backend_base_url: "http://127.0.0.1:1".to_string(),
        client: Client::new(),
    };
    let (front_addr, shutdown_tx, server_handle) = start_front_proxy_test_server(state).await;
    let request = build_ws_request(
        &format!("ws://{front_addr}/v1/responses"),
        "platform_key_ws_supported",
        &[
            ("OpenAI-Beta", "responses_websockets=2026-02-06"),
            ("session_id", "session_ws_1"),
            ("x-codex-window-id", "session_ws_1:7"),
            ("x-client-request-id", "client_req_ws_1"),
            ("x-openai-subagent", "review"),
            ("x-codex-parent-thread-id", "thread_parent_ws_1"),
            ("x-codex-other-limit-name", "promo_header_ws"),
            ("x-codex-turn-state", "turn_state_ws_1"),
            ("x-codex-turn-metadata", "turn_meta_ws_1"),
            ("x-responsesapi-include-timing-metrics", "true"),
        ],
    );
    let (mut client_ws, response) = connect_async(request).await.expect("websocket connects");
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    client_ws
        .send(Message::Text(
            serde_json::json!({
                "type": "response.create",
                "model": "gpt-4.1",
                "input": "hello",
                "stream": false,
                "store": true,
                "service_tier": "Fast",
                "generate": false
            })
            .to_string()
            .into(),
        ))
        .await
        .expect("send first frame");

    let first_upstream_frame = tokio::time::timeout(Duration::from_secs(5), upstream_events.recv())
        .await
        .expect("first upstream frame timeout")
        .expect("first upstream frame channel");
    let first_payload: serde_json::Value =
        serde_json::from_str(&first_upstream_frame).expect("parse first upstream frame");
    assert_eq!(first_payload["type"], "response.create");
    assert_eq!(first_payload["model"], "gpt-5.4-mini");
    assert_eq!(first_payload["stream"], false);
    assert_eq!(first_payload["store"], true);
    assert_eq!(first_payload["service_tier"], "priority");
    assert_eq!(first_payload["generate"], false);
    assert!(first_payload.get("prompt_cache_key").is_none());

    let first_client_event = tokio::time::timeout(Duration::from_secs(5), client_ws.next())
        .await
        .expect("first client event timeout")
        .expect("first client event");
    let first_client_event = first_client_event.expect("first client event result");
    match first_client_event {
        Message::Text(text) => {
            assert!(
                text.contains("\"response.created\""),
                "unexpected event: {text}"
            );
        }
        other => panic!("unexpected first client event: {other:?}"),
    }

    client_ws
        .send(Message::Text(
            serde_json::json!({
                "type": "response.create",
                "previous_response_id": "resp_prev_ws_1",
                "input": "follow up",
                "client_metadata": {
                    "source": "proxy-runtime-test"
                }
            })
            .to_string()
            .into(),
        ))
        .await
        .expect("send second frame");

    let second_upstream_frame =
        tokio::time::timeout(Duration::from_secs(5), upstream_events.recv())
            .await
            .expect("second upstream frame timeout")
            .expect("second upstream frame channel");
    let second_payload: serde_json::Value =
        serde_json::from_str(&second_upstream_frame).expect("parse second upstream frame");
    assert_eq!(second_payload["type"], "response.create");
    assert_eq!(second_payload["previous_response_id"], "resp_prev_ws_1");
    assert_eq!(
        second_payload["client_metadata"]["source"],
        "proxy-runtime-test"
    );
    assert_eq!(
        second_payload["client_metadata"]["x-codex-turn-metadata"],
        "turn_meta_ws_1"
    );
    assert!(second_payload.get("prompt_cache_key").is_none());

    let second_client_event = tokio::time::timeout(Duration::from_secs(5), client_ws.next())
        .await
        .expect("second client event timeout")
        .expect("second client event");
    let second_client_event = second_client_event.expect("second client event result");
    match second_client_event {
        Message::Text(text) => {
            assert!(
                text.contains("\"response.completed\""),
                "unexpected event: {text}"
            );
        }
        other => panic!("unexpected second client event: {other:?}"),
    }

    let capture = tokio::time::timeout(Duration::from_secs(2), capture_rx)
        .await
        .expect("capture timeout")
        .expect("capture result");
    assert_eq!(capture.path, "/chatgpt.com/backend-api/codex/responses");
    assert_eq!(
        capture.headers.get("authorization").map(String::as_str),
        Some("Bearer access_token_ws")
    );
    assert_eq!(
        capture
            .headers
            .get("chatgpt-account-id")
            .map(String::as_str),
        Some("chatgpt_proxy_runtime_ws")
    );
    assert_eq!(
        capture.headers.get("openai-beta").map(String::as_str),
        Some("responses_websockets=2026-02-06")
    );
    assert_eq!(capture.headers.get("version").map(String::as_str), None);
    assert_eq!(
        capture
            .headers
            .get("openai-organization")
            .map(String::as_str),
        None
    );
    assert_eq!(
        capture.headers.get("openai-project").map(String::as_str),
        None
    );
    assert_eq!(
        capture.headers.get("session_id").map(String::as_str),
        Some("session_ws_1")
    );
    assert_eq!(
        capture.headers.get("x-codex-window-id").map(String::as_str),
        Some("session_ws_1:7")
    );
    assert_eq!(
        capture
            .headers
            .get("x-client-request-id")
            .map(String::as_str),
        Some("client_req_ws_1")
    );
    assert_eq!(
        capture.headers.get("x-openai-subagent").map(String::as_str),
        Some("review")
    );
    assert_eq!(
        capture
            .headers
            .get("x-codex-parent-thread-id")
            .map(String::as_str),
        Some("thread_parent_ws_1")
    );
    assert_eq!(
        capture
            .headers
            .get("x-codex-other-limit-name")
            .map(String::as_str),
        None
    );
    assert_eq!(
        capture
            .headers
            .get("x-codex-turn-state")
            .map(String::as_str),
        Some("turn_state_ws_1")
    );
    assert_eq!(
        capture
            .headers
            .get("x-codex-turn-metadata")
            .map(String::as_str),
        Some("turn_meta_ws_1")
    );
    assert_eq!(
        capture
            .headers
            .get("x-responsesapi-include-timing-metrics")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(capture.frames.len(), 2);

    let request_logs = storage
        .list_request_logs(None, 10)
        .expect("list request logs");
    let ws_logs: Vec<_> = request_logs
        .iter()
        .filter(|item| item.request_type.as_deref() == Some("ws"))
        .collect();
    assert_eq!(
        ws_logs.len(),
        2,
        "expected two websocket request log entries"
    );
    assert!(
        ws_logs
            .iter()
            .any(|item| item.service_tier.as_deref() == Some("fast")),
        "expected websocket request log to keep explicit fast service tier"
    );
    assert!(
        ws_logs
            .iter()
            .any(|item| item.effective_service_tier.as_deref() == Some("fast")),
        "expected websocket request log to persist effective fast service tier"
    );
    assert!(
        ws_logs.iter().any(|item| item.service_tier.is_none()),
        "expected follow-up websocket request without explicit service tier to stay empty"
    );
    assert!(
        ws_logs
            .iter()
            .filter(|item| item.service_tier.is_none())
            .any(|item| item.effective_service_tier.as_deref() == Some("fast")),
        "expected follow-up websocket request to keep effective fast service tier"
    );

    client_ws.close(None).await.expect("close client websocket");
    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("front proxy shutdown timeout")
        .expect("join front proxy");
    tokio::time::timeout(Duration::from_secs(5), upstream_handle)
        .await
        .expect("mock upstream shutdown timeout")
        .expect("join mock upstream");
}

#[tokio::test]
async fn official_responses_websocket_preserves_explicit_prompt_cache_key_when_session_anchor_exists(
) {
    let _guard = crate::test_env_guard();
    let db_path = new_test_db_path("codexmanager-proxy-runtime-ws-explicit-prompt-cache-key");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = init_test_storage(&db_path);
    let (upstream_addr, mut upstream_events, capture_rx, upstream_handle) =
        start_mock_upstream_ws().await;
    insert_api_key_record(
        &storage,
        "platform_key_ws_explicit_prompt_cache_key",
        crate::apikey_profile::ROTATION_ACCOUNT,
        Some(format!(
            "http://{upstream_addr}/chatgpt.com/backend-api/codex"
        )),
    );
    insert_account_and_token(&storage);
    tokio::task::spawn_blocking(|| {
        crate::gateway::reload_runtime_config_from_env();
        let _ = crate::gateway::front_proxy_max_body_bytes();
    })
    .await
    .expect("reload runtime config");

    let state = ProxyState {
        backend_base_url: "http://127.0.0.1:1".to_string(),
        client: Client::new(),
    };
    let (front_addr, shutdown_tx, server_handle) = start_front_proxy_test_server(state).await;
    let request = build_ws_request(
        &format!("ws://{front_addr}/v1/responses"),
        "platform_key_ws_explicit_prompt_cache_key",
        &[
            ("OpenAI-Beta", "responses_websockets=2026-02-06"),
            ("session_id", "session_ws_explicit_prompt_cache_key"),
            (
                "x-client-request-id",
                "client_req_ws_explicit_prompt_cache_key",
            ),
        ],
    );
    let (mut client_ws, response) = connect_async(request).await.expect("websocket connects");
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    client_ws
        .send(Message::Text(
            serde_json::json!({
                "type": "response.create",
                "model": "gpt-4.1",
                "input": "hello",
                "prompt_cache_key": "client_ws_thread_123"
            })
            .to_string()
            .into(),
        ))
        .await
        .expect("send frame");

    let upstream_frame = tokio::time::timeout(Duration::from_secs(5), upstream_events.recv())
        .await
        .expect("upstream frame timeout")
        .expect("upstream frame channel");
    let payload: serde_json::Value =
        serde_json::from_str(&upstream_frame).expect("parse upstream frame");
    assert_eq!(payload["type"], "response.create");
    assert_eq!(
        payload
            .get("prompt_cache_key")
            .and_then(serde_json::Value::as_str),
        Some("client_ws_thread_123")
    );

    let client_event = tokio::time::timeout(Duration::from_secs(5), client_ws.next())
        .await
        .expect("client event timeout")
        .expect("client event")
        .expect("client event result");
    match client_event {
        Message::Text(text) => {
            assert!(
                text.contains("\"response.created\""),
                "unexpected event: {text}"
            );
        }
        other => panic!("unexpected client event: {other:?}"),
    }

    let _ = client_ws.close(None).await;
    shutdown_tx.send(()).ok();
    server_handle.await.expect("front proxy join");
    upstream_handle.abort();
    let _ = capture_rx.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn official_responses_websocket_retries_current_request_after_terminal_failure() {
    let _guard = crate::test_env_guard();
    let db_path = new_test_db_path("codexmanager-proxy-runtime-ws-failover");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = init_test_storage(&db_path);
    let (upstream_addr, mut upstream_events, capture_rx, upstream_handle) =
        start_mock_upstream_ws_fail_then_success().await;
    insert_api_key_record(
        &storage,
        "platform_key_ws_failover",
        crate::apikey_profile::ROTATION_ACCOUNT,
        Some(format!(
            "http://{upstream_addr}/chatgpt.com/backend-api/codex"
        )),
    );
    insert_account_and_token_with_id(
        &storage,
        "acc_proxy_runtime_ws_a",
        "proxy-runtime-ws-a",
        "chatgpt_proxy_runtime_ws_a",
        "access_token_ws_a",
        0,
    );
    insert_account_and_token_with_id(
        &storage,
        "acc_proxy_runtime_ws_b",
        "proxy-runtime-ws-b",
        "chatgpt_proxy_runtime_ws_b",
        "access_token_ws_b",
        1,
    );
    tokio::task::spawn_blocking(|| {
        crate::gateway::reload_runtime_config_from_env();
        let _ = crate::gateway::front_proxy_max_body_bytes();
    })
    .await
    .expect("reload runtime config");

    let state = ProxyState {
        backend_base_url: "http://127.0.0.1:1".to_string(),
        client: Client::new(),
    };
    let (front_addr, shutdown_tx, server_handle) = start_front_proxy_test_server(state).await;
    let request = build_ws_request(
        &format!("ws://{front_addr}/v1/responses"),
        "platform_key_ws_failover",
        &[("OpenAI-Beta", "responses_websockets=2026-02-06")],
    );
    let (mut client_ws, response) = connect_async(request).await.expect("websocket connects");
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    client_ws
        .send(Message::Text(
            serde_json::json!({
                "type": "response.create",
                "model": "gpt-4.1",
                "previous_response_id": "resp_ws_old_account",
                "input": "first request"
            })
            .to_string()
            .into(),
        ))
        .await
        .expect("send first frame");

    let first_upstream_frame = tokio::time::timeout(Duration::from_secs(5), upstream_events.recv())
        .await
        .expect("first upstream frame timeout")
        .expect("first upstream frame channel");
    let first_payload: serde_json::Value =
        serde_json::from_str(&first_upstream_frame).expect("parse first upstream frame");
    assert_eq!(first_payload["type"], "response.create");
    assert_eq!(first_payload["previous_response_id"], "resp_ws_old_account");

    let second_upstream_frame =
        tokio::time::timeout(Duration::from_secs(5), upstream_events.recv())
            .await
            .expect("retry upstream frame timeout")
            .expect("retry upstream frame channel");
    let second_payload: serde_json::Value =
        serde_json::from_str(&second_upstream_frame).expect("parse second upstream frame");
    assert_eq!(second_payload["type"], "response.create");
    assert_eq!(second_payload["input"], "first request");
    assert!(second_payload.get("previous_response_id").is_none());

    let first_client_event = tokio::time::timeout(Duration::from_secs(5), client_ws.next())
        .await
        .expect("client retry event timeout")
        .expect("client retry event")
        .expect("client retry event result");
    match first_client_event {
        Message::Text(text) => {
            assert!(
                text.contains("\"response.completed\""),
                "unexpected retry event: {text}"
            );
        }
        other => panic!("unexpected retry client event: {other:?}"),
    }

    let captures = tokio::time::timeout(Duration::from_secs(5), capture_rx)
        .await
        .expect("capture timeout")
        .expect("capture result");
    assert_eq!(
        captures.len(),
        2,
        "expected two upstream websocket sessions"
    );
    assert_eq!(
        captures[0].headers.get("authorization").map(String::as_str),
        Some("Bearer access_token_ws_a")
    );
    assert_eq!(
        captures[1].headers.get("authorization").map(String::as_str),
        Some("Bearer access_token_ws_b")
    );
    assert_eq!(
        captures[0]
            .headers
            .get("chatgpt-account-id")
            .map(String::as_str),
        Some("chatgpt_proxy_runtime_ws_a")
    );
    assert_eq!(
        captures[1]
            .headers
            .get("chatgpt-account-id")
            .map(String::as_str),
        Some("chatgpt_proxy_runtime_ws_b")
    );

    client_ws.close(None).await.expect("close client websocket");
    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("front proxy shutdown timeout")
        .expect("join front proxy");
    tokio::time::timeout(Duration::from_secs(5), upstream_handle)
        .await
        .expect("mock upstream shutdown timeout")
        .expect("join mock upstream");
}
