CREATE TABLE IF NOT EXISTS model_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    sort INTEGER NOT NULL DEFAULT 0,
    is_default INTEGER NOT NULL DEFAULT 0,
    rate_multiplier_millis INTEGER NOT NULL DEFAULT 1000,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_model_groups_status_sort
    ON model_groups(status, sort, name);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_groups_default
    ON model_groups(is_default)
    WHERE is_default = 1;

CREATE TABLE IF NOT EXISTS model_group_models (
    group_id TEXT NOT NULL REFERENCES model_groups(id) ON DELETE CASCADE,
    platform_model_slug TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    rate_multiplier_millis INTEGER,
    billing_model_slug TEXT,
    note TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (group_id, platform_model_slug)
);

CREATE INDEX IF NOT EXISTS idx_model_group_models_model
    ON model_group_models(platform_model_slug, enabled);

CREATE TABLE IF NOT EXISTS user_model_groups (
    user_id TEXT NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    group_id TEXT NOT NULL REFERENCES model_groups(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    expires_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, group_id)
);

CREATE INDEX IF NOT EXISTS idx_user_model_groups_user_status
    ON user_model_groups(user_id, status, expires_at);

INSERT OR IGNORE INTO model_groups (
    id, name, description, status, sort, is_default, rate_multiplier_millis, created_at, updated_at
)
VALUES (
    'mg_default',
    '默认模型组',
    '升级前已有用户的默认可用模型集合',
    'active',
    0,
    1,
    1000,
    strftime('%s', 'now'),
    strftime('%s', 'now')
);

INSERT OR IGNORE INTO model_group_models (
    group_id, platform_model_slug, enabled, rate_multiplier_millis,
    billing_model_slug, note, created_at, updated_at
)
SELECT
    'mg_default',
    slug,
    1,
    NULL,
    NULL,
    'migration',
    strftime('%s', 'now'),
    strftime('%s', 'now')
FROM model_catalog_models
WHERE scope = 'default'
  AND COALESCE(supported_in_api, 1) = 1
  AND TRIM(slug) <> '';

INSERT OR IGNORE INTO user_model_groups (
    user_id, group_id, status, expires_at, created_at, updated_at
)
SELECT
    id,
    'mg_default',
    'active',
    NULL,
    strftime('%s', 'now'),
    strftime('%s', 'now')
FROM app_users
WHERE role = 'member'
  AND status = 'active';
