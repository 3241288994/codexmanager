CREATE TABLE IF NOT EXISTS account_subscriptions (
  account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
  has_subscription INTEGER NOT NULL DEFAULT 0,
  plan_type TEXT,
  expires_at INTEGER,
  renews_at INTEGER,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_account_subscriptions_updated_at
  ON account_subscriptions(updated_at DESC, account_id ASC);
