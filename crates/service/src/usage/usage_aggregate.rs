use std::collections::HashMap;

use codexmanager_core::rpc::types::UsageAggregateSummaryResult;
use codexmanager_core::storage::{Account, UsageSnapshotRecord};
use serde_json::Value;

use crate::storage_helpers::open_storage;

const MINUTES_PER_HOUR: i64 = 60;
const MINUTES_PER_DAY: i64 = 24 * MINUTES_PER_HOUR;
const ROUNDING_BIAS: i64 = 3;

/// 函数 `read_usage_aggregate_summary`
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
pub(crate) fn read_usage_aggregate_summary() -> Result<UsageAggregateSummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let accounts = storage
        .list_accounts()
        .map_err(|err| format!("list accounts failed: {err}"))?;
    let usage_items = storage
        .latest_usage_snapshots_by_account()
        .map_err(|err| format!("list usage snapshots failed: {err}"))?;

    Ok(compute_usage_aggregate_summary(&accounts, &usage_items))
}

/// 函数 `compute_usage_aggregate_summary`
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
pub(crate) fn compute_usage_aggregate_summary(
    accounts: &[Account],
    usage_items: &[UsageSnapshotRecord],
) -> UsageAggregateSummaryResult {
    let usage_map = usage_items
        .iter()
        .map(|item| (item.account_id.as_str(), item))
        .collect::<HashMap<_, _>>();

    let mut primary_bucket_count = 0_i64;
    let mut primary_known_count = 0_i64;
    let mut primary_remaining_total = 0_f64;
    let mut secondary_bucket_count = 0_i64;
    let mut secondary_known_count = 0_i64;
    let mut secondary_remaining_total = 0_f64;

    for account in accounts {
        let usage = usage_map.get(account.id.as_str()).copied();
        let has_primary_signal = usage
            .map(|value| value.used_percent.is_some() || value.window_minutes.is_some())
            .unwrap_or(false);
        let has_secondary_signal = usage
            .map(|value| {
                value.secondary_used_percent.is_some() || value.secondary_window_minutes.is_some()
            })
            .unwrap_or(false);
        let primary_belongs_to_secondary = usage
            .map(|value| {
                !has_secondary_signal
                    && (is_long_window(value.window_minutes)
                        || is_free_plan_usage(value.credits_json.as_deref()))
            })
            .unwrap_or(false);

        if has_primary_signal {
            if primary_belongs_to_secondary {
                secondary_bucket_count += 1;
            } else {
                primary_bucket_count += 1;
            }
        }

        if let Some(primary_remain) = usage.and_then(|value| remaining_percent(value.used_percent))
        {
            if primary_belongs_to_secondary {
                secondary_known_count += 1;
                secondary_remaining_total += primary_remain;
            } else {
                primary_known_count += 1;
                primary_remaining_total += primary_remain;
            }
        }

        if has_secondary_signal {
            secondary_bucket_count += 1;
        }
        if let Some(secondary_remain) =
            usage.and_then(|value| remaining_percent(value.secondary_used_percent))
        {
            secondary_known_count += 1;
            secondary_remaining_total += secondary_remain;
        }
    }

    UsageAggregateSummaryResult {
        primary_bucket_count,
        primary_known_count,
        primary_unknown_count: (primary_bucket_count - primary_known_count).max(0),
        primary_remain_percent: average_percent(primary_remaining_total, primary_known_count),
        secondary_bucket_count,
        secondary_known_count,
        secondary_unknown_count: (secondary_bucket_count - secondary_known_count).max(0),
        secondary_remain_percent: average_percent(secondary_remaining_total, secondary_known_count),
    }
}

/// 函数 `normalize_percent`
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
fn normalize_percent(value: Option<f64>) -> Option<f64> {
    value.map(|parsed| parsed.clamp(0.0, 100.0))
}

/// 函数 `remaining_percent`
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
fn remaining_percent(value: Option<f64>) -> Option<f64> {
    normalize_percent(value).map(|used| (100.0 - used).max(0.0))
}

/// 函数 `average_percent`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - total: 参数 total
/// - count: 参数 count
///
/// # 返回
/// 返回函数执行结果
fn average_percent(total: f64, count: i64) -> Option<i64> {
    if count <= 0 {
        return None;
    }
    Some((total / count as f64).round() as i64)
}

/// 函数 `is_long_window`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - window_minutes: 参数 window_minutes
///
/// # 返回
/// 返回函数执行结果
fn is_long_window(window_minutes: Option<i64>) -> bool {
    window_minutes.is_some_and(|value| value > MINUTES_PER_DAY + ROUNDING_BIAS)
}

