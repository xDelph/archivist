CREATE TABLE IF NOT EXISTS app_events (
    event_id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    channel_kind TEXT NOT NULL,
    event_type TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    payload_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS app_events_team_channel_event_time_idx
ON app_events (channel_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS app_events_event_type_occurred_at_idx
ON app_events (event_type, occurred_at DESC);

CREATE TABLE IF NOT EXISTS channels (
    id TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id),
    CHECK (kind IN ('public', 'private', 'direct', 'unknown'))
);

CREATE INDEX IF NOT EXISTS channels_team_name_idx
ON channels (name);

CREATE TABLE IF NOT EXISTS users (
    id TEXT NOT NULL,
    display_name TEXT,
    avatar_url TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_synced_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS auth_identities (
    slack_user_id TEXT NOT NULL,
    email TEXT,
    display_name TEXT,
    avatar_url TEXT,
    last_authenticated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (slack_user_id),
    FOREIGN KEY (slack_user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS messages (
    channel_id TEXT NOT NULL,
    ts TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    user_id TEXT,
    text TEXT NOT NULL DEFAULT '',
    subtype TEXT,
    raw_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL,
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, ts),
    FOREIGN KEY (channel_id) REFERENCES channels (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS messages_team_thread_idx
ON messages (channel_id, root_ts, occurred_at DESC, ts);

CREATE INDEX IF NOT EXISTS messages_user_occurred_at_idx
ON messages (user_id, occurred_at DESC);

CREATE TABLE IF NOT EXISTS reactions (
    channel_id TEXT NOT NULL,
    message_ts TEXT NOT NULL,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, message_ts, user_id, name),
    FOREIGN KEY (channel_id, message_ts)
        REFERENCES messages (channel_id, ts)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS files (
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    mimetype TEXT,
    permalink TEXT,
    size_bytes BIGINT,
    storage_key TEXT,
    storage_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS message_files (
    channel_id TEXT NOT NULL,
    message_ts TEXT NOT NULL,
    file_id TEXT NOT NULL,
    attached_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, message_ts, file_id),
    FOREIGN KEY (channel_id, message_ts)
        REFERENCES messages (channel_id, ts)
        ON DELETE CASCADE,
    FOREIGN KEY (file_id)
        REFERENCES files (id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS files_team_message_idx
ON message_files (file_id, attached_at DESC);

CREATE TABLE IF NOT EXISTS analytics_events (
    id BIGSERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    user_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS analytics_events_team_created_idx
ON analytics_events (event_type, created_at DESC);

CREATE INDEX IF NOT EXISTS analytics_events_team_name_idx
ON analytics_events (user_id, created_at DESC);
