CREATE TABLE IF NOT EXISTS model_source_models (
  source_kind TEXT NOT NULL,
  source_id TEXT NOT NULL,
  upstream_model TEXT NOT NULL,
  display_name TEXT,
  status TEXT NOT NULL DEFAULT 'available',
  discovery_kind TEXT NOT NULL DEFAULT 'synced',
  last_synced_at INTEGER,
  extra_json TEXT NOT NULL DEFAULT '{}',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (source_kind, source_id, upstream_model)
);

CREATE INDEX IF NOT EXISTS idx_model_source_models_source
  ON model_source_models(source_kind, source_id);

CREATE INDEX IF NOT EXISTS idx_model_source_models_upstream_model
  ON model_source_models(upstream_model);

CREATE TABLE IF NOT EXISTS model_source_mappings (
  id TEXT PRIMARY KEY,
  platform_model_slug TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_id TEXT NOT NULL,
  upstream_model TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 0,
  weight INTEGER NOT NULL DEFAULT 1,
  billing_model_slug TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(platform_model_slug, source_kind, source_id, upstream_model)
);

CREATE INDEX IF NOT EXISTS idx_model_source_mappings_platform
  ON model_source_mappings(platform_model_slug, enabled, priority DESC);

CREATE INDEX IF NOT EXISTS idx_model_source_mappings_source
  ON model_source_mappings(source_kind, source_id, enabled);

INSERT OR IGNORE INTO model_source_models (
  source_kind, source_id, upstream_model, display_name, status, discovery_kind,
  last_synced_at, extra_json, created_at, updated_at
)
SELECT
  source_kind,
  source_id,
  model_slug,
  model_slug,
  'available',
  'legacy',
  updated_at,
  '{}',
  updated_at,
  updated_at
FROM quota_source_model_assignments
WHERE TRIM(model_slug) <> '';

INSERT OR IGNORE INTO model_source_mappings (
  id, platform_model_slug, source_kind, source_id, upstream_model,
  enabled, priority, weight, billing_model_slug, created_at, updated_at
)
SELECT
  'msm_' || lower(hex(randomblob(8))),
  model_slug,
  source_kind,
  source_id,
  model_slug,
  1,
  0,
  1,
  NULL,
  updated_at,
  updated_at
FROM quota_source_model_assignments
WHERE TRIM(model_slug) <> '';
