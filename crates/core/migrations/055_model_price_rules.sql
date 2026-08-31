CREATE TABLE IF NOT EXISTS model_price_rules (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    model_pattern TEXT NOT NULL,
    match_type TEXT NOT NULL,
    billing_mode TEXT NOT NULL,
    currency TEXT NOT NULL,
    unit TEXT NOT NULL,
    input_price_per_1m REAL,
    cached_input_price_per_1m REAL,
    output_price_per_1m REAL,
    reasoning_output_price_per_1m REAL,
    cache_write_5m_price_per_1m REAL,
    cache_write_1h_price_per_1m REAL,
    cache_hit_price_per_1m REAL,
    long_context_threshold_tokens INTEGER,
    long_context_input_price_per_1m REAL,
    long_context_cached_input_price_per_1m REAL,
    long_context_output_price_per_1m REAL,
    source TEXT NOT NULL,
    source_url TEXT,
    seed_version TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_model_price_rules_provider_model_mode
    ON model_price_rules(provider, model_pattern, billing_mode);

CREATE INDEX IF NOT EXISTS idx_model_price_rules_source_seed
    ON model_price_rules(source, seed_version);

CREATE INDEX IF NOT EXISTS idx_model_price_rules_enabled_priority
    ON model_price_rules(enabled, priority DESC);
