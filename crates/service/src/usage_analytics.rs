//! Daily workspace analytics. Missing metrics must never become fabricated zeros.
pub(crate) mod pricing;
use chrono::{Duration, NaiveDate, Utc};
use codexmanager_core::storage::{
    now_ts,
    usage_analytics::{AnalyticsDay, AnalyticsSync, AnalyticsTotals},
};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeSet, sync::Mutex};

const RATE_KEY: &str = "usage_analytics_usd_per_credit";
static REFRESH_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AnalyticsReport {
    account_id: String,
    source: &'static str,
    start_date: String,
    end_date: String,
    days: Vec<AnalyticsDay>,
    sync: Option<AnalyticsSync>,
    usd_per_credit: Option<f64>,
    pricing: pricing::PricingState,
}

pub(crate) fn validate_range(start: &str, end: &str) -> Result<(), String> {
    let parse = |s: &str| {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .filter(|d| d.format("%Y-%m-%d").to_string() == s)
            .ok_or_else(|| "日期格式必须为 YYYY-MM-DD".to_string())
    };
    let (start, end) = (parse(start)?, parse(end)?);
    let days = (end - start).num_days();
    if !(1..=366).contains(&days) || end > Utc::now().date_naive() + Duration::days(1) {
        return Err("请选择最多 366 天的有效日期范围，结束日期不能晚于明天".into());
    }
    Ok(())
}

fn integer(totals: &Value, key: &str) -> Result<Option<u64>, String> {
    match totals.get(key).filter(|v| !v.is_null()) {
        None => Ok(None),
        Some(v) => v
            .as_u64()
            .filter(|n| *n <= 9_007_199_254_740_991)
            .map(Some)
            .ok_or_else(|| format!("用量接口字段 {key} 不是有效非负整数")),
    }
}

pub(crate) fn parse_days(
    value: &Value,
    start: &str,
    end: &str,
    captured_at: i64,
) -> Result<Vec<AnalyticsDay>, String> {
    if value
        .get("balance_unit")
        .and_then(Value::as_str)
        .is_some_and(|s| s != "credit")
    {
        return Err("用量接口返回了未知 Credits 单位，已保留历史数据".into());
    }
    if value
        .get("group_by")
        .and_then(Value::as_str)
        .is_some_and(|s| s != "day")
    {
        return Err("用量接口返回了非每日分组".into());
    }
    let rows = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or("用量接口缺少 data 数组，已保留历史数据")?;
    if rows.len() > 366 {
        return Err("用量接口返回的行数超出范围".into());
    }
    let mut seen = BTreeSet::new();
    let mut days = Vec::new();
    for row in rows {
        let date = row
            .get("date")
            .and_then(Value::as_str)
            .ok_or("每日用量缺少日期")?;
        let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| "每日用量日期无效")?;
        if parsed.format("%Y-%m-%d").to_string() != date
            || date < start
            || date >= end
            || !seen.insert(date)
        {
            return Err("每日用量包含重复或范围外日期".into());
        }
        let raw = row
            .get("totals")
            .filter(|v| v.is_object())
            .ok_or("每日用量缺少 totals")?;
        let credits = match raw.get("credits").filter(|v| !v.is_null()) {
            None => None,
            Some(v) => Some(
                v.as_f64()
                    .filter(|n| n.is_finite() && *n >= 0.0 && *n <= 1e15)
                    .ok_or("Credits 字段无效")?,
            ),
        };
        let uncached = integer(raw, "uncached_text_input_tokens")?;
        let cached = integer(raw, "cached_text_input_tokens")?;
        let output = integer(raw, "text_output_tokens")?;
        let total = integer(raw, "text_total_tokens")?
            .or_else(|| Some(uncached?.checked_add(cached?)?.checked_add(output?)?));
        days.push(AnalyticsDay {
            date: date.into(),
            captured_at,
            totals: AnalyticsTotals {
                credits,
                turns: integer(raw, "turns")?,
                uncached_input_tokens: uncached,
                cached_input_tokens: cached,
                output_tokens: output,
                total_tokens: total,
            },
        });
    }
    days.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(days)
}

pub(crate) fn read(account_id: &str, start: &str, end: &str) -> Result<AnalyticsReport, String> {
    validate_range(start, end)?;
    let storage = crate::storage_helpers::open_storage().ok_or("storage unavailable")?;
    storage
        .find_account_by_id(account_id)
        .map_err(|e| e.to_string())?
        .ok_or("账号不存在")?;
    let rate = storage
        .get_app_setting(RATE_KEY)
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 1000.0);
    Ok(AnalyticsReport {
        account_id: account_id.into(),
        source: "chatgpt-wham",
        start_date: start.into(),
        end_date: end.into(),
        days: storage
            .read_usage_analytics(account_id, start, end)
            .map_err(|e| e.to_string())?,
        sync: storage
            .usage_analytics_sync(account_id)
            .map_err(|e| e.to_string())?,
        usd_per_credit: rate,
        pricing: pricing::read(&storage)?,
    })
}

