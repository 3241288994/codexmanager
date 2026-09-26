CREATE TABLE IF NOT EXISTS usage_analytics_daily (
    account_id TEXT NOT NULL,
    day TEXT NOT NULL,
    totals_json TEXT NOT NULL,
    captured_at INTEGER NOT NULL,
    PRIMARY KEY (account_id, day)
);
CREATE TABLE IF NOT EXISTS usage_analytics_sync (
    account_id TEXT PRIMARY KEY,
    attempted_at INTEGER NOT NULL,
    succeeded_at INTEGER,
    error TEXT,
    start_date TEXT,
    end_date TEXT
);
CREATE TABLE IF NOT EXISTS usage_analytics_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    captured_at INTEGER NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    report_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS usage_analytics_snapshots_account ON usage_analytics_snapshots(account_id, id DESC);