/// 函数 `is_free_plan_usage`
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
fn is_free_plan_usage(raw: Option<&str>) -> bool {
    let Some(value) = parse_credits(raw) else {
        return false;
    };
    extract_plan_type_recursive(&value)
        .map(|value| value.contains("free"))
        .unwrap_or(false)
}

/// 函数 `parse_credits`
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
fn parse_credits(raw: Option<&str>) -> Option<Value> {
    let text = raw?.trim();
    if text.is_empty() {
        return None;
    }
    serde_json::from_str(text).ok()
}

/// 函数 `extract_plan_type_recursive`
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
fn extract_plan_type_recursive(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => items.iter().find_map(extract_plan_type_recursive),
        Value::Object(map) => {
            for key in [
                "plan_type",
                "planType",
                "subscription_tier",
                "subscriptionTier",
                "tier",
                "account_type",
                "accountType",
                "type",
            ] {
                if let Some(text) = map.get(key).and_then(Value::as_str) {
                    let normalized = text.trim().to_ascii_lowercase();
                    if !normalized.is_empty() {
                        return Some(normalized);
                    }
                }
            }
            map.values().find_map(extract_plan_type_recursive)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::compute_usage_aggregate_summary;
    use codexmanager_core::storage::{now_ts, Account, UsageSnapshotRecord};

    /// 函数 `account`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - id: 参数 id
    ///
    /// # 返回
    /// 返回函数执行结果
    fn account(id: &str) -> Account {
        Account {
            id: id.to_string(),
            label: id.to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now_ts(),
            updated_at: now_ts(),
        }
    }

    /// 函数 `usage_record`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - account_id: 参数 account_id
    /// - used_percent: 参数 used_percent
    /// - window_minutes: 参数 window_minutes
    /// - secondary_used_percent: 参数 secondary_used_percent
    /// - secondary_window_minutes: 参数 secondary_window_minutes
    /// - credits_json: 参数 credits_json
    ///
    /// # 返回
    /// 返回函数执行结果
    fn usage_record(
        account_id: &str,
        used_percent: Option<f64>,
        window_minutes: Option<i64>,
        secondary_used_percent: Option<f64>,
        secondary_window_minutes: Option<i64>,
        credits_json: Option<&str>,
    ) -> UsageSnapshotRecord {
        UsageSnapshotRecord {
            account_id: account_id.to_string(),
            used_percent,
            window_minutes,
            resets_at: None,
            secondary_used_percent,
            secondary_window_minutes,
            secondary_resets_at: None,
            credits_json: credits_json.map(|value| value.to_string()),
            captured_at: now_ts(),
        }
    }

    /// 函数 `aggregate_summary_routes_free_single_window_account_to_secondary_bucket`
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
    fn aggregate_summary_routes_free_single_window_account_to_secondary_bucket() {
        let accounts = vec![account("a1"), account("a2")];
        let usage_items = vec![
            usage_record("a1", Some(20.0), Some(300), Some(40.0), Some(10080), None),
            usage_record(
                "a2",
                Some(10.0),
                Some(10080),
                None,
                None,
                Some(r#"{"planType":"free"}"#),
            ),
        ];

        let result = compute_usage_aggregate_summary(&accounts, &usage_items);
        assert_eq!(result.primary_bucket_count, 1);
        assert_eq!(result.primary_known_count, 1);
        assert_eq!(result.primary_remain_percent, Some(80));
        assert_eq!(result.secondary_bucket_count, 2);
        assert_eq!(result.secondary_known_count, 2);
        assert_eq!(result.secondary_remain_percent, Some(75));
    }

    /// 函数 `aggregate_summary_preserves_unknown_counts_per_bucket`
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
    fn aggregate_summary_preserves_unknown_counts_per_bucket() {
        let accounts = vec![account("a1"), account("a2"), account("a3")];
        let usage_items = vec![
            usage_record("a1", Some(20.0), Some(300), None, None, None),
            usage_record(
                "a2",
                None,
                Some(10080),
                None,
                None,
                Some(r#"{"planType":"free"}"#),
            ),
        ];

        let result = compute_usage_aggregate_summary(&accounts, &usage_items);
        assert_eq!(result.primary_bucket_count, 1);
        assert_eq!(result.primary_known_count, 1);
        assert_eq!(result.primary_unknown_count, 0);
        assert_eq!(result.secondary_bucket_count, 1);
        assert_eq!(result.secondary_known_count, 0);
        assert_eq!(result.secondary_unknown_count, 1);
        assert_eq!(result.secondary_remain_percent, None);
    }
}
