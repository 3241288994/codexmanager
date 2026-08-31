use rusqlite::{params, Result};

use super::{ModelPriceRule, Storage};

impl Storage {
    pub fn upsert_model_price_rule(&self, rule: &ModelPriceRule) -> Result<()> {
        self.conn.execute(
            "INSERT INTO model_price_rules (
                id, provider, model_pattern, match_type, billing_mode,
                currency, unit, input_price_per_1m, cached_input_price_per_1m,
                output_price_per_1m, reasoning_output_price_per_1m,
                cache_write_5m_price_per_1m, cache_write_1h_price_per_1m,
                cache_hit_price_per_1m, long_context_threshold_tokens,
                long_context_input_price_per_1m,
                long_context_cached_input_price_per_1m,
                long_context_output_price_per_1m, source, source_url,
                seed_version, enabled, priority, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9,
                ?10, ?11,
                ?12, ?13,
                ?14, ?15,
                ?16,
                ?17,
                ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25
             )
             ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                model_pattern = excluded.model_pattern,
                match_type = excluded.match_type,
                billing_mode = excluded.billing_mode,
                currency = excluded.currency,
                unit = excluded.unit,
                input_price_per_1m = excluded.input_price_per_1m,
                cached_input_price_per_1m = excluded.cached_input_price_per_1m,
                output_price_per_1m = excluded.output_price_per_1m,
                reasoning_output_price_per_1m = excluded.reasoning_output_price_per_1m,
                cache_write_5m_price_per_1m = excluded.cache_write_5m_price_per_1m,
                cache_write_1h_price_per_1m = excluded.cache_write_1h_price_per_1m,
                cache_hit_price_per_1m = excluded.cache_hit_price_per_1m,
                long_context_threshold_tokens = excluded.long_context_threshold_tokens,
                long_context_input_price_per_1m = excluded.long_context_input_price_per_1m,
                long_context_cached_input_price_per_1m = excluded.long_context_cached_input_price_per_1m,
                long_context_output_price_per_1m = excluded.long_context_output_price_per_1m,
                source = excluded.source,
                source_url = excluded.source_url,
                seed_version = excluded.seed_version,
                enabled = excluded.enabled,
                priority = excluded.priority,
                updated_at = excluded.updated_at",
            params![
                &rule.id,
                &rule.provider,
                &rule.model_pattern,
                &rule.match_type,
                &rule.billing_mode,
                &rule.currency,
                &rule.unit,
                rule.input_price_per_1m,
                rule.cached_input_price_per_1m,
                rule.output_price_per_1m,
                rule.reasoning_output_price_per_1m,
                rule.cache_write_5m_price_per_1m,
                rule.cache_write_1h_price_per_1m,
                rule.cache_hit_price_per_1m,
                rule.long_context_threshold_tokens,
                rule.long_context_input_price_per_1m,
                rule.long_context_cached_input_price_per_1m,
                rule.long_context_output_price_per_1m,
                &rule.source,
                &rule.source_url,
                &rule.seed_version,
                if rule.enabled { 1_i64 } else { 0_i64 },
                rule.priority,
                rule.created_at,
                rule.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn count_model_price_rules_for_seed(&self, seed_version: &str) -> Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(1)
             FROM model_price_rules
             WHERE source = 'official_seed' AND seed_version = ?1",
            [seed_version],
            |row| row.get(0),
        )
    }

    pub fn list_enabled_model_price_rules(&self) -> Result<Vec<ModelPriceRule>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                id, provider, model_pattern, match_type, billing_mode,
                currency, unit, input_price_per_1m, cached_input_price_per_1m,
                output_price_per_1m, reasoning_output_price_per_1m,
                cache_write_5m_price_per_1m, cache_write_1h_price_per_1m,
                cache_hit_price_per_1m, long_context_threshold_tokens,
                long_context_input_price_per_1m,
                long_context_cached_input_price_per_1m,
                long_context_output_price_per_1m, source, source_url,
                seed_version, enabled, priority, created_at, updated_at
             FROM model_price_rules
             WHERE enabled = 1
             ORDER BY priority DESC, length(model_pattern) DESC, model_pattern ASC",
        )?;
        let mut rows = stmt.query([])?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(ModelPriceRule {
                id: row.get(0)?,
                provider: row.get(1)?,
                model_pattern: row.get(2)?,
                match_type: row.get(3)?,
                billing_mode: row.get(4)?,
                currency: row.get(5)?,
                unit: row.get(6)?,
                input_price_per_1m: row.get(7)?,
                cached_input_price_per_1m: row.get(8)?,
                output_price_per_1m: row.get(9)?,
                reasoning_output_price_per_1m: row.get(10)?,
                cache_write_5m_price_per_1m: row.get(11)?,
                cache_write_1h_price_per_1m: row.get(12)?,
                cache_hit_price_per_1m: row.get(13)?,
                long_context_threshold_tokens: row.get(14)?,
                long_context_input_price_per_1m: row.get(15)?,
                long_context_cached_input_price_per_1m: row.get(16)?,
                long_context_output_price_per_1m: row.get(17)?,
                source: row.get(18)?,
                source_url: row.get(19)?,
                seed_version: row.get(20)?,
                enabled: row.get(21)?,
                priority: row.get(22)?,
                created_at: row.get(23)?,
                updated_at: row.get(24)?,
            });
        }
        Ok(items)
    }

    pub(super) fn ensure_model_price_rules_table(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS model_price_rules (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                model_pattern TEXT NOT NULL,
                match_type TEXT NOT NULL,
                billing_mode TEXT NOT NULL,
                currency TEXT NOT NULL,
                unit TEXT NOT NULL,
                input_price_per_1m REAL,
                cached_input_price_per_1m REAL,
                output_price_per_1m REAL,
                reasoning_output_price_per_1m REAL,
                cache_write_5m_price_per_1m REAL,
                cache_write_1h_price_per_1m REAL,
                cache_hit_price_per_1m REAL,
                long_context_threshold_tokens INTEGER,
                long_context_input_price_per_1m REAL,
                long_context_cached_input_price_per_1m REAL,
                long_context_output_price_per_1m REAL,
                source TEXT NOT NULL,
                source_url TEXT,
                seed_version TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_model_price_rules_provider_model_mode
             ON model_price_rules(provider, model_pattern, billing_mode)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_model_price_rules_source_seed
             ON model_price_rules(source, seed_version)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_model_price_rules_enabled_priority
             ON model_price_rules(enabled, priority DESC)",
            [],
        )?;
        Ok(())
    }
}
