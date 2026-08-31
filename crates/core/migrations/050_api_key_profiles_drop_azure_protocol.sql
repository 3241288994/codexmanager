BEGIN TRANSACTION;
PRAGMA foreign_keys = OFF;

UPDATE api_keys
SET status = 'disabled'
WHERE id IN (
  SELECT key_id
  FROM api_key_profiles
  WHERE lower(replace(protocol_type, '-', '_')) IN ('azure', 'azure_openai')
);

CREATE TABLE api_key_profiles_new (
  key_id TEXT PRIMARY KEY REFERENCES api_keys(id) ON DELETE CASCADE,
  client_type TEXT NOT NULL CHECK (client_type IN ('codex', 'claude_code')),
  protocol_type TEXT NOT NULL CHECK (protocol_type IN ('openai_compat', 'anthropic_native')),
  auth_scheme TEXT NOT NULL CHECK (auth_scheme IN ('authorization_bearer', 'x_api_key')),
  upstream_base_url TEXT,
  static_headers_json TEXT,
  default_model TEXT,
  reasoning_effort TEXT,
  service_tier TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

INSERT INTO api_key_profiles_new (
  key_id,
  client_type,
  protocol_type,
  auth_scheme,
  upstream_base_url,
  static_headers_json,
  default_model,
  reasoning_effort,
  service_tier,
  created_at,
  updated_at
)
SELECT
  key_id,
  CASE
    WHEN lower(client_type) IN ('codex', 'claude_code') THEN lower(client_type)
    ELSE 'codex'
  END,
  CASE
    WHEN lower(replace(protocol_type, '-', '_')) IN ('anthropic', 'anthropic_native') THEN 'anthropic_native'
    ELSE 'openai_compat'
  END,
  CASE
    WHEN lower(replace(protocol_type, '-', '_')) IN ('anthropic', 'anthropic_native') THEN 'x_api_key'
    ELSE 'authorization_bearer'
  END,
  CASE
    WHEN lower(replace(protocol_type, '-', '_')) IN ('azure', 'azure_openai') THEN NULL
    ELSE upstream_base_url
  END,
  CASE
    WHEN lower(replace(protocol_type, '-', '_')) IN ('azure', 'azure_openai') THEN NULL
    ELSE static_headers_json
  END,
  default_model,
  reasoning_effort,
  service_tier,
  created_at,
  updated_at
FROM api_key_profiles;

DROP TABLE api_key_profiles;
ALTER TABLE api_key_profiles_new RENAME TO api_key_profiles;

CREATE INDEX IF NOT EXISTS idx_api_key_profiles_client_protocol
  ON api_key_profiles(client_type, protocol_type);

PRAGMA foreign_keys = ON;
COMMIT;
