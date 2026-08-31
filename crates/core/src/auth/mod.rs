use base64::Engine;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const DEFAULT_ISSUER: &str = "https://auth.openai.com";
pub const DEFAULT_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const DEFAULT_ORIGINATOR: &str = "codex_cli_rs";

#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenClaims {
    pub sub: String,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(rename = "https://api.openai.com/auth", default)]
    pub auth: Option<AuthClaims>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthClaims {
    #[serde(default)]
    pub chatgpt_account_id: Option<String>,
    #[serde(default)]
    pub chatgpt_plan_type: Option<String>,
    #[serde(default)]
    pub chatgpt_user_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PkceCodes {
    pub code_verifier: String,
    pub code_challenge: String,
}

/// 函数 `generate_pkce`
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
pub fn generate_pkce() -> PkceCodes {
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    let code_verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let digest = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    PkceCodes {
        code_verifier,
        code_challenge,
    }
}

/// 函数 `generate_state`
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
pub fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// 函数 `parse_id_token_claims`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - token: 参数 token
///
/// # 返回
/// 返回函数执行结果
pub fn parse_id_token_claims(token: &str) -> Result<IdTokenClaims, String> {
    let mut parts = token.split('.');
    let _header = parts.next();
    let payload = parts.next().ok_or_else(|| "invalid token".to_string())?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| e.to_string())?;
    let json = std::str::from_utf8(&decoded).map_err(|e| e.to_string())?;
    serde_json::from_str(json).map_err(|e| e.to_string())
}

pub fn extract_client_id_claim(token: &str) -> Option<String> {
    parse_id_token_claims(token)
        .ok()
        .and_then(|claims| claims.client_id)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_scoped_identity_value(value: Option<&str>, marker: &str) -> Option<String> {
    let raw = value.map(str::trim).filter(|value| !value.is_empty())?;

    let scoped = raw
        .rsplit_once("::")
        .map(|(_, suffix)| suffix)
        .unwrap_or(raw);
    if let Some(found) = scoped.split('|').find_map(|segment| {
        segment
            .trim()
            .strip_prefix(marker)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }) {
        return Some(found);
    }

    if raw.contains("::")
        || raw.contains('|')
        || raw.contains('=')
        || raw.starts_with("import-sub-")
    {
        return None;
    }

    Some(raw.to_string())
}

pub fn normalize_chatgpt_account_id(value: Option<&str>) -> Option<String> {
    normalize_scoped_identity_value(value, "cgpt=")
}

pub fn normalize_workspace_id(value: Option<&str>) -> Option<String> {
    normalize_scoped_identity_value(value, "ws=")
}

/// 函数 `extract_token_exp`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - token: 参数 token
///
/// # 返回
/// 返回函数执行结果
pub fn extract_token_exp(token: &str) -> Option<i64> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let json = std::str::from_utf8(&decoded).ok()?;
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    value.get("exp").and_then(|v| v.as_i64())
}

/// 函数 `extract_chatgpt_account_id`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - token: 参数 token
///
/// # 返回
/// 返回函数执行结果
pub fn extract_chatgpt_account_id(token: &str) -> Option<String> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let json = std::str::from_utf8(&decoded).ok()?;
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    if let Some(v) = value.get("chatgpt_account_id").and_then(|v| v.as_str()) {
        if let Some(account_id) = normalize_chatgpt_account_id(Some(v)) {
            return Some(account_id);
        }
    }
    value
        .get("https://api.openai.com/auth")
        .and_then(|v| v.get("chatgpt_account_id"))
        .and_then(|v| v.as_str())
        .and_then(|v| normalize_chatgpt_account_id(Some(v)))
}

/// 函数 `extract_chatgpt_user_id`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - token: 参数 token
///
/// # 返回
/// 返回函数执行结果
pub fn extract_chatgpt_user_id(token: &str) -> Option<String> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let json = std::str::from_utf8(&decoded).ok()?;
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    for key in ["chatgpt_user_id", "user_id"] {
        if let Some(v) = value.get(key).and_then(|v| v.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    value
        .get("https://api.openai.com/auth")
        .and_then(|v| v.as_object())
        .and_then(|auth| {
            ["chatgpt_user_id", "user_id"].into_iter().find_map(|key| {
                auth.get(key)
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string)
            })
        })
        .or_else(|| {
            value
                .get("sub")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        })
}

/// 函数 `extract_workspace_id`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - token: 参数 token
///
/// # 返回
/// 返回函数执行结果
pub fn extract_workspace_id(token: &str) -> Option<String> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let json = std::str::from_utf8(&decoded).ok()?;
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let keys = [
        "workspace_id",
        "chatgpt_account_id",
        "organization_id",
        "org_id",
    ];
    for key in keys {
        if let Some(v) = value.get(key).and_then(|v| v.as_str()) {
            if let Some(workspace_id) = normalize_workspace_id(Some(v)) {
                return Some(workspace_id);
            }
        }
    }
    if let Some(auth) = value
        .get("https://api.openai.com/auth")
        .and_then(|v| v.as_object())
    {
        if let Some(orgs) = auth.get("organizations").and_then(|v| v.as_array()) {
            if let Some(default_org) = orgs.iter().find(|item| {
                item.get("is_default")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            }) {
                if let Some(v) = default_org.get("id").and_then(|v| v.as_str()) {
                    if let Some(workspace_id) = normalize_workspace_id(Some(v)) {
                        return Some(workspace_id);
                    }
                }
            }
            if let Some(first_org) = orgs.first() {
                if let Some(v) = first_org.get("id").and_then(|v| v.as_str()) {
                    if let Some(workspace_id) = normalize_workspace_id(Some(v)) {
                        return Some(workspace_id);
                    }
                }
            }
        }
        for key in keys {
            if let Some(v) = auth.get(key).and_then(|v| v.as_str()) {
                if let Some(workspace_id) = normalize_workspace_id(Some(v)) {
                    return Some(workspace_id);
                }
            }
        }
    }
    None
}

