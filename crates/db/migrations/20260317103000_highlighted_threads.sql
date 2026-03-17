CREATE TABLE IF NOT EXISTS highlighted_threads (
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    pinned_by_user_id TEXT NOT NULL,
    pinned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, root_ts),
    FOREIGN KEY (channel_id, root_ts)
        REFERENCES thread_summaries (channel_id, root_ts)
        ON DELETE CASCADE,
    FOREIGN KEY (pinned_by_user_id)
        REFERENCES users (id)
        ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS highlighted_threads_pinned_at_idx
ON highlighted_threads (pinned_at DESC, channel_id ASC, root_ts ASC);