pub(crate) fn set_rate(rate: Option<f64>) -> Result<(), String> {
    if rate.is_some_and(|r| !r.is_finite() || !(0.0..=1000.0).contains(&r)) {
        return Err("折算系数必须在 0 到 1000 之间".into());
    }
    let storage = crate::storage_helpers::open_storage().ok_or("storage unavailable")?;
    match rate {
        Some(rate) => storage.set_app_setting(RATE_KEY, &rate.to_string(), now_ts()),
        None => storage.delete_app_setting(RATE_KEY),
    }
    .map_err(|e| e.to_string())
}

pub(crate) fn refresh(account_id: &str, start: &str, end: &str) -> Result<AnalyticsReport, String> {
    validate_range(start, end)?;
    let _guard = REFRESH_LOCK
        .try_lock()
        .map_err(|_| "另一个用量同步正在进行，请稍后刷新")?;
    // The active CLI owns refresh-token rotation. Only reconcile its credentials;
    // an analytics failure must not change account availability or refresh tokens.
    crate::codex_profile::sync_active_profile_credentials_for_account(account_id)?;
    let mut storage = crate::storage_helpers::open_storage().ok_or("storage unavailable")?;
    let account = storage
        .find_account_by_id(account_id)
        .map_err(|e| e.to_string())?
        .ok_or("账号不存在")?;
    let captured_at = now_ts();
    let result = (|| {
        let token = storage
            .find_token_by_account_id(account_id)
            .map_err(|e| e.to_string())?
            .ok_or("账号缺少凭据，请重新登录")?;
        let workspace = crate::usage_account_meta::workspace_header_for_account(&account)
            .ok_or("账号缺少工作区标识，请先刷新账号额度")?;
        let base = std::env::var("CODEXMANAGER_USAGE_BASE_URL")
            .unwrap_or_else(|_| "https://chatgpt.com".into());
        let value = crate::usage_http::fetch_daily_analytics(
            &base,
            &token.access_token,
            &workspace,
            start,
            end,
        )?;
        parse_days(&value, start, end, captured_at)
    })();
    match result {
        Ok(days) => storage
            .save_usage_analytics(account_id, start, end, &days, captured_at)
            .map_err(|e| e.to_string())?,
        Err(error) => {
            storage
                .record_usage_analytics_failure(account_id, captured_at, &error)
                .map_err(|e| e.to_string())?;
            return Err(error);
        }
    }
    drop(storage);
    read(account_id, start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn validates_and_preserves_missing_metrics() {
        let data = json!({"data":[{"date":"2026-01-02","totals":{"credits":1.5,"uncached_text_input_tokens":10,"cached_text_input_tokens":20,"text_output_tokens":3}}]});
        let days = parse_days(&data, "2026-01-01", "2026-01-03", 1).unwrap();
        assert_eq!(days[0].totals.total_tokens, Some(33));
        assert_eq!(days[0].totals.turns, None);
        assert!(parse_days(&json!({}), "2026-01-01", "2026-01-03", 1).is_err());
        assert!(parse_days(
            &json!({"data":[data["data"][0], data["data"][0]]}),
            "2026-01-01",
            "2026-01-03",
            1
        )
        .is_err());
        assert!(parse_days(&data, "2026-01-01", "2026-01-02", 1).is_err());
        assert!(parse_days(
            &json!({"data":[{"date":"2026-01-02","totals":{"credits":-1}}]}),
            "2026-01-01",
            "2026-01-03",
            1
        )
        .is_err());
        let zero = json!({"data":[{"date":"2026-01-02","totals":{"text_total_tokens":0}}]});
        assert_eq!(
            parse_days(&zero, "2026-01-01", "2026-01-03", 1).unwrap()[0]
                .totals
                .total_tokens,
            Some(0)
        );
    }
    #[test]
    fn bounds_dates() {
        assert!(validate_range("2026-01-01", "2026-01-02").is_ok());
        assert!(validate_range("2026-01-02", "2026-01-02").is_err());
        assert!(validate_range("2026-1-1", "2026-01-02").is_err());
        assert!(validate_range("2020-01-01", "2026-01-02").is_err());
    }

    #[test]
    fn rejects_changed_units_and_negative_tokens() {
        assert!(parse_days(
            &json!({"balance_unit":"usd", "data":[]}),
            "2026-01-01",
            "2026-01-02",
            1
        )
        .is_err());
        assert!(parse_days(
            &json!({"data":[{"date":"2026-01-01", "totals":{"text_total_tokens":-1}}]}),
            "2026-01-01",
            "2026-01-02",
            1
        )
        .is_err());
    }
}
