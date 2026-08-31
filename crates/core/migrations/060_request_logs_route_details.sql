ALTER TABLE request_logs ADD COLUMN upstream_model TEXT;
ALTER TABLE request_logs ADD COLUMN actual_source_kind TEXT;
ALTER TABLE request_logs ADD COLUMN actual_source_id TEXT;

CREATE INDEX IF NOT EXISTS idx_request_logs_actual_source_created_at
  ON request_logs(actual_source_kind, actual_source_id, created_at DESC);
