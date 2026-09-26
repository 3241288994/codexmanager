//! A reference valuation, not a credit purchase price or an invoice.
use codexmanager_core::storage::{now_ts, Storage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Mutex};

pub(crate) const API_SOURCE: &str = "https://developers.openai.com/api/docs/pricing";
pub(crate) const CREDIT_SOURCE: &str = "https://learn.chatgpt.com/docs/pricing";
const KEY: &str = "usage_analytics_official_reference_v1";
const DAY: i64 = 86400;
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PricingState {
    pub attempted_at: Option<i64>,
    pub snapshot: Option<PriceSnapshot>,
    pub error: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PriceSnapshot {
    pub fetched_at: i64,
    pub usd_per_credit: f64,
    pub api_source: String,
    pub credit_source: String,
    pub api_sha256: String,
    pub credit_sha256: String,
    pub evidence: Vec<PriceEvidence>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PriceEvidence {
    model: String,
    api_usd_per_million: [f64; 3],
    credits_per_million: [f64; 3],
}

pub(crate) fn read(storage: &Storage) -> Result<PricingState, String> {
    match storage.get_app_setting(KEY).map_err(|e| e.to_string())? {
        Some(raw) => serde_json::from_str(&raw).map_err(|_| "定价缓存损坏，请重新同步定价".into()),
        None => Ok(PricingState::default()),
    }
}

fn due(state: &PricingState, now: i64) -> bool {
    state
        .snapshot
        .as_ref()
        .is_none_or(|s| now - s.fetched_at >= DAY)
        && state.attempted_at.is_none_or(|t| now - t >= 3600)
}

pub(crate) fn refresh(force: bool) -> Result<PricingState, String> {
    let _guard = LOCK.try_lock().map_err(|_| "定价同步正在进行")?;
    let storage = crate::storage_helpers::open_storage().ok_or("storage unavailable")?;
    let mut state = read(&storage).unwrap_or_default();
    let now = now_ts();
    if !force && !due(&state, now) {
        return Ok(state);
    }
    state.attempted_at = Some(now);
    let result = (|| {
        let api = crate::usage_http::fetch_official_pricing(&format!("{API_SOURCE}.md"))?;
        let credits = crate::usage_http::fetch_official_pricing(&format!("{CREDIT_SOURCE}.md"))?;
        derive(&api, &credits, now)
    })();
    match result {
        Ok(snapshot) => {
            state.snapshot = Some(snapshot);
            state.error = None;
        }
        Err(error) => state.error = Some(error), // retain last validated snapshot
    }
    storage
        .set_app_setting(
            KEY,
            &serde_json::to_string(&state).map_err(|e| e.to_string())?,
            now,
        )
        .map_err(|e| e.to_string())?;
    Ok(state)
}

fn positive(s: &str) -> Result<f64, String> {
    s.trim()
        .trim_start_matches('$')
        .trim_end_matches("credits")
        .trim()
        .replace(',', "")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0 && *v < 1e9)
        .ok_or_else(|| "官方定价表包含无法识别的数值".into())
}

fn model(s: &str) -> String {
    s.trim().to_ascii_lowercase().replace(' ', "-")
}

fn derive(api: &str, credits: &str, now: i64) -> Result<PriceSnapshot, String> {
    let invalid = "官方定价表结构已变化，未覆盖已有参考价格";
    if !api.contains("Prices per 1M tokens") {
        return Err(invalid.into());
    }
    let section = api
        .split_once("### Standard pricing data")
        .ok_or(invalid)?
        .1
        .split("###")
        .next()
        .ok_or(invalid)?;
    if !section.contains("| Model | Short context input | Short context cached input | Short context cache writes | Short context output |") {
        return Err(invalid.into());
    }
    let mut prices = BTreeMap::new();
    for line in section.lines().filter(|l| l.starts_with("| gpt-")) {
        let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() != 9 {
            return Err(invalid.into());
        }
        if cells[2] == "-" {
            continue;
        }
        let name = cells[0].split(" (").next().unwrap_or(cells[0]);
        if prices
            .insert(
                model(name),
                [
                    positive(cells[1])?,
                    positive(cells[2])?,
                    positive(cells[4])?,
                ],
            )
            .is_some()
        {
            return Err(invalid.into());
        }
    }
    let table = credits
        .split_once("Credits per 1M tokens")
        .ok_or(invalid)?
        .1
        .split_once("<tbody>")
        .ok_or(invalid)?
        .1
        .split_once("</tbody>")
        .ok_or(invalid)?
        .0;
    let mut evidence = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for row in table.split("<tr>").skip(1) {
        let cells: Vec<_> = row
            .split("<td")
            .skip(1)
            .filter_map(|s| {
                s.split_once('>')
                    .and_then(|(_, s)| s.split_once("</td>").map(|(s, _)| s.trim()))
            })
            .collect();
        if cells.len() != 4 {
            return Err(invalid.into());
        }
        let name = model(cells[0]);
        let Some(price) = prices.get(&name) else {
            continue;
        };
        if !seen.insert(name.clone()) {
            return Err(invalid.into());
        }
        evidence.push(PriceEvidence {
            model: name,
            api_usd_per_million: *price,
            credits_per_million: [
                positive(cells[1])?,
                positive(cells[2])?,
                positive(cells[3])?,
            ],
        });
    }
    if evidence.len() < 3 {
        return Err("官方两张定价表可交叉核对的模型不足，保留旧价格".into());
    }
    // Median input ratio; independently check all three token categories for every
    // matched model. One-percent tolerance accommodates rounded credit tables.
    let mut ratios: Vec<_> = evidence
        .iter()
        .map(|e| e.api_usd_per_million[0] / e.credits_per_million[0])
        .collect();
    ratios.sort_by(f64::total_cmp);
    let rate = ratios[ratios.len() / 2];
    if evidence.iter().any(|e| {
        (0..3).any(|i| {
            ((e.api_usd_per_million[i] / e.credits_per_million[i]) / rate - 1.0).abs() > 0.01
        })
    }) {
        return Err("官方模型价格与 Credits 比例不再一致，无法推导统一参考系数；保留旧价格".into());
    }
    Ok(PriceSnapshot {
        fetched_at: now,
        usd_per_credit: rate,
        api_source: API_SOURCE.into(),
        credit_source: CREDIT_SOURCE.into(),
        api_sha256: format!("{:x}", Sha256::digest(api.as_bytes())),
        credit_sha256: format!("{:x}", Sha256::digest(credits.as_bytes())),
        evidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_table_fixture_includes_rounded_credit_rates() {
        let snapshot = derive(
            include_str!("fixtures/api-pricing-2026-09-19.md"),
            include_str!("fixtures/credit-pricing-2026-09-19.md"),
            1,
        )
        .unwrap();
        assert_eq!(snapshot.usd_per_credit, 0.04);
        assert_eq!(snapshot.evidence.len(), 7);
        assert!(snapshot.evidence.iter().any(|e| e.model == "gpt-5.4-mini"));
    }
    fn documents() -> (String, String) {
        let mut api = "Prices per 1M tokens\n### Standard pricing data\n| Model | Short context input | Short context cached input | Short context cache writes | Short context output | Long context input | Long context cached input | Long context cache writes | Long context output |\n".to_string();
        let mut credit = "Credits per 1M tokens<tbody>".to_string();
        for name in ["gpt-6-astra", "gpt-5.6-sol", "gpt-5.6-terra"] {
            api.push_str(&format!(
                "| {name} | $4 | $0.4 | $5 | $20 | $8 | $0.8 | $10 | $30 |\n"
            ));
            credit.push_str(&format!("<tr><td>{name}</td><td>100 credits</td><td>10 credits</td><td>500 credits</td></tr>"));
        }
        credit.push_str("</tbody>");
        api.push_str(
            "### Fast pricing data\n| gpt-6-astra | $99 | $99 | $99 | $99 | - | - | - | - |\n",
        );
        (api, credit)
    }
    #[test]
    fn derives_rate_from_standard_only_and_rejects_inconsistent_or_changed_tables() {
        let (api, credits) = documents();
        let result = derive(&api, &credits, 7).unwrap();
        assert_eq!(result.usd_per_credit, 0.04);
        assert_eq!(result.evidence.len(), 3);
        assert_eq!(result.api_sha256.len(), 64);
        assert!(derive(&api.replacen("$20", "$40", 1), &credits, 7).is_err());
        assert!(derive(&api, &credits.replace("100 credits", "NaN credits"), 7).is_err());
        assert!(derive(
            &api.replace("### Standard pricing data", "### Batch pricing data"),
            &credits,
            7
        )
        .is_err());
        assert!(derive("<html>blocked</html>", &credits, 7).is_err());
    }
    #[test]
    fn cache_survives_failure_and_retry_is_bounded() {
        let (api, credits) = documents();
        let mut state = PricingState {
            snapshot: Some(derive(&api, &credits, 100).unwrap()),
            ..Default::default()
        };
        assert!(!due(&state, 200));
        assert!(due(&state, 100 + DAY));
        state.attempted_at = Some(100 + DAY);
        state.error = Some("offline".into());
        assert!(!due(&state, 101 + DAY));
        assert!(due(&state, 3700 + DAY));
        let db = Storage::open_in_memory().unwrap();
        db.init().unwrap();
        db.set_app_setting(KEY, &serde_json::to_string(&state).unwrap(), 1)
            .unwrap();
        assert_eq!(read(&db).unwrap().snapshot.unwrap().usd_per_credit, 0.04);
        assert!(read(&db).unwrap().error.is_some());
    }
}
