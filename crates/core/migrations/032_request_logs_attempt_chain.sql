ALTER TABLE request_logs ADD COLUMN initial_account_id TEXT;
ALTER TABLE request_logs ADD COLUMN attempted_account_ids_json TEXT;
