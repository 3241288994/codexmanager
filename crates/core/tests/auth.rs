use base64::Engine;
use codexmanager_core::auth::{build_authorize_url, extract_client_id_claim};

fn jwt_with_json(payload_json: &str) -> String {
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json);
    format!("eyJhbGciOiJIUzI1NiJ9.{payload}.sig")
}

/// 函数 `build_authorize_url_matches_codex`
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
fn build_authorize_url_matches_codex() {
    let issuer = "https://auth.openai.com";
    let client_id = "app_123";
    let redirect_uri = "http://localhost:1455/auth/callback";
    let code_challenge = "challenge";
    let state = "state123";
    let originator = "codex_cli";

    let url = build_authorize_url(
        issuer,
        client_id,
        redirect_uri,
        code_challenge,
        state,
        originator,
        Some("org_abc"),
    );

    assert!(url.starts_with("https://auth.openai.com/oauth/authorize?"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("client_id=app_123"));
    assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback"));
    assert!(url.contains(
        "scope=openid%20profile%20email%20offline_access%20api.connectors.read%20api.connectors.invoke"
    ));
    assert!(url.contains("code_challenge=challenge"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("id_token_add_organizations=true"));
    assert!(url.contains("codex_cli_simplified_flow=true"));
    assert!(url.contains("state=state123"));
    assert!(url.contains("originator=codex_cli"));
    assert!(url.contains("allowed_workspace_id=org_abc"));
}

/// 函数 `parse_id_token_claims_extracts_email_and_sub`
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
fn parse_id_token_claims_extracts_email_and_sub() {
    let token =
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyLTEiLCJlbWFpbCI6InRlc3RAZXhhbXBsZS5jb20ifQ.sig";
    let claims = codexmanager_core::auth::parse_id_token_claims(token).expect("claims");
    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.email.as_deref(), Some("test@example.com"));
}

#[test]
fn parse_id_token_claims_extracts_client_id() {
    let token = jwt_with_json(
        r#"{"sub":"user-1","client_id":"app_EMoamEEZ73f0CkXaXp7hrann","email":"test@example.com"}"#,
    );
    let claims = codexmanager_core::auth::parse_id_token_claims(&token).expect("claims");

    assert_eq!(
        claims.client_id.as_deref(),
        Some("app_EMoamEEZ73f0CkXaXp7hrann")
    );
    assert_eq!(
        extract_client_id_claim(&token).as_deref(),
        Some("app_EMoamEEZ73f0CkXaXp7hrann")
    );
}

/// 函数 `extract_token_exp_reads_exp_claim`
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
fn extract_token_exp_reads_exp_claim() {
    let token = "eyJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3NzA0NjU4ODYsInN1YiI6InVzZXItMSJ9.sig";
    let exp = codexmanager_core::auth::extract_token_exp(token);
    assert_eq!(exp, Some(1770465886));
}

#[test]
fn normalize_scoped_identity_values_extracts_account_and_workspace_segments() {
    let composite =
        "google-oauth2|105671307665841419748::cgpt=ed08d56a-c038-4322-b325-53f504c0c88c|ws=org-AP6ypcMi84Thfueli6EU3B4m";

    assert_eq!(
        codexmanager_core::auth::normalize_chatgpt_account_id(Some(composite)),
        Some("ed08d56a-c038-4322-b325-53f504c0c88c".to_string())
    );
    assert_eq!(
        codexmanager_core::auth::normalize_workspace_id(Some(composite)),
        Some("org-AP6ypcMi84Thfueli6EU3B4m".to_string())
    );
}

#[test]
fn extract_scope_ids_from_token_filters_storage_style_identity_suffix() {
    let token = jwt_with_json(
        r#"{"sub":"user-1","workspace_id":"google-oauth2|105671307665841419748::cgpt=ed08d56a-c038-4322-b325-53f504c0c88c|ws=org-AP6ypcMi84Thfueli6EU3B4m","https://api.openai.com/auth":{"chatgpt_account_id":"google-oauth2|105671307665841419748::cgpt=ed08d56a-c038-4322-b325-53f504c0c88c|ws=org-AP6ypcMi84Thfueli6EU3B4m"}}"#,
    );

    assert_eq!(
        codexmanager_core::auth::extract_chatgpt_account_id(&token),
        Some("ed08d56a-c038-4322-b325-53f504c0c88c".to_string())
    );
    assert_eq!(
        codexmanager_core::auth::extract_workspace_id(&token),
        Some("org-AP6ypcMi84Thfueli6EU3B4m".to_string())
    );
}

#[test]
fn extract_chatgpt_user_id_prefers_nested_user_identity() {
    let token = jwt_with_json(
        r#"{"sub":"subject-1","https://api.openai.com/auth":{"chatgpt_user_id":"user-1","user_id":"fallback-user"}}"#,
    );

    assert_eq!(
        codexmanager_core::auth::extract_chatgpt_user_id(&token),
        Some("user-1".to_string())
    );
}
