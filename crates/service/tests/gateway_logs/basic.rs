use super::*;
use codexmanager_core::storage::RequestTokenStat;

const MISSING_AUTH_JSON_OPENAI_API_KEY_ERROR: &str =
    "配置错误：未配置auth.json的OPENAI_API_KEY(invalid api key)";

/// 函数 `gateway_logs_invalid_api_key_error`
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
fn gateway_logs_invalid_api_key_error() {
    let _lock = test_env_guard();
    let dir = new_test_dir("codexmanager-gateway-logs");
    let db_path: PathBuf = dir.join("codexmanager.db");

    let _guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());

    let server = TestServer::start();
    let req_body = r#"{"model":"gpt-5.3-codex","input":"hello"}"#;
    let (status, body) = post_http_raw(
        &server.addr,
        "/v1/responses",
        req_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", "Bearer invalid-platform-key"),
        ],
    );
    assert_eq!(status, 403);
    assert!(
        body.contains("invalid api key"),
        "gateway should return raw upstream message, got {body}"
    );
    assert!(
        !body.contains("未配置auth.json"),
        "gateway response should not expose bilingual log text, got {body}"
    );

    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");
    let mut logs = Vec::new();
    for _ in 0..40 {
        logs = storage
            .list_request_logs(None, 100)
            .expect("list request logs");
        if !logs.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let found = logs.iter().any(|item| {
        item.request_path == "/v1/responses"
            && item.status_code == Some(403)
            && item.input_tokens.is_none()
            && item.cached_input_tokens.is_none()
            && item.output_tokens.is_none()
            && item.total_tokens.is_none()
            && item.reasoning_output_tokens.is_none()
            && item.error.as_deref() == Some(MISSING_AUTH_JSON_OPENAI_API_KEY_ERROR)
    });
    assert!(
        found,
        "expected missing auth.json OPENAI_API_KEY request to be logged, got {:?}",
        logs.iter()
            .map(|v| (&v.request_path, v.status_code, v.error.as_deref()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn gateway_rejects_api_key_after_quota_limit() {
    let _lock = test_env_guard();
    let dir = new_test_dir("codexmanager-gateway-key-quota");
    let db_path: PathBuf = dir.join("codexmanager.db");
    let _guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());

    let platform_key = "pk_quota_limit_reached";
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    storage
        .insert_api_key(&ApiKey {
            id: "gk_quota_limit_reached".to_string(),
            name: Some("quota-limit".to_string()),
            model_slug: None,
            reasoning_effort: None,
            service_tier: None,
            rotation_strategy: "account_rotation".to_string(),
            aggregate_api_id: None,
            account_plan_filter: None,
            aggregate_api_url: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: hash_platform_key_for_test(platform_key),
            status: "active".to_string(),
            created_at: now,
            last_used_at: None,
        })
        .expect("insert api key");
    storage
        .upsert_api_key_quota_limit("gk_quota_limit_reached", Some(100))
        .expect("upsert quota");
    storage
        .insert_request_token_stat(&RequestTokenStat {
            request_log_id: 1,
            key_id: Some("gk_quota_limit_reached".to_string()),
            total_tokens: Some(100),
            created_at: now,
            ..RequestTokenStat::default()
        })
        .expect("insert token stat");

    let server = TestServer::start();
    let req_body = r#"{"model":"gpt-5.3-codex","input":"hello"}"#;
    let (status, body) = post_http_raw(
        &server.addr,
        "/v1/responses",
        req_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    assert_eq!(status, 429, "response body: {body}");
    assert!(
        body.contains("quota") || body.contains("额度"),
        "gateway should report quota exhaustion, got {body}"
    );
}

#[test]
fn gateway_reports_wallet_quota_exhaustion_in_chinese() {
    let _lock = test_env_guard();
    let dir = new_test_dir("codexmanager-gateway-wallet-quota");
    let db_path: PathBuf = dir.join("codexmanager.db");
    let _guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());

    let platform_key = "pk_wallet_quota_exhausted";
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    storage
        .set_app_setting("distribution.enabled", "true", now)
        .expect("enable distribution");
    storage
        .insert_app_user(&AppUser {
            id: "usr_wallet_quota_exhausted".to_string(),
            username: "wallet-quota-member".to_string(),
            display_name: None,
            password_hash: "test-password-hash".to_string(),
            role: "member".to_string(),
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
            last_login_at: None,
        })
        .expect("insert app user");
    storage
        .insert_api_key(&ApiKey {
            id: "gk_wallet_quota_exhausted".to_string(),
            name: Some("wallet-quota".to_string()),
            model_slug: None,
            reasoning_effort: None,
            service_tier: None,
            rotation_strategy: "account_rotation".to_string(),
            aggregate_api_id: None,
            account_plan_filter: None,
            aggregate_api_url: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: hash_platform_key_for_test(platform_key),
            status: "active".to_string(),
            created_at: now,
            last_used_at: None,
        })
        .expect("insert api key");
    storage
        .upsert_api_key_owner(&ApiKeyOwner {
            key_id: "gk_wallet_quota_exhausted".to_string(),
            owner_kind: "user".to_string(),
            owner_user_id: Some("usr_wallet_quota_exhausted".to_string()),
            project_id: None,
            updated_at: now,
        })
        .expect("insert api key owner");
    storage
        .ensure_wallet_for_owner(
            "wlt_wallet_quota_exhausted",
            "user",
            "usr_wallet_quota_exhausted",
        )
        .expect("ensure zero wallet");

    let server = TestServer::start();
    let req_body = r#"{"model":"gpt-5.3-codex","input":"hello"}"#;
    let (status, body) = post_http_raw(
        &server.addr,
        "/v1/responses",
        req_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    assert_eq!(status, 402, "response body: {body}");
    assert!(
        body.contains("额度不足，请联系管理员"),
        "gateway should report wallet quota exhaustion in Chinese, got {body}"
    );
    assert!(
        !body.contains("Too Many Requests"),
        "wallet quota exhaustion must not look like retryable upstream throttling, got {body}"
    );
}

#[test]
fn gateway_reports_platform_model_route_errors() {
    let _lock = test_env_guard();
    let dir = new_test_dir("codexmanager-gateway-model-route-errors");
    let db_path: PathBuf = dir.join("codexmanager.db");
    let _guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());

    let platform_key = "pk_model_route_errors";
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    storage
        .insert_api_key(&ApiKey {
            id: "gk_model_route_errors".to_string(),
            name: Some("model-route-errors".to_string()),
            model_slug: None,
            reasoning_effort: None,
            service_tier: None,
            rotation_strategy: "account_rotation".to_string(),
            aggregate_api_id: None,
            account_plan_filter: None,
            aggregate_api_url: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: hash_platform_key_for_test(platform_key),
            status: "active".to_string(),
            created_at: now,
            last_used_at: None,
        })
        .expect("insert api key");

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let (status, body) = post_http_raw(
        &server.addr,
        "/v1/responses",
        r#"{"model":"missing-platform","input":"hello"}"#,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    server.join();
    assert_eq!(status, 404, "response body: {body}");
    assert!(
        body.contains("model_not_found"),
        "gateway should report missing platform model, got {body}"
    );

    seed_model_catalog_models(&storage, &["gpt-platform"]);

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let (status, body) = post_http_raw(
        &server.addr,
        "/v1/responses",
        r#"{"model":"gpt-platform","input":"hello"}"#,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    server.join();
    assert_eq!(status, 503, "response body: {body}");
    assert!(
        body.contains("model_unavailable"),
        "gateway should report platform model without enabled mappings, got {body}"
    );
}

#[test]
fn gateway_rewrites_account_pool_model_from_enabled_mapping() {
    let _lock = test_env_guard();
    let dir = new_test_dir("codexmanager-gateway-account-model-mapping");
    let db_path: PathBuf = dir.join("codexmanager.db");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());

    let response = serde_json::json!({
        "id": "resp_model_mapping",
        "model": "gpt-upstream",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "ok" }]
        }],
        "usage": { "input_tokens": 2, "output_tokens": 1, "total_tokens": 3 }
    });
    let (upstream_addr, upstream_rx, upstream_join) =
        start_mock_upstream_once(&serde_json::to_string(&response).expect("serialize response"));
    let upstream_base = format!("http://{upstream_addr}/v1");
    let _upstream_guard = EnvGuard::set("CODEXMANAGER_UPSTREAM_BASE_URL", &upstream_base);

    let platform_key = "pk_account_model_mapping";
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    seed_model_catalog_models(&storage, &["gpt-platform"]);
    storage
        .insert_account(&Account {
            id: "acc_model_mapping".to_string(),
            label: "mapping account".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: Some("ws_model_mapping".to_string()),
            group_name: None,
            sort: 1,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&Token {
            account_id: "acc_model_mapping".to_string(),
            id_token: String::new(),
            access_token: "access_token_model_mapping".to_string(),
            refresh_token: String::new(),
            api_key_access_token: Some("api_access_token_model_mapping".to_string()),
            last_refresh: now,
        })
        .expect("insert token");
    storage
        .insert_api_key(&ApiKey {
            id: "gk_account_model_mapping".to_string(),
            name: Some("account-model-mapping".to_string()),
            model_slug: None,
            reasoning_effort: None,
            service_tier: None,
            rotation_strategy: "account_rotation".to_string(),
            aggregate_api_id: None,
            account_plan_filter: None,
            aggregate_api_url: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: hash_platform_key_for_test(platform_key),
            status: "active".to_string(),
            created_at: now,
            last_used_at: None,
        })
        .expect("insert api key");
    storage
        .upsert_model_source_model(&ModelSourceModel {
            source_kind: "openai_account".to_string(),
            source_id: "acc_model_mapping".to_string(),
            upstream_model: "gpt-upstream".to_string(),
            display_name: None,
            status: "available".to_string(),
            discovery_kind: "manual".to_string(),
            last_synced_at: Some(now),
            extra_json: "{}".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("upsert source model");
    storage
        .upsert_model_source_mapping(&ModelSourceMapping {
            id: "map_account_model_mapping".to_string(),
            platform_model_slug: "gpt-platform".to_string(),
            source_kind: "openai_account".to_string(),
            source_id: "acc_model_mapping".to_string(),
            upstream_model: "gpt-upstream".to_string(),
            enabled: true,
            priority: 0,
            weight: 1,
            billing_model_slug: None,
            created_at: now,
            updated_at: now,
        })
        .expect("upsert mapping");

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let (status, response_body) = post_http_raw(
        &server.addr,
        "/v1/responses",
        r#"{"model":"gpt-platform","input":"hello","stream":false}"#,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    server.join();
    assert_eq!(status, 200, "gateway response: {response_body}");

    let upstream_request = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive upstream request");
    upstream_join.join().expect("join upstream");
    let request_body: serde_json::Value =
        serde_json::from_slice(&decode_upstream_request_body(&upstream_request))
            .expect("parse upstream request body");
    assert_eq!(
        request_body
            .get("model")
            .and_then(serde_json::Value::as_str),
        Some("gpt-upstream")
    );
    let logs = storage
        .list_request_logs(None, 10)
        .expect("list request logs");
    let log = logs.first().expect("request log should be written");
    assert_eq!(log.model.as_deref(), Some("gpt-platform"));
    assert_eq!(log.upstream_model.as_deref(), Some("gpt-upstream"));
    assert_eq!(log.actual_source_kind.as_deref(), Some("openai_account"));
    assert_eq!(log.actual_source_id.as_deref(), Some("acc_model_mapping"));
}

#[test]
fn gateway_applies_saved_model_forward_rules_to_codex_responses_request() {
    let _lock = test_env_guard();
    let dir = new_test_dir("codexmanager-gateway-saved-model-forward-rules");
    let db_path: PathBuf = dir.join("codexmanager.db");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());

    let response = serde_json::json!({
        "id": "resp_saved_model_forward_rules",
        "model": "gpt-5.4-mini",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "ok" }]
        }],
        "usage": { "input_tokens": 2, "output_tokens": 1, "total_tokens": 3 }
    });
    let (upstream_addr, upstream_rx, upstream_join) =
        start_mock_upstream_once(&serde_json::to_string(&response).expect("serialize response"));
    let upstream_base = format!("http://{upstream_addr}/backend-api/codex");
    let _upstream_guard = EnvGuard::set("CODEXMANAGER_UPSTREAM_BASE_URL", &upstream_base);

    let platform_key = "pk_saved_model_forward_rules";
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    seed_model_catalog_models(&storage, &["spark", "gpt-5.4-mini"]);
    storage
        .insert_account(&Account {
            id: "acc_saved_model_forward_rules".to_string(),
            label: "saved-model-forward-rules".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: Some("ws_saved_model_forward_rules".to_string()),
            group_name: None,
            sort: 1,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&Token {
            account_id: "acc_saved_model_forward_rules".to_string(),
            id_token: String::new(),
            access_token: "access_token_saved_model_forward_rules".to_string(),
            refresh_token: String::new(),
            api_key_access_token: Some("api_access_token_saved_model_forward_rules".to_string()),
            last_refresh: now,
        })
        .expect("insert token");
    storage
        .upsert_model_source_model(&ModelSourceModel {
            source_kind: "openai_account".to_string(),
            source_id: "acc_saved_model_forward_rules".to_string(),
            upstream_model: "gpt-5.4-mini".to_string(),
            display_name: Some("gpt-5.4-mini".to_string()),
            status: "available".to_string(),
            discovery_kind: "manual".to_string(),
            last_synced_at: Some(now),
            extra_json: "{}".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("seed source model");
    storage
        .insert_api_key(&ApiKey {
            id: "gk_saved_model_forward_rules".to_string(),
            name: Some("saved-model-forward-rules".to_string()),
            model_slug: None,
            reasoning_effort: None,
            service_tier: None,
            rotation_strategy: "account_rotation".to_string(),
            aggregate_api_id: None,
            account_plan_filter: None,
            aggregate_api_url: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: hash_platform_key_for_test(platform_key),
            status: "active".to_string(),
            created_at: now,
            last_used_at: None,
        })
        .expect("insert api key");

    codexmanager_service::app_settings_set(Some(&serde_json::json!({
        "modelForwardRules": "spark*=gpt-5.4-mini"
    })))
    .expect("save app settings");

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let (status, response_body) = post_http_raw(
        &server.addr,
        "/v1/responses",
        r#"{"model":"spark","input":"hello","stream":false}"#,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    server.join();
    assert_eq!(status, 200, "gateway response: {response_body}");

    let upstream_request = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive upstream request");
    upstream_join.join().expect("join upstream");
    let request_body: serde_json::Value =
        serde_json::from_slice(&decode_upstream_request_body(&upstream_request))
            .expect("parse upstream request body");
    assert_eq!(
        request_body
            .get("model")
            .and_then(serde_json::Value::as_str),
        Some("gpt-5.4-mini")
    );
}

/// 函数 `gateway_tolerates_non_ascii_turn_metadata_header`
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
fn gateway_tolerates_non_ascii_turn_metadata_header() {
    let _lock = test_env_guard();
    let dir = new_test_dir("codexmanager-gateway-logs-nonascii");
    let db_path: PathBuf = dir.join("codexmanager.db");

    let _guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());

    let server = TestServer::start();
    let req_body = r#"{"model":"gpt-5.3-codex","input":"hello"}"#;
    let metadata = r#"{"workspaces":{"D:\\MyComputer\\own\\GPTTeam相关\\CodexManager\\CodexManager":{"latest_git_commit_hash":"abc123"}}}"#;
    let (status, body) = post_http_raw(
        &server.addr,
        "/v1/responses",
        req_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", "Bearer invalid-platform-key"),
            ("x-codex-turn-metadata", metadata),
        ],
    );
    assert_eq!(status, 403, "response body: {body}");
}
