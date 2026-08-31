CREATE TABLE IF NOT EXISTS api_key_quota_limits (
  key_id TEXT PRIMARY KEY REFERENCES api_keys(id) ON DELETE CASCADE,
  quota_limit_tokens INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_api_key_quota_limits_updated_at
  ON api_key_quota_limits(updated_at DESC);
