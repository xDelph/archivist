CREATE TABLE IF NOT EXISTS saved_items (
    user_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    saved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, channel_id, root_ts),
    FOREIGN KEY (user_id)
        REFERENCES users (id)
        ON DELETE CASCADE,
    FOREIGN KEY (channel_id, root_ts)
        REFERENCES thread_summaries (channel_id, root_ts)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS saved_items_user_saved_at_idx
ON saved_items (user_id, saved_at DESC);
