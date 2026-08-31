use std::collections::BTreeSet;

use rusqlite::{params, Result, Row};

use super::{
    now_ts, AccountQuotaCapacityOverride, AccountQuotaCapacityTemplate, QuotaSourceModelAssignment,
    Storage,
};

impl Storage {
    pub fn list_quota_source_model_assignments(&self) -> Result<Vec<QuotaSourceModelAssignment>> {
        let mut stmt = self.conn.prepare(
            "SELECT source_kind, source_id, model_slug, updated_at
             FROM quota_source_model_assignments
             ORDER BY source_kind ASC, source_id ASC, model_slug ASC",
        )?;
        let rows = stmt.query_map([], map_quota_source_model_assignment_row)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn list_quota_source_model_assignments_for(
        &self,
        source_kind: &str,
        source_id: &str,
    ) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT model_slug
             FROM quota_source_model_assignments
             WHERE source_kind = ?1 AND source_id = ?2
             ORDER BY model_slug ASC",
        )?;
        let rows = stmt.query_map(params![source_kind, source_id], |row| row.get(0))?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn set_quota_source_model_assignments(
        &mut self,
        source_kind: &str,
        source_id: &str,
        model_slugs: &[String],
    ) -> Result<()> {
        let source_kind = normalize_required_text(source_kind);
        let source_id = normalize_required_text(source_id);
        if source_kind.is_empty() || source_id.is_empty() {
            return Ok(());
        }

        let now = now_ts();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM quota_source_model_assignments
             WHERE source_kind = ?1 AND source_id = ?2",
            params![source_kind, source_id],
        )?;
        for model_slug in normalize_model_slugs(model_slugs) {
            tx.execute(
                "INSERT INTO quota_source_model_assignments (
                    source_kind, source_id, model_slug, updated_at
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![source_kind, source_id, model_slug, now],
            )?;
        }
        tx.commit()
    }

    pub fn list_account_quota_capacity_templates(
        &self,
    ) -> Result<Vec<AccountQuotaCapacityTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT plan_type, primary_window_tokens, secondary_window_tokens, updated_at
             FROM account_quota_capacity_templates
             ORDER BY plan_type ASC",
        )?;
        let rows = stmt.query_map([], map_account_quota_capacity_template_row)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn upsert_account_quota_capacity_template(
        &self,
        plan_type: &str,
        primary_window_tokens: Option<i64>,
        secondary_window_tokens: Option<i64>,
    ) -> Result<()> {
        let plan_type = normalize_required_text(plan_type).to_ascii_lowercase();
        if plan_type.is_empty() {
            return Ok(());
        }
        self.conn.execute(
            "INSERT INTO account_quota_capacity_templates (
                plan_type, primary_window_tokens, secondary_window_tokens, updated_at
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(plan_type) DO UPDATE SET
                primary_window_tokens = excluded.primary_window_tokens,
                secondary_window_tokens = excluded.secondary_window_tokens,
                updated_at = excluded.updated_at",
            params![
                plan_type,
                positive_tokens(primary_window_tokens),
                positive_tokens(secondary_window_tokens),
                now_ts()
            ],
        )?;
        Ok(())
    }

    pub fn list_account_quota_capacity_overrides(
        &self,
    ) -> Result<Vec<AccountQuotaCapacityOverride>> {
        let mut stmt = self.conn.prepare(
            "SELECT account_id, primary_window_tokens, secondary_window_tokens, updated_at
             FROM account_quota_capacity_overrides
             ORDER BY account_id ASC",
        )?;
        let rows = stmt.query_map([], map_account_quota_capacity_override_row)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn upsert_account_quota_capacity_override(
        &self,
        account_id: &str,
        primary_window_tokens: Option<i64>,
        secondary_window_tokens: Option<i64>,
    ) -> Result<()> {
        let account_id = normalize_required_text(account_id);
        if account_id.is_empty() {
            return Ok(());
        }
        let primary = positive_tokens(primary_window_tokens);
        let secondary = positive_tokens(secondary_window_tokens);
        if primary.is_none() && secondary.is_none() {
            self.conn.execute(
                "DELETE FROM account_quota_capacity_overrides WHERE account_id = ?1",
                [account_id],
            )?;
            return Ok(());
        }
        self.conn.execute(
            "INSERT INTO account_quota_capacity_overrides (
                account_id, primary_window_tokens, secondary_window_tokens, updated_at
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(account_id) DO UPDATE SET
                primary_window_tokens = excluded.primary_window_tokens,
                secondary_window_tokens = excluded.secondary_window_tokens,
                updated_at = excluded.updated_at",
            params![account_id, primary, secondary, now_ts()],
        )?;
        Ok(())
    }

    pub(super) fn ensure_quota_pool_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS quota_source_model_assignments (
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                model_slug TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (source_kind, source_id, model_slug)
            );
            CREATE INDEX IF NOT EXISTS idx_quota_source_model_assignments_source
                ON quota_source_model_assignments(source_kind, source_id);
            CREATE INDEX IF NOT EXISTS idx_quota_source_model_assignments_model
                ON quota_source_model_assignments(model_slug, source_kind, source_id);
            CREATE TABLE IF NOT EXISTS account_quota_capacity_templates (
                plan_type TEXT PRIMARY KEY,
                primary_window_tokens INTEGER,
                secondary_window_tokens INTEGER,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS account_quota_capacity_overrides (
                account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
                primary_window_tokens INTEGER,
                secondary_window_tokens INTEGER,
                updated_at INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }
}

fn map_quota_source_model_assignment_row(row: &Row<'_>) -> Result<QuotaSourceModelAssignment> {
    Ok(QuotaSourceModelAssignment {
        source_kind: row.get(0)?,
        source_id: row.get(1)?,
        model_slug: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn map_account_quota_capacity_template_row(row: &Row<'_>) -> Result<AccountQuotaCapacityTemplate> {
    Ok(AccountQuotaCapacityTemplate {
        plan_type: row.get(0)?,
        primary_window_tokens: row.get(1)?,
        secondary_window_tokens: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn map_account_quota_capacity_override_row(row: &Row<'_>) -> Result<AccountQuotaCapacityOverride> {
    Ok(AccountQuotaCapacityOverride {
        account_id: row.get(0)?,
        primary_window_tokens: row.get(1)?,
        secondary_window_tokens: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn normalize_required_text(value: &str) -> String {
    value.trim().to_string()
}

fn normalize_model_slugs(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn positive_tokens(value: Option<i64>) -> Option<i64> {
    value.filter(|tokens| *tokens > 0)
}
