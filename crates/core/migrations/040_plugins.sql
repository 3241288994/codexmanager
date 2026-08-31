CREATE TABLE IF NOT EXISTS plugin_installs (
    plugin_id TEXT PRIMARY KEY,
    source_url TEXT,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    author TEXT,
    homepage_url TEXT,
    script_url TEXT,
    script_body TEXT NOT NULL,
    permissions_json TEXT NOT NULL DEFAULT '[]',
    manifest_json TEXT NOT NULL,
    status TEXT NOT NULL,
    installed_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_run_at INTEGER,
    last_error TEXT
);

CREATE TABLE IF NOT EXISTS plugin_tasks (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    entrypoint TEXT NOT NULL,
    schedule_kind TEXT NOT NULL,
    interval_seconds INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,
    next_run_at INTEGER,
    last_run_at INTEGER,
    last_status TEXT,
    last_error TEXT,
    task_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (plugin_id) REFERENCES plugin_installs(plugin_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_plugin_tasks_plugin_id_enabled_next_run_at
    ON plugin_tasks(plugin_id, enabled, next_run_at);

CREATE TABLE IF NOT EXISTS plugin_run_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id TEXT NOT NULL,
    task_id TEXT,
    run_type TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    duration_ms INTEGER,
    output_json TEXT,
    error TEXT
);

CREATE INDEX IF NOT EXISTS idx_plugin_run_logs_created_at
    ON plugin_run_logs(started_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_plugin_run_logs_plugin_id_created_at
    ON plugin_run_logs(plugin_id, started_at DESC, id DESC);
