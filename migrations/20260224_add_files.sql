CREATE TABLE files (
    file_id      TEXT PRIMARY KEY,
    team_id      TEXT NOT NULL,
    channel_id   TEXT NOT NULL,
    message_ts   TEXT NOT NULL,
    name         TEXT NOT NULL DEFAULT '',
    mimetype     TEXT NOT NULL DEFAULT '',
    size_bytes   BIGINT NOT NULL DEFAULT 0,
    storage_key  TEXT NOT NULL DEFAULT '',
    storage_url  TEXT NOT NULL DEFAULT '',
    cached_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX files_message_idx ON files (channel_id, message_ts);