/// 函数 `extract_workspace_name`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - token: 参数 token
///
/// # 返回
/// 返回函数执行结果
pub fn extract_workspace_name(token: &str) -> Option<String> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let json = std::str::from_utf8(&decoded).ok()?;
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let keys = [
        "organization_name",
        "org_name",
        "workspace_name",
        "team_name",
        "organization",
    ];
    for key in keys {
        if let Some(v) = value.get(key).and_then(|v| v.as_str()) {
            if !v.trim().is_empty() {
                return Some(v.to_string());
            }
        }
    }
    if let Some(auth) = value
        .get("https://api.openai.com/auth")
        .and_then(|v| v.as_object())
    {
        for key in keys {
            if let Some(v) = auth.get(key).and_then(|v| v.as_str()) {
                if !v.trim().is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

/// 函数 `build_authorize_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - issuer: 参数 issuer
/// - client_id: 参数 client_id
/// - redirect_uri: 参数 redirect_uri
/// - code_challenge: 参数 code_challenge
/// - state: 参数 state
/// - originator: 参数 originator
/// - workspace_id: 参数 workspace_id
///
/// # 返回
/// 返回函数执行结果
pub fn build_authorize_url(
    issuer: &str,
    client_id: &str,
    redirect_uri: &str,
    code_challenge: &str,
    state: &str,
    originator: &str,
    workspace_id: Option<&str>,
) -> String {
    let mut query = vec![
        ("response_type", "code".to_string()),
        ("client_id", client_id.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
        (
            "scope",
            "openid profile email offline_access api.connectors.read api.connectors.invoke"
                .to_string(),
        ),
        ("code_challenge", code_challenge.to_string()),
        ("code_challenge_method", "S256".to_string()),
        ("id_token_add_organizations", "true".to_string()),
        ("codex_cli_simplified_flow", "true".to_string()),
        ("state", state.to_string()),
        ("originator", originator.to_string()),
    ];
    if let Some(workspace_id) = workspace_id {
        query.push(("allowed_workspace_id", workspace_id.to_string()));
    }
    let qs = query
        .into_iter()
        .map(|(k, v)| format!("{k}={}", urlencoding::encode(&v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{issuer}/oauth/authorize?{qs}")
}

/// 函数 `token_exchange_body_authorization_code`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - code: 参数 code
/// - redirect_uri: 参数 redirect_uri
/// - client_id: 参数 client_id
/// - code_verifier: 参数 code_verifier
///
/// # 返回
/// 返回函数执行结果
pub fn token_exchange_body_authorization_code(
    code: &str,
    redirect_uri: &str,
    client_id: &str,
    code_verifier: &str,
) -> String {
    format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}",
        urlencoding::encode(code),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(client_id),
        urlencoding::encode(code_verifier)
    )
}

/// 函数 `token_exchange_body_token_exchange`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - id_token: 参数 id_token
/// - client_id: 参数 client_id
///
/// # 返回
/// 返回函数执行结果
pub fn token_exchange_body_token_exchange(id_token: &str, client_id: &str) -> String {
    format!(
        "grant_type={}&client_id={}&requested_token={}&subject_token={}&subject_token_type={}",
        urlencoding::encode("urn:ietf:params:oauth:grant-type:token-exchange"),
        urlencoding::encode(client_id),
        urlencoding::encode("openai-api-key"),
        urlencoding::encode(id_token),
        urlencoding::encode("urn:ietf:params:oauth:token-type:id_token")
    )
}

/// 函数 `device_usercode_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - issuer: 参数 issuer
///
/// # 返回
/// 返回函数执行结果
pub fn device_usercode_url(issuer: &str) -> String {
    format!(
        "{}/api/accounts/deviceauth/usercode",
        issuer.trim_end_matches('/')
    )
}

/// 函数 `device_token_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - issuer: 参数 issuer
///
/// # 返回
/// 返回函数执行结果
pub fn device_token_url(issuer: &str) -> String {
    format!(
        "{}/api/accounts/deviceauth/token",
        issuer.trim_end_matches('/')
    )
}

/// 函数 `device_verification_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - issuer: 参数 issuer
///
/// # 返回
/// 返回函数执行结果
pub fn device_verification_url(issuer: &str) -> String {
    format!("{}/codex/device", issuer.trim_end_matches('/'))
}

/// 函数 `device_redirect_uri`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - issuer: 参数 issuer
///
/// # 返回
/// 返回函数执行结果
pub fn device_redirect_uri(issuer: &str) -> String {
    format!("{}/deviceauth/callback", issuer.trim_end_matches('/'))
}
