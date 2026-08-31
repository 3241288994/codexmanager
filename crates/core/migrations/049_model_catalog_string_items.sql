CREATE TABLE IF NOT EXISTS model_catalog_string_items (
  scope TEXT NOT NULL,
  slug TEXT NOT NULL,
  item_kind TEXT NOT NULL,
  value TEXT NOT NULL,
  sort_index INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (scope, slug, item_kind, value)
);

CREATE INDEX IF NOT EXISTS idx_model_catalog_string_items_scope_kind_sort
  ON model_catalog_string_items(scope, item_kind, slug, sort_index, value);

DROP TABLE IF EXISTS model_catalog_additional_speed_tiers;
DROP TABLE IF EXISTS model_catalog_experimental_supported_tools;
DROP TABLE IF EXISTS model_catalog_input_modalities;
DROP TABLE IF EXISTS model_catalog_available_in_plans;
