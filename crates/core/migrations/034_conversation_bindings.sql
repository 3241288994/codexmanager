CREATE TABLE IF NOT EXISTS conversation_bindings (
    platform_key_hash TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    thread_epoch INTEGER NOT NULL,
    thread_anchor TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    last_model TEXT,
    last_switch_reason TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL,
    PRIMARY KEY (platform_key_hash, conversation_id)
);

CREATE INDEX IF NOT EXISTS idx_conversation_bindings_account_id
    ON conversation_bindings(account_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_conversation_bindings_last_used_at
    ON conversation_bindings(last_used_at DESC);
