CREATE TABLE IF NOT EXISTS aggregate_api_supplier_models (
    supplier_key TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    upstream_model TEXT NOT NULL,
    display_name TEXT,
    status TEXT NOT NULL DEFAULT 'available',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (supplier_key, provider_type, upstream_model)
);

CREATE INDEX IF NOT EXISTS idx_aggregate_api_supplier_models_supplier
    ON aggregate_api_supplier_models(supplier_key, provider_type, status);

CREATE INDEX IF NOT EXISTS idx_aggregate_api_supplier_models_model
    ON aggregate_api_supplier_models(upstream_model);
