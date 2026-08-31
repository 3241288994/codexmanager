use codexmanager_core::auth::parse_id_token_claims;
use codexmanager_core::rpc::types::ModelsResponse;
use codexmanager_core::storage::{Account, Storage, Token};
use reqwest::Method;
use serde_json::Value;

mod parse;
mod request;

pub(crate) use parse::parse_models_response;
use request::send_models_request;

/// 函数 `should_retry_models_with_openai_fallback`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - err: 参数 err
///
/// # 返回
/// 返回函数执行结果
fn should_retry_models_with_openai_fallback(err: &str) -> bool {
    let normalized = err.to_ascii_lowercase();
    normalized.contains("cloudflare")
        || normalized.contains("text/html")
        || normalized.contains("<html")
        || normalized.contains("<!doctype html")
        || normalized.contains("body=<html")
        || normalized.contains("challenge")
}

/// 函数 `fetch_models_for_picker`
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
pub(crate) fn fetch_models_for_picker() -> Result<ModelsResponse, String> {
    if crate::gateway::current_codex_user_agent_version()
        == crate::gateway::default_codex_user_agent_version()
    {
        if let Err(err) = crate::app_settings::sync_gateway_user_agent_version_from_codex_latest() {
            log::warn!("codex latest client_version sync before models failed: {err}");
        }
    }
    let storage = super::open_storage()
        .ok_or_else(|| crate::gateway::bilingual_error("存储不可用", "storage unavailable"))?;
    let mut candidates = super::collect_gateway_candidates(&storage)?;
    if candidates.is_empty() {
        return Err(crate::gateway::bilingual_error(
            "无可用账号 Token，请重新登录或导入 AT/RT",
            "no account token available",
        ));
    }

    let upstream_base = super::resolve_upstream_base_url();
    let base = upstream_base.as_str();
    let upstream_fallback_base = super::resolve_upstream_fallback_base_url(base);
    let path = super::normalize_models_path("/v1/models");
    let method = Method::GET;
    sort_model_picker_candidates(&storage, &mut candidates);
    let mut last_error = "models request failed".to_string();
    for (account, mut token) in candidates {
        match send_models_request(
            &storage,
            &method,
            &upstream_base,
            &path,
            &account,
            &mut token,
        ) {
            Ok(response_body) => return Ok(parse_models_response(&response_body)),
            Err(err) => {
                // ChatGPT upstream occasionally returns HTML challenge. Try OpenAI fallback.
                if should_retry_models_with_openai_fallback(&err) {
                    if let Some(fallback_base) = upstream_fallback_base.as_deref() {
                        if let Ok(response_body) = send_models_request(
                            &storage,
                            &method,
                            fallback_base,
                            &path,
                            &account,
                            &mut token,
                        ) {
                            return Ok(parse_models_response(&response_body));
                        }
                    }
                }
                last_error = err;
            }
        }
    }

    Err(last_error)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ModelPickerPlanTier {
    Pro,
    Team,
    Plus,
    Go,
    Free,
    Unknown,
}

/// 函数 `sort_model_picker_candidates`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - storage: 参数 storage
/// - candidates: 参数 candidates
///
/// # 返回
/// 无
fn sort_model_picker_candidates(storage: &Storage, candidates: &mut [(Account, Token)]) {
    candidates.sort_by_key(|(account, token)| {
        (
            super::is_account_in_cooldown(&account.id),
            super::account_inflight_count(&account.id),
            resolve_model_picker_plan_tier(storage, account.id.as_str(), token),
        )
    });
}

/// 函数 `resolve_model_picker_plan_tier`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - storage: 参数 storage
/// - account_id: 参数 account_id
/// - token: 参数 token
///
/// # 返回
/// 返回函数执行结果
fn resolve_model_picker_plan_tier(
    storage: &Storage,
    account_id: &str,
    token: &Token,
) -> ModelPickerPlanTier {
    plan_tier_from_token(&token.access_token)
        .or_else(|| plan_tier_from_token(&token.id_token))
        .or_else(|| plan_tier_from_usage_snapshot(storage, account_id))
        .unwrap_or(ModelPickerPlanTier::Unknown)
}

/// 函数 `plan_tier_from_token`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - raw_token: 参数 raw_token
///
/// # 返回
/// 返回函数执行结果
fn plan_tier_from_token(raw_token: &str) -> Option<ModelPickerPlanTier> {
    parse_id_token_claims(raw_token)
        .ok()
        .and_then(|claims| claims.auth.and_then(|auth| auth.chatgpt_plan_type))
        .and_then(|value| normalize_model_picker_plan_tier(value.as_str()))
}

/// 函数 `plan_tier_from_usage_snapshot`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - storage: 参数 storage
/// - account_id: 参数 account_id
///
/// # 返回
/// 返回函数执行结果
fn plan_tier_from_usage_snapshot(
    storage: &Storage,
    account_id: &str,
) -> Option<ModelPickerPlanTier> {
    storage
        .latest_usage_snapshot_for_account(account_id)
        .ok()
        .flatten()
        .and_then(|snapshot| snapshot.credits_json)
        .and_then(|raw| plan_tier_from_credits_json(raw.as_str()))
}

/// 函数 `plan_tier_from_credits_json`
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
fn plan_tier_from_credits_json(raw: &str) -> Option<ModelPickerPlanTier> {
    let value = serde_json::from_str::<Value>(raw).ok()?;
    extract_plan_string_by_keys_recursive(
        &value,
        &[
            "plan_type",
            "planType",
            "subscription_tier",
            "subscriptionTier",
            "tier",
            "account_type",
            "accountType",
            "type",
        ],
    )
    .and_then(|value| normalize_model_picker_plan_tier(value.as_str()))
}

/// 函数 `extract_plan_string_by_keys_recursive`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
/// - keys: 参数 keys
///
/// # 返回
/// 返回函数执行结果
fn extract_plan_string_by_keys_recursive(value: &Value, keys: &[&str]) -> Option<String> {
    if let Some(object) = value.as_object() {
        for key in keys {
            let candidate = object
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(ToString::to_string);
            if candidate.is_some() {
                return candidate;
            }
        }

        for child in object.values() {
            if let Some(nested) = extract_plan_string_by_keys_recursive(child, keys) {
                return Some(nested);
            }
        }
    }

    if let Some(array) = value.as_array() {
        for child in array {
            if let Some(nested) = extract_plan_string_by_keys_recursive(child, keys) {
                return Some(nested);
            }
        }
    }

    None
}

/// 函数 `normalize_model_picker_plan_tier`
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
fn normalize_model_picker_plan_tier(raw: &str) -> Option<ModelPickerPlanTier> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    match normalized.as_str() {
        "pro" => Some(ModelPickerPlanTier::Pro),
        "team" | "business" | "enterprise" | "edu" | "education" => Some(ModelPickerPlanTier::Team),
        "plus" => Some(ModelPickerPlanTier::Plus),
        "go" => Some(ModelPickerPlanTier::Go),
        "free" => Some(ModelPickerPlanTier::Free),
        _ if normalized.contains("enterprise")
            || normalized.contains("business")
            || normalized.contains("team")
            || normalized.contains("education")
            || normalized.contains("edu") =>
        {
            Some(ModelPickerPlanTier::Team)
        }
        _ if normalized.contains("pro") => Some(ModelPickerPlanTier::Pro),
        _ if normalized.contains("plus") => Some(ModelPickerPlanTier::Plus),
        _ if normalized.contains("go") => Some(ModelPickerPlanTier::Go),
        _ if normalized.contains("free") => Some(ModelPickerPlanTier::Free),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use codexmanager_core::storage::{now_ts, Storage, Token};

    use super::{should_retry_models_with_openai_fallback, sort_model_picker_candidates, Account};

    /// 函数 `encode_base64url`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - bytes: 参数 bytes
    ///
    /// # 返回
    /// 返回函数执行结果
    fn encode_base64url(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = String::new();
        let mut index = 0;
        while index + 3 <= bytes.len() {
            let chunk = ((bytes[index] as u32) << 16)
                | ((bytes[index + 1] as u32) << 8)
                | (bytes[index + 2] as u32);
            out.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
            out.push(TABLE[((chunk >> 6) & 0x3f) as usize] as char);
            out.push(TABLE[(chunk & 0x3f) as usize] as char);
            index += 3;
        }
        match bytes.len().saturating_sub(index) {
            1 => {
                let chunk = (bytes[index] as u32) << 16;
                out.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
                out.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
            }
            2 => {
                let chunk = ((bytes[index] as u32) << 16) | ((bytes[index + 1] as u32) << 8);
                out.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
                out.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
                out.push(TABLE[((chunk >> 6) & 0x3f) as usize] as char);
            }
            _ => {}
        }
        out
    }

    /// 函数 `plan_token`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - plan: 参数 plan
    ///
    /// # 返回
    /// 返回函数执行结果
    fn plan_token(plan: &str) -> String {
        let header = encode_base64url(br#"{"alg":"none","typ":"JWT"}"#);
        let payload = encode_base64url(
            serde_json::json!({
                "sub": format!("acc-{plan}"),
                "https://api.openai.com/auth": {
                    "chatgpt_plan_type": plan
                }
            })
            .to_string()
            .as_bytes(),
        );
        format!("{header}.{payload}.sig")
    }

    /// 函数 `candidate`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - id: 参数 id
    /// - sort: 参数 sort
    /// - plan: 参数 plan
    ///
    /// # 返回
    /// 返回函数执行结果
    fn candidate(id: &str, sort: i64, plan: &str) -> (Account, Token) {
        let now = now_ts();
        let token = plan_token(plan);
        (
            Account {
                id: id.to_string(),
                label: id.to_string(),
                issuer: "issuer".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: None,
                sort,
                status: "active".to_string(),
                created_at: now,
                updated_at: now,
            },
            Token {
                account_id: id.to_string(),
                id_token: token.clone(),
                access_token: token,
                refresh_token: "refresh".to_string(),
                api_key_access_token: None,
                last_refresh: now,
            },
        )
    }

    /// 函数 `fallback_retry_matches_stable_html_and_challenge_summaries`
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
    fn fallback_retry_matches_stable_html_and_challenge_summaries() {
        assert!(should_retry_models_with_openai_fallback(
            "models upstream failed: status=403 body=Cloudflare 安全验证页（title=Just a moment...）"
        ));
        assert!(should_retry_models_with_openai_fallback(
            "models upstream failed: status=502 body=<html><head><title>502 Bad Gateway</title></head></html>"
        ));
        assert!(!should_retry_models_with_openai_fallback(
            "models upstream failed: status=401 body=missing_authorization_header"
        ));
    }

    /// 函数 `sort_model_picker_candidates_prefers_plan_tier_priority`
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
    fn sort_model_picker_candidates_prefers_plan_tier_priority() {
        let storage = Storage::open_in_memory().expect("open");
        storage.init().expect("init");
        let mut candidates = vec![
            candidate("acc-free", 0, "free"),
            candidate("acc-team-a", 1, "team"),
            candidate("acc-plus", 2, "plus"),
            candidate("acc-pro", 3, "pro"),
            candidate("acc-go", 4, "go"),
            candidate("acc-team-b", 5, "business"),
        ];

        sort_model_picker_candidates(&storage, &mut candidates);

        let ids = candidates
            .iter()
            .map(|(account, _)| account.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "acc-pro",
                "acc-team-a",
                "acc-team-b",
                "acc-plus",
                "acc-go",
                "acc-free",
            ]
        );
    }
}
