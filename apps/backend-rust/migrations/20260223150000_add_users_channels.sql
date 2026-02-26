-- users: cached Slack user profiles (populated during backfill)
CREATE TABLE users (
    user_id      TEXT        PRIMARY KEY,
    team_id      TEXT        NOT NULL,
    display_name TEXT        NOT NULL DEFAULT '',
    avatar_url   TEXT        NOT NULL DEFAULT '',
    cached_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- channels: cached Slack channel names (populated during backfill)
CREATE TABLE channels (
    channel_id  TEXT        PRIMARY KEY,
    team_id     TEXT        NOT NULL,
    name        TEXT        NOT NULL DEFAULT '',
    cached_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
