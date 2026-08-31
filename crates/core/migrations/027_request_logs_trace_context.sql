ALTER TABLE request_logs ADD COLUMN trace_id TEXT;

ALTER TABLE request_logs ADD COLUMN original_path TEXT;

ALTER TABLE request_logs ADD COLUMN adapted_path TEXT;

ALTER TABLE request_logs ADD COLUMN response_adapter TEXT;

CREATE INDEX IF NOT EXISTS idx_request_logs_trace_id_created_at
  ON request_logs(trace_id, created_at DESC);
