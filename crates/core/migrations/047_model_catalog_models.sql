CREATE TABLE IF NOT EXISTS model_catalog_scopes (
  scope TEXT PRIMARY KEY,
  extra_json TEXT NOT NULL DEFAULT '{}',
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS model_catalog_models (
  scope TEXT NOT NULL,
  slug TEXT NOT NULL,
  display_name TEXT NOT NULL,
  source_kind TEXT NOT NULL DEFAULT 'remote',
  user_edited INTEGER NOT NULL DEFAULT 0,
  description TEXT,
  default_reasoning_level TEXT,
  shell_type TEXT,
  visibility TEXT,
  supported_in_api INTEGER,
  priority INTEGER,
  availability_nux_json TEXT,
  upgrade_json TEXT,
  base_instructions TEXT,
  model_messages_json TEXT,
  supports_reasoning_summaries INTEGER,
  default_reasoning_summary TEXT,
  support_verbosity INTEGER,
  default_verbosity_json TEXT,
  apply_patch_tool_type TEXT,
  web_search_tool_type TEXT,
  truncation_mode TEXT,
  truncation_limit INTEGER,
  truncation_extra_json TEXT,
  supports_parallel_tool_calls INTEGER,
  supports_image_detail_original INTEGER,
  context_window INTEGER,
  auto_compact_token_limit INTEGER,
  effective_context_window_percent INTEGER,
  minimal_client_version_json TEXT,
  supports_search_tool INTEGER,
  extra_json TEXT NOT NULL DEFAULT '{}',
  sort_index INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (scope, slug)
);

CREATE INDEX IF NOT EXISTS idx_model_catalog_models_scope_sort
  ON model_catalog_models(scope, sort_index, slug);

CREATE TABLE IF NOT EXISTS model_catalog_reasoning_levels (
  scope TEXT NOT NULL,
  slug TEXT NOT NULL,
  effort TEXT NOT NULL,
  description TEXT NOT NULL,
  extra_json TEXT NOT NULL DEFAULT '{}',
  sort_index INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (scope, slug, effort)
);

CREATE INDEX IF NOT EXISTS idx_model_catalog_reasoning_levels_scope_sort
  ON model_catalog_reasoning_levels(scope, slug, sort_index, effort);

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
