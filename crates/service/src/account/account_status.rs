use codexmanager_core::storage::{now_ts, Event, Storage};

pub(crate) const REFRESH_TOKEN_REGION_BLOCKED_REASON: &str = "refresh_token_region_blocked";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccountAvailabilitySignal {
    RefreshToken(crate::usage_http::RefreshTokenAuthErrorReason),
    RefreshTokenRegionBlocked,
    Deactivation(&'static str),
    UsageHttp(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GatewayErrorKind {
    Deactivation,
    UsageLimit,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GatewayErrorFollowUp {
    pub kind: GatewayErrorKind,
    pub should_failover: bool,
    pub should_mark_account_unavailable: bool,
    pub should_mark_default_cooldown: bool,
}

/// 函数 `latest_status_reason`
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
fn latest_status_reason(storage: &Storage, account_id: &str) -> Option<String> {
    storage
        .latest_account_status_reasons(&[account_id.to_string()])
        .ok()
        .and_then(|mut reasons| reasons.remove(account_id))
}

/// 函数 `set_account_status`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 无
pub(crate) fn set_account_status(storage: &Storage, account_id: &str, status: &str, reason: &str) {
    let changed = matches!(
        storage.update_account_status_if_changed(account_id, status),
        Ok(true)
    );
    if changed {
        crate::gateway::invalidate_candidate_cache();
    }
    let account_exists = storage
        .find_account_by_id(account_id)
        .ok()
        .flatten()
        .is_some();
    if account_exists
        && (changed || latest_status_reason(storage, account_id).as_deref() != Some(reason))
    {
        let _ = storage.insert_event(&Event {
            account_id: Some(account_id.to_string()),
            event_type: "account_status_update".to_string(),
            message: format!("status={status} reason={reason}"),
            created_at: now_ts(),
        });
    }
}

/// 函数 `should_preserve_manual_account_status`
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
fn should_preserve_manual_account_status(storage: &Storage, account_id: &str) -> bool {
    storage
        .find_account_by_id(account_id)
        .ok()
        .flatten()
        .map(|account| {
            account.status.trim().eq_ignore_ascii_case("disabled")
                || account.status.trim().eq_ignore_ascii_case("inactive")
        })
        .unwrap_or(false)
}

/// 函数 `classify_account_availability_signal`
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
pub(crate) fn classify_account_availability_signal(err: &str) -> Option<AccountAvailabilitySignal> {
    if crate::usage_http::is_refresh_token_region_blocked_error_message(err) {
        return Some(AccountAvailabilitySignal::RefreshTokenRegionBlocked);
    }
    if let Some(reason) = crate::usage_http::refresh_token_auth_error_reason_from_message(err) {
        return Some(AccountAvailabilitySignal::RefreshToken(reason));
    }
    if let Some(reason) = deactivation_reason_from_message(err) {
        return Some(AccountAvailabilitySignal::Deactivation(reason));
    }
    if let Some(status_code) = extract_usage_http_status_code(err) {
        return Some(AccountAvailabilitySignal::UsageHttp(status_code));
    }
    None
}

/// 函数 `extract_usage_http_status_code`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - message: 参数 message
///
/// # 返回
/// 返回函数执行结果
fn extract_usage_http_status_code(message: &str) -> Option<u16> {
    let trimmed = message.trim();
    let rest = if let Some(rest) = trimmed.strip_prefix("usage endpoint status ") {
        Some(rest)
    } else if let Some(rest) = trimmed.strip_prefix("usage endpoint failed: status=") {
        Some(rest)
    } else if let Some(rest) = trimmed.strip_prefix("subscription endpoint status ") {
        Some(rest)
    } else if let Some(rest) = trimmed.strip_prefix("subscription endpoint failed: status=") {
        Some(rest)
    } else {
        None
    }?;
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u16>().ok()
}

/// 函数 `deactivation_reason_from_message`
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
pub(crate) fn deactivation_reason_from_message(message: &str) -> Option<&'static str> {
    let normalized = message.trim().to_ascii_lowercase();
    if normalized.contains("workspace_deactivated")
        || normalized.contains("deactivated_workspace")
        || normalized.contains("workspace deactivated")
        || normalized.contains("workspace-deactivated")
        || normalized.contains("deactivated workspace")
    {
        return Some("workspace_deactivated");
    }
    if normalized.contains("account_deactivated")
        || normalized.contains("account deactivated")
        || normalized.contains("deactivated")
    {
        return Some("account_deactivated");
    }
    None
}

pub(crate) fn usage_limit_reason_from_message(message: &str) -> Option<&'static str> {
    let normalized = message.trim().to_ascii_lowercase();
    if normalized.contains("you've hit your usage limit")
        || normalized.contains("you have hit your usage limit")
        || normalized.contains("usage limit has been reached")
        || normalized.contains("insufficient_quota")
        || normalized.contains("quota exceeded")
        || normalized.contains("usage exhausted")
        || (normalized.contains("usage limit") && normalized.contains("try again"))
    {
        return Some("usage_limit_exhausted");
    }
    None
}

pub(crate) fn analyze_gateway_error(err: &str, has_more_candidates: bool) -> GatewayErrorFollowUp {
    let kind = if deactivation_reason_from_message(err).is_some() {
        GatewayErrorKind::Deactivation
    } else if usage_limit_reason_from_message(err).is_some() {
        GatewayErrorKind::UsageLimit
    } else {
        GatewayErrorKind::Other
    };
    let is_actionable = !matches!(kind, GatewayErrorKind::Other);
    let should_failover = has_more_candidates && is_actionable;
    GatewayErrorFollowUp {
        kind,
        should_failover,
        should_mark_account_unavailable: is_actionable,
        should_mark_default_cooldown: matches!(kind, GatewayErrorKind::UsageLimit)
            && should_failover,
    }
}

/// 函数 `is_banned_status_reason`
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
pub(crate) fn is_banned_status_reason(reason: &str) -> bool {
    matches!(
        reason.trim().to_ascii_lowercase().as_str(),
        "account_deactivated" | "workspace_deactivated" | "deactivated_workspace"
    )
}

pub(crate) fn is_refresh_blocked_status_reason(reason: &str) -> bool {
    reason
        .trim()
        .eq_ignore_ascii_case(REFRESH_TOKEN_REGION_BLOCKED_REASON)
}

pub(crate) fn is_account_refresh_blocked_status_reason(reason: &str) -> bool {
    is_banned_status_reason(reason) || is_refresh_blocked_status_reason(reason)
}

/// 函数 `should_failover_for_deactivation_error`
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
pub(crate) fn mark_account_unavailable_for_gateway_error(
    storage: &Storage,
    account_id: &str,
    err: &str,
) -> bool {
    if let Some(reason) = deactivation_reason_from_message(err) {
        return set_account_banned_with_reason(storage, account_id, reason);
    }
    if usage_limit_reason_from_message(err).is_some() {
        return mark_account_unavailable_for_confirmed_usage_exhausted(storage, account_id);
    }
    false
}

fn mark_account_unavailable_for_confirmed_usage_exhausted(
    storage: &Storage,
    account_id: &str,
) -> bool {
    set_account_limited_with_reason(storage, account_id, "usage_limit_exhausted")
}

/// 函数 `set_account_unavailable_with_reason`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - storage: 参数 storage
/// - account_id: 参数 account_id
/// - reason: 参数 reason
///
/// # 返回
/// 返回函数执行结果
fn set_account_unavailable_with_reason(storage: &Storage, account_id: &str, reason: &str) -> bool {
    if should_preserve_manual_account_status(storage, account_id) {
        return false;
    }
    set_account_status(storage, account_id, "unavailable", reason);
    true
}

fn set_account_limited_with_reason(storage: &Storage, account_id: &str, reason: &str) -> bool {
    if should_preserve_manual_account_status(storage, account_id) {
        return false;
    }
    set_account_status(storage, account_id, "limited", reason);
    true
}

/// 函数 `set_account_banned_with_reason`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - storage: 参数 storage
/// - account_id: 参数 account_id
/// - reason: 参数 reason
///
/// # 返回
/// 返回函数执行结果
fn set_account_banned_with_reason(storage: &Storage, account_id: &str, reason: &str) -> bool {
    if should_preserve_manual_account_status(storage, account_id) {
        return false;
    }
    set_account_status(storage, account_id, "banned", reason);
    true
}

/// 函数 `mark_account_unavailable_for_usage_http_error`
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
pub(crate) fn mark_account_unavailable_for_usage_http_error(
    storage: &Storage,
    account_id: &str,
    err: &str,
) -> bool {
    let Some(AccountAvailabilitySignal::UsageHttp(status_code)) =
        classify_account_availability_signal(err)
    else {
        return false;
    };
    match status_code {
        401 | 403 => {
            let status_reason = format!("usage_http_{status_code}");
            set_account_unavailable_with_reason(storage, account_id, &status_reason)
        }
        _ => false,
    }
}

/// 函数 `mark_account_unavailable_for_deactivation_error`
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
pub(crate) fn mark_account_unavailable_for_deactivation_error(
    storage: &Storage,
    account_id: &str,
    err: &str,
) -> bool {
    let Some(AccountAvailabilitySignal::Deactivation(reason)) =
        classify_account_availability_signal(err)
    else {
        return false;
    };
    set_account_banned_with_reason(storage, account_id, reason)
}

/// 函数 `mark_account_unavailable_for_auth_error`
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
pub(crate) fn mark_account_unavailable_for_auth_error(
    storage: &Storage,
    account_id: &str,
    err: &str,
) -> bool {
    let Some(signal) = classify_account_availability_signal(err) else {
        return false;
    };
    match signal {
        AccountAvailabilitySignal::RefreshTokenRegionBlocked => {
            set_account_unavailable_with_reason(
                storage,
                account_id,
                REFRESH_TOKEN_REGION_BLOCKED_REASON,
            )
        }
        AccountAvailabilitySignal::RefreshToken(reason) => {
            let status_reason = format!("refresh_token_invalid:{}", reason.as_code());
            set_account_unavailable_with_reason(storage, account_id, &status_reason)
        }
        AccountAvailabilitySignal::Deactivation(reason) => {
            set_account_banned_with_reason(storage, account_id, reason)
        }
        AccountAvailabilitySignal::UsageHttp(_) => false,
    }
}

/// 函数 `mark_account_unavailable_for_refresh_token_error`
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
pub(crate) fn mark_account_unavailable_for_refresh_token_error(
    storage: &Storage,
    account_id: &str,
    err: &str,
) -> bool {
    match classify_account_availability_signal(err) {
        Some(AccountAvailabilitySignal::RefreshTokenRegionBlocked) => {
            set_account_unavailable_with_reason(
                storage,
                account_id,
                REFRESH_TOKEN_REGION_BLOCKED_REASON,
            )
        }
        Some(AccountAvailabilitySignal::RefreshToken(reason)) => {
            let status_reason = format!("refresh_token_invalid:{}", reason.as_code());
            set_account_unavailable_with_reason(storage, account_id, &status_reason)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        analyze_gateway_error, classify_account_availability_signal,
        mark_account_unavailable_for_gateway_error, AccountAvailabilitySignal, GatewayErrorKind,
    };
    use codexmanager_core::storage::{now_ts, Account, Storage, UsageSnapshotRecord};

    /// 函数 `classify_account_availability_signal_separates_usage_refresh_and_deactivation`
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
    fn classify_account_availability_signal_separates_usage_refresh_and_deactivation() {
        assert!(matches!(
            classify_account_availability_signal("usage endpoint status 401 Unauthorized"),
            Some(AccountAvailabilitySignal::UsageHttp(401))
        ));
        assert!(matches!(
            classify_account_availability_signal("usage endpoint status 403 Forbidden"),
            Some(AccountAvailabilitySignal::UsageHttp(403))
        ));
        assert!(matches!(
            classify_account_availability_signal("usage endpoint status 429 Too Many Requests"),
            Some(AccountAvailabilitySignal::UsageHttp(429))
        ));
        assert!(matches!(
            classify_account_availability_signal("subscription endpoint status 403 Forbidden"),
            Some(AccountAvailabilitySignal::UsageHttp(403))
        ));
        assert!(matches!(
            classify_account_availability_signal(
                "subscription endpoint failed: status=401 Unauthorized body=token expired"
            ),
            Some(AccountAvailabilitySignal::UsageHttp(401))
        ));

        assert!(matches!(
            classify_account_availability_signal(
                "refresh token failed with status 401 Unauthorized: Your access token could not be refreshed because your refresh token was revoked. Please log out and sign in again."
            ),
            Some(AccountAvailabilitySignal::RefreshToken(
                crate::usage_http::RefreshTokenAuthErrorReason::Invalidated
            ))
        ));

        assert!(matches!(
            classify_account_availability_signal("account_deactivated"),
            Some(AccountAvailabilitySignal::Deactivation(
                "account_deactivated"
            ))
        ));

        let deactivation = analyze_gateway_error("Your OpenAI account has been deactivated", true);
        assert_eq!(deactivation.kind, GatewayErrorKind::Deactivation);
        assert!(deactivation.should_failover);
        assert!(deactivation.should_mark_account_unavailable);
        assert!(!deactivation.should_mark_default_cooldown);

        let usage_limit = analyze_gateway_error(
            "You've hit your usage limit. To get more access now, try again at 8:02 PM.",
            true,
        );
        assert_eq!(usage_limit.kind, GatewayErrorKind::UsageLimit);
        assert!(usage_limit.should_failover);
        assert!(usage_limit.should_mark_account_unavailable);
        assert!(usage_limit.should_mark_default_cooldown);

        let usage_limit_last = analyze_gateway_error(
            "You've hit your usage limit. To get more access now, try again at 8:02 PM.",
            false,
        );
        assert_eq!(usage_limit_last.kind, GatewayErrorKind::UsageLimit);
        assert!(!usage_limit_last.should_failover);
        assert!(usage_limit_last.should_mark_account_unavailable);
        assert!(!usage_limit_last.should_mark_default_cooldown);

        // Regression: backend-native WS upstream phrasing.
        let ws_usage_limit = analyze_gateway_error("The usage limit has been reached", true);
        assert_eq!(ws_usage_limit.kind, GatewayErrorKind::UsageLimit);
        assert!(ws_usage_limit.should_failover);
        assert!(ws_usage_limit.should_mark_account_unavailable);
        assert!(ws_usage_limit.should_mark_default_cooldown);
    }

    /// 函数 `gateway_usage_limit_error_marks_account_limited_immediately`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-03
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn gateway_usage_limit_error_marks_account_limited_immediately() {
        let _guard = crate::test_env_guard();
        let storage = Storage::open_in_memory().expect("open storage");
        storage.init().expect("init storage");
        let now = now_ts();
        storage
            .insert_account(&Account {
                id: "acc-usage-limit".to_string(),
                label: "usage-limit".to_string(),
                issuer: "issuer".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: None,
                sort: 0,
                status: "active".to_string(),
                created_at: now,
                updated_at: now,
            })
            .expect("insert account");

        assert!(mark_account_unavailable_for_gateway_error(
            &storage,
            "acc-usage-limit",
            "You've hit your usage limit. To get more access now, try again at 8:02 PM."
        ));

        let account = storage
            .find_account_by_id("acc-usage-limit")
            .expect("find account")
            .expect("account exists");
        assert_eq!(account.status, "limited");
        let reasons = storage
            .latest_account_status_reasons(&["acc-usage-limit".to_string()])
            .expect("load reasons");
        assert_eq!(
            reasons.get("acc-usage-limit").map(String::as_str),
            Some("usage_limit_exhausted")
        );
    }

    /// 函数 `gateway_usage_limit_error_marks_account_limited_when_snapshot_exhausted`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-03
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn gateway_usage_limit_error_marks_account_limited_when_snapshot_exhausted() {
        let _guard = crate::test_env_guard();
        let storage = Storage::open_in_memory().expect("open storage");
        storage.init().expect("init storage");
        let now = now_ts();
        storage
            .insert_account(&Account {
                id: "acc-usage-exhausted".to_string(),
                label: "usage-exhausted".to_string(),
                issuer: "issuer".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: None,
                sort: 0,
                status: "active".to_string(),
                created_at: now,
                updated_at: now,
            })
            .expect("insert account");
        storage
            .insert_usage_snapshot(&UsageSnapshotRecord {
                account_id: "acc-usage-exhausted".to_string(),
                used_percent: Some(100.0),
                window_minutes: Some(300),
                resets_at: None,
                secondary_used_percent: Some(100.0),
                secondary_window_minutes: Some(10080),
                secondary_resets_at: None,
                credits_json: None,
                captured_at: now,
            })
            .expect("insert usage snapshot");

        assert!(mark_account_unavailable_for_gateway_error(
            &storage,
            "acc-usage-exhausted",
            "You've hit your usage limit. To get more access now, try again at 8:02 PM."
        ));

        let account = storage
            .find_account_by_id("acc-usage-exhausted")
            .expect("find account")
            .expect("account exists");
        assert_eq!(account.status, "limited");
        let reasons = storage
            .latest_account_status_reasons(&["acc-usage-exhausted".to_string()])
            .expect("load reasons");
        assert_eq!(
            reasons.get("acc-usage-exhausted").map(String::as_str),
            Some("usage_limit_exhausted")
        );
    }
}
