CREATE TABLE backfill_state (
    id               BOOLEAN     PRIMARY KEY DEFAULT TRUE CHECK (id),
    next_channel_id  TEXT,
    users_cursor     TEXT,
    users_synced_at  TIMESTAMPTZ,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO backfill_state (id)
VALUES (TRUE)
ON CONFLICT (id) DO NOTHING;
