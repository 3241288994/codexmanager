use super::{
    refresh_all_chatgpt_auth_tokens, resolve_plan_type, resolve_plan_type_raw,
    resolve_refresh_target, set_current_auth_account_id,
};
use codexmanager_core::auth::parse_id_token_claims;
use codexmanager_core::storage::{now_ts, Account, Storage, Token};

/// 函数 `jwt_with_claims`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - payload: 参数 payload
///
/// # 返回
/// 返回函数执行结果
fn jwt_with_claims(payload: &str) -> String {
    format!("eyJhbGciOiJIUzI1NiJ9.{payload}.sig")
}

/// 函数 `build_token`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - access_token: 参数 access_token
/// - id_token: 参数 id_token
///
/// # 返回
/// 返回函数执行结果
fn build_token(access_token: &str, id_token: &str) -> Token {
    Token {
        account_id: "acc-1".to_string(),
        id_token: id_token.to_string(),
        access_token: access_token.to_string(),
        refresh_token: "refresh".to_string(),
        api_key_access_token: None,
        last_refresh: 0,
    }
}

fn build_account(
    id: &str,
    chatgpt_account_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Account {
    Account {
        id: id.to_string(),
        label: id.to_string(),
        issuer: "issuer".to_string(),
        chatgpt_account_id: chatgpt_account_id.map(str::to_string),
        workspace_id: workspace_id.map(str::to_string),
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    }
}

fn build_refresh_token(account_id: &str, refresh_token: &str) -> Token {
    Token {
        account_id: account_id.to_string(),
        id_token: "id".to_string(),
        access_token: "access".to_string(),
        refresh_token: refresh_token.to_string(),
        api_key_access_token: None,
        last_refresh: 0,
    }
}

#[test]
fn resolve_refresh_target_prefers_explicit_account_id_over_current_account() {
    let _guard = crate::test_env_guard();
    let previous_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
    let db_path = std::env::temp_dir().join(format!(
        "codexmanager-auth-target-{}-{}.sqlite",
        std::process::id(),
        now_ts()
    ));
    std::env::set_var("CODEXMANAGER_DB_PATH", &db_path);

    {
        let storage = Storage::open(&db_path).expect("open storage");
        storage.init().expect("init storage");
        storage
            .insert_account(&build_account("acc-current", Some("org-current"), None))
            .expect("insert current account");
        storage
            .insert_account(&build_account(
                "acc-target",
                Some("org-target"),
                Some("ws-target"),
            ))
            .expect("insert target account");
        storage
            .insert_token(&build_refresh_token("acc-current", "refresh-current"))
            .expect("insert current token");
        storage
            .insert_token(&build_refresh_token("acc-target", "refresh-target"))
            .expect("insert target token");
        set_current_auth_account_id(Some("acc-current")).expect("set current account");

        let (account, token) = resolve_refresh_target(&storage, Some("acc-target"))
            .expect("resolve target")
            .expect("target exists");

        assert_eq!(account.id, "acc-target");
        assert_eq!(token.refresh_token, "refresh-target");
        set_current_auth_account_id(None).expect("clear current account");
    }

    match previous_db_path {
        Some(value) => std::env::set_var("CODEXMANAGER_DB_PATH", value),
        None => std::env::remove_var("CODEXMANAGER_DB_PATH"),
    }
    let _ = std::fs::remove_file(db_path);
}

#[test]
fn refresh_all_chatgpt_auth_tokens_skips_accounts_without_refresh_token() {
    let _guard = crate::test_env_guard();
    let previous_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
    let db_path = std::env::temp_dir().join(format!(
        "codexmanager-auth-refresh-all-{}-{}.sqlite",
        std::process::id(),
        now_ts()
    ));
    std::env::set_var("CODEXMANAGER_DB_PATH", &db_path);

    {
        let storage = Storage::open(&db_path).expect("open storage");
        storage.init().expect("init storage");
        storage
            .insert_account(&build_account("missing-token", None, None))
            .expect("insert missing token account");
        storage
            .insert_account(&build_account("missing-refresh", None, None))
            .expect("insert missing refresh account");
        storage
            .insert_token(&build_refresh_token("missing-refresh", ""))
            .expect("insert empty refresh token");

        let result = refresh_all_chatgpt_auth_tokens().expect("refresh all");

        assert_eq!(result.requested, 0);
        assert_eq!(result.succeeded, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.skipped, 2);
        assert_eq!(result.results.len(), 2);
        assert!(result
            .results
            .iter()
            .any(|item| item.account_id == "missing-token"
                && item.message.as_deref() == Some("missing token")));
        assert!(result
            .results
            .iter()
            .any(|item| item.account_id == "missing-refresh"
                && item.message.as_deref() == Some("missing refresh_token")));
    }

    match previous_db_path {
        Some(value) => std::env::set_var("CODEXMANAGER_DB_PATH", value),
        None => std::env::remove_var("CODEXMANAGER_DB_PATH"),
    }
    let _ = std::fs::remove_file(db_path);
}

/// 函数 `resolve_plan_type_prefers_latest_access_token_claims`
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
fn resolve_plan_type_prefers_latest_access_token_claims() {
    let access_token = jwt_with_claims(
        "eyJzdWIiOiJ1c2VyLTEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9wbGFuX3R5cGUiOiJnbyJ9fQ",
    );
    let id_token = jwt_with_claims(
        "eyJzdWIiOiJ1c2VyLTEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9wbGFuX3R5cGUiOiJwcm8ifX0",
    );
    let token = build_token(&access_token, &id_token);
    let claims = parse_id_token_claims(&access_token).expect("access claims");

    let resolved = resolve_plan_type(&token, Some(&claims));

    assert_eq!(resolved.as_deref(), Some("go"));
}

/// 函数 `resolve_plan_type_falls_back_to_id_token_when_access_claims_missing`
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
fn resolve_plan_type_falls_back_to_id_token_when_access_claims_missing() {
    let access_token = jwt_with_claims("eyJzdWIiOiJ1c2VyLTEifQ");
    let id_token = jwt_with_claims(
        "eyJzdWIiOiJ1c2VyLTEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9wbGFuX3R5cGUiOiJ0ZWFtIn19",
    );
    let token = build_token(&access_token, &id_token);
    let claims = parse_id_token_claims(&access_token).expect("access claims");

    let resolved = resolve_plan_type(&token, Some(&claims));

    assert_eq!(resolved.as_deref(), Some("team"));
}

/// 函数 `resolve_plan_type_preserves_unknown_raw_value_for_diagnostics`
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
fn resolve_plan_type_preserves_unknown_raw_value_for_diagnostics() {
    let access_token = jwt_with_claims(
        "eyJzdWIiOiJ1c2VyLTEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9wbGFuX3R5cGUiOiJyZXNlYXJjaGVyX2JldGEifX0",
    );
    let token = build_token(&access_token, &access_token);
    let claims = parse_id_token_claims(&access_token).expect("access claims");

    let resolved = resolve_plan_type(&token, Some(&claims));
    let raw = resolve_plan_type_raw(&token, Some(&claims));

    assert_eq!(resolved.as_deref(), Some("unknown"));
    assert_eq!(raw.as_deref(), Some("researcher_beta"));
}
