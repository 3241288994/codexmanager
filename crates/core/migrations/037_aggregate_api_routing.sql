ALTER TABLE api_keys ADD COLUMN rotation_strategy TEXT NOT NULL DEFAULT 'account_rotation';
ALTER TABLE api_keys ADD COLUMN aggregate_api_id TEXT;

CREATE TABLE IF NOT EXISTS aggregate_apis (
  id TEXT PRIMARY KEY,
  provider_type TEXT NOT NULL DEFAULT 'codex',
  supplier_name TEXT,
  sort INTEGER NOT NULL DEFAULT 0,
  url TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  last_test_at INTEGER,
  last_test_status TEXT,
  last_test_error TEXT
);

CREATE INDEX IF NOT EXISTS idx_aggregate_apis_created_at
  ON aggregate_apis(created_at DESC);

CREATE TABLE IF NOT EXISTS aggregate_api_secrets (
  aggregate_api_id TEXT PRIMARY KEY REFERENCES aggregate_apis(id) ON DELETE CASCADE,
  secret_value TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_aggregate_api_secrets_updated_at
  ON aggregate_api_secrets(updated_at);
