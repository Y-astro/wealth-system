CREATE TABLE sync_cursor (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    cursor BIGINT NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

INSERT INTO sync_cursor (id, cursor, updated_at)
VALUES (1, 0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
ON CONFLICT(id) DO NOTHING;

CREATE TABLE sync_outbox (
    event_id TEXT PRIMARY KEY NOT NULL,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    op TEXT NOT NULL,
    client_timestamp TEXT NOT NULL,
    payload TEXT NOT NULL,
    payload_key_version INTEGER NOT NULL,
    sent INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT,
    last_error TEXT,
    last_error_code TEXT,
    device_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX ix_sync_outbox_sent_created
    ON sync_outbox(sent, created_at);

CREATE INDEX ix_sync_outbox_entity
    ON sync_outbox(entity, entity_id);

CREATE TABLE sync_entity_metadata (
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    last_event_id TEXT NOT NULL,
    last_client_timestamp TEXT NOT NULL,
    last_seq BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (entity, entity_id)
);

CREATE TABLE sync_device_config (
    device_id TEXT PRIMARY KEY NOT NULL,
    key_version INTEGER,
    trust_state TEXT NOT NULL DEFAULT 'untrusted',
    last_bootstrap_at TEXT
);

CREATE TABLE sync_engine_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    lock_version BIGINT NOT NULL DEFAULT 0,
    last_push_at TEXT,
    last_pull_at TEXT,
    last_error TEXT,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT,
    last_cycle_status TEXT,
    last_cycle_duration_ms BIGINT
);

INSERT INTO sync_engine_state (id, lock_version)
VALUES (1, 0)
ON CONFLICT(id) DO NOTHING;

CREATE TABLE sync_table_state (
    table_name TEXT PRIMARY KEY NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    last_snapshot_restore_at TEXT,
    last_incremental_apply_at TEXT
);

CREATE TABLE sync_applied_events (
    event_id TEXT PRIMARY KEY NOT NULL,
    seq BIGINT NOT NULL,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE INDEX ix_sync_applied_events_seq
    ON sync_applied_events(seq);

CREATE INDEX ix_sync_outbox_status_retry_created
    ON sync_outbox(status, next_retry_at, created_at);

CREATE INDEX ix_sync_entity_metadata_last_seq
    ON sync_entity_metadata(entity, entity_id, last_seq);

INSERT INTO sync_table_state (table_name, enabled) VALUES
    ('accounts', 1),
    ('assets', 1),
    ('asset_taxonomy_assignments', 1),
    ('activities', 1),
    ('activity_import_profiles', 1),
    ('goals', 1),
    ('goals_allocation', 1),
    ('ai_threads', 1),
    ('ai_messages', 1),
    ('ai_thread_tags', 1),
    ('contribution_limits', 1),
    ('platforms', 1);
