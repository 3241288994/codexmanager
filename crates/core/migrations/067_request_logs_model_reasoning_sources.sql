ALTER TABLE request_logs ADD COLUMN client_model TEXT;
ALTER TABLE request_logs ADD COLUMN model_source TEXT;
ALTER TABLE request_logs ADD COLUMN client_reasoning_effort TEXT;
ALTER TABLE request_logs ADD COLUMN reasoning_source TEXT;
