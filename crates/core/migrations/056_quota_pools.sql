CREATE TABLE IF NOT EXISTS quota_source_model_assignments (
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
);
