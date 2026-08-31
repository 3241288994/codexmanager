ALTER TABLE aggregate_apis ADD COLUMN balance_query_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE aggregate_apis ADD COLUMN balance_query_template TEXT;
ALTER TABLE aggregate_apis ADD COLUMN balance_query_base_url TEXT;
ALTER TABLE aggregate_apis ADD COLUMN balance_query_user_id TEXT;
ALTER TABLE aggregate_apis ADD COLUMN balance_query_config_json TEXT;
ALTER TABLE aggregate_apis ADD COLUMN last_balance_at INTEGER;
ALTER TABLE aggregate_apis ADD COLUMN last_balance_status TEXT;
ALTER TABLE aggregate_apis ADD COLUMN last_balance_error TEXT;
ALTER TABLE aggregate_apis ADD COLUMN last_balance_json TEXT;

CREATE TABLE IF NOT EXISTS aggregate_api_balance_secrets (
  aggregate_api_id TEXT PRIMARY KEY REFERENCES aggregate_apis(id) ON DELETE CASCADE,
  access_token TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_aggregate_api_balance_secrets_updated_at
  ON aggregate_api_balance_secrets(updated_at);
