use super::Storage;
use rusqlite::{params, OptionalExtension, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsTotals {
    pub credits: Option<f64>,
    pub turns: Option<u64>,
    pub uncached_input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsDay {
    pub date: String,
    pub totals: AnalyticsTotals,
    pub captured_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsSync {
    pub attempted_at: i64,
    pub succeeded_at: Option<i64>,
    pub error: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

impl Storage {
    /// A refresh replaces its entire half-open date range atomically. Dates omitted
    /// by the upstream remain absent, rather than retaining obsolete totals.
    pub fn save_usage_analytics(
        &mut self,
        account_id: &str,
        start: &str,
        end: &str,
        days: &[AnalyticsDay],
        captured_at: i64,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM usage_analytics_daily WHERE account_id = ?1 AND day >= ?2 AND day < ?3",
            params![account_id, start, end],
        )?;
        for day in days {
            tx.execute("INSERT INTO usage_analytics_daily(account_id, day, totals_json, captured_at) VALUES (?1, ?2, ?3, ?4)",
                params![account_id, day.date, serde_json::to_string(&day.totals).expect("finite totals"), captured_at])?;
        }
        tx.execute("INSERT INTO usage_analytics_sync(account_id, attempted_at, succeeded_at, start_date, end_date) VALUES (?1, ?2, ?2, ?3, ?4) ON CONFLICT(account_id) DO UPDATE SET attempted_at=excluded.attempted_at, succeeded_at=excluded.succeeded_at, error=NULL, start_date=excluded.start_date, end_date=excluded.end_date", params![account_id, captured_at, start, end])?;
        tx.execute("INSERT INTO usage_analytics_snapshots(account_id, captured_at, start_date, end_date, report_json) VALUES (?1, ?2, ?3, ?4, ?5)", params![account_id, captured_at, start, end, serde_json::to_string(days).expect("finite days")])?;
        tx.execute("DELETE FROM usage_analytics_snapshots WHERE account_id=?1 AND id NOT IN (SELECT id FROM usage_analytics_snapshots WHERE account_id=?1 ORDER BY id DESC LIMIT 180)", [account_id])?;
        tx.commit()
    }

    pub fn record_usage_analytics_failure(
        &self,
        account_id: &str,
        attempted_at: i64,
        error: &str,
    ) -> Result<()> {
        self.conn.execute("INSERT INTO usage_analytics_sync(account_id, attempted_at, error) VALUES (?1, ?2, ?3) ON CONFLICT(account_id) DO UPDATE SET attempted_at=excluded.attempted_at, error=excluded.error", params![account_id, attempted_at, error])?;
        Ok(())
    }

    pub fn read_usage_analytics(
        &self,
        account_id: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<AnalyticsDay>> {
        let mut stmt = self.conn.prepare("SELECT day, totals_json, captured_at FROM usage_analytics_daily WHERE account_id=?1 AND day>=?2 AND day<?3 ORDER BY day")?;
        let rows = stmt.query_map(params![account_id, start, end], |r| {
            let raw: String = r.get(1)?;
            let totals = serde_json::from_str(&raw).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(AnalyticsDay {
                date: r.get(0)?,
                totals,
                captured_at: r.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn usage_analytics_sync(&self, account_id: &str) -> Result<Option<AnalyticsSync>> {
        self.conn.query_row("SELECT attempted_at, succeeded_at, error, start_date, end_date FROM usage_analytics_sync WHERE account_id=?1", [account_id], |r| Ok(AnalyticsSync {
            attempted_at:r.get(0)?, succeeded_at:r.get(1)?, error:r.get(2)?, start_date:r.get(3)?, end_date:r.get(4)?,
        })).optional()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rollback_and_snapshot_retention() {
        let mut db = Storage::open_in_memory().unwrap();
        db.init().unwrap();
        let day = AnalyticsDay {
            date: "2026-01-01".into(),
            totals: AnalyticsTotals::default(),
            captured_at: 1,
        };
        db.save_usage_analytics("a", "2026-01-01", "2026-01-02", &[day.clone()], 1)
            .unwrap();
        assert!(db
            .save_usage_analytics(
                "a",
                "2026-01-01",
                "2026-01-02",
                &[day.clone(), day.clone()],
                2
            )
            .is_err());
        assert_eq!(
            db.read_usage_analytics("a", "2026-01-01", "2026-01-02")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            db.usage_analytics_sync("a").unwrap().unwrap().succeeded_at,
            Some(1)
        );
        for n in 2..=182 {
            db.save_usage_analytics("a", "2026-01-01", "2026-01-02", &[day.clone()], n)
                .unwrap();
        }
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM usage_analytics_snapshots", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 180);
    }
    #[test]
    fn refresh_replaces_range_isolates_accounts_and_preserves_data_on_failure() {
        let mut db = Storage::open_in_memory().unwrap();
        db.init().unwrap();
        let days = vec![AnalyticsDay {
            date: "2026-09-18".into(),
            totals: AnalyticsTotals {
                credits: Some(2.0),
                ..Default::default()
            },
            captured_at: 1,
        }];
        db.save_usage_analytics("a", "2026-09-18", "2026-09-20", &days, 1)
            .unwrap();
        db.save_usage_analytics("b", "2026-09-18", "2026-09-20", &days, 1)
            .unwrap();
        db.record_usage_analytics_failure("a", 2, "denied").unwrap();
        assert_eq!(
            db.read_usage_analytics("a", "2026-09-18", "2026-09-20")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            db.usage_analytics_sync("a").unwrap().unwrap().succeeded_at,
            Some(1)
        );
        db.save_usage_analytics("a", "2026-09-18", "2026-09-20", &[], 3)
            .unwrap();
        assert!(db
            .read_usage_analytics("a", "2026-09-18", "2026-09-20")
            .unwrap()
            .is_empty());
        assert_eq!(
            db.read_usage_analytics("b", "2026-09-18", "2026-09-20")
                .unwrap()
                .len(),
            1
        );
        assert!(db
            .usage_analytics_sync("a")
            .unwrap()
            .unwrap()
            .error
            .is_none());
    }
}
