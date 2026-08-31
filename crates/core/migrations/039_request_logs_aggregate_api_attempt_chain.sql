ALTER TABLE request_logs ADD COLUMN initial_aggregate_api_id TEXT;
ALTER TABLE request_logs ADD COLUMN attempted_aggregate_api_ids_json TEXT;
