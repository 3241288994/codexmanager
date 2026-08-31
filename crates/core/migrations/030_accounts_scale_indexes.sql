CREATE INDEX IF NOT EXISTS idx_accounts_status_sort_updated_at
  ON accounts(status, sort ASC, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_accounts_group_name_sort_updated_at
  ON accounts(group_name, sort ASC, updated_at DESC);
