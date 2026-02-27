-- slack_events: raw dedup store, one row per event_id
CREATE TABLE slack_events (
    event_id    TEXT        PRIMARY KEY,
    team_id     TEXT        NOT NULL,
    event_time  BIGINT      NOT NULL,
    payload_json JSONB      NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- messages: normalised message store
CREATE TABLE messages (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id     TEXT        NOT NULL,
    channel_id  TEXT        NOT NULL,
    ts          TEXT        NOT NULL,
    thread_ts   TEXT,
    user_id     TEXT,
    text        TEXT        NOT NULL DEFAULT '',
    subtype     TEXT,
    edited_ts   TEXT,
    deleted     BOOLEAN     NOT NULL DEFAULT FALSE,
    raw_json    JSONB       NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (channel_id, ts)
);

CREATE INDEX idx_messages_channel_ts   ON messages (channel_id, ts);
CREATE INDEX idx_messages_thread_ts    ON messages (thread_ts) WHERE thread_ts IS NOT NULL;

-- reactions: one row per (team, channel, message, user, reaction)
CREATE TABLE reactions (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id         TEXT        NOT NULL,
    channel_id      TEXT        NOT NULL,
    message_ts      TEXT        NOT NULL,
    user_id         TEXT        NOT NULL,
    reaction_name   TEXT        NOT NULL,
    event_ts        TEXT        NOT NULL,
    UNIQUE (team_id, channel_id, message_ts, user_id, reaction_name)
);
