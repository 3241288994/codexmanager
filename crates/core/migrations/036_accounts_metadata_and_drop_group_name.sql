PRAGMA foreign_keys = OFF;

BEGIN TRANSACTION;

DROP INDEX IF EXISTS idx_accounts_status_sort_updated_at;
DROP INDEX IF EXISTS idx_accounts_group_name_sort_updated_at;
DROP INDEX IF EXISTS idx_accounts_sort_updated_at;

CREATE TABLE accounts_new (
  id TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  issuer TEXT NOT NULL,
  chatgpt_account_id TEXT,
  workspace_id TEXT,
  sort INTEGER DEFAULT 0,
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

INSERT INTO accounts_new (
  id,
  label,
  issuer,
  chatgpt_account_id,
  workspace_id,
  sort,
  status,
  created_at,
  updated_at
)
SELECT
  id,
  label,
  issuer,
  chatgpt_account_id,
  workspace_id,
  sort,
  status,
  created_at,
  updated_at
FROM accounts;

DROP TABLE accounts;
ALTER TABLE accounts_new RENAME TO accounts;

CREATE INDEX IF NOT EXISTS idx_accounts_sort_updated_at
  ON accounts(sort ASC, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_accounts_status_sort_updated_at
  ON accounts(status, sort ASC, updated_at DESC);

CREATE TABLE login_sessions_new (
  login_id TEXT PRIMARY KEY,
  code_verifier TEXT NOT NULL,
  state TEXT NOT NULL,
  status TEXT NOT NULL,
  error TEXT,
  workspace_id TEXT,
  note TEXT,
  tags TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

INSERT INTO login_sessions_new (
  login_id,
  code_verifier,
  state,
  status,
  error,
  workspace_id,
  note,
  tags,
  created_at,
  updated_at
)
SELECT
  login_id,
  code_verifier,
  state,
  status,
  error,
  workspace_id,
  note,
  tags,
  created_at,
  updated_at
FROM login_sessions;

DROP TABLE login_sessions;
ALTER TABLE login_sessions_new RENAME TO login_sessions;

CREATE TABLE IF NOT EXISTS account_metadata (
  account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
  note TEXT,
  tags TEXT,
  updated_at INTEGER NOT NULL
);

COMMIT;

PRAGMA foreign_keys = ON;
