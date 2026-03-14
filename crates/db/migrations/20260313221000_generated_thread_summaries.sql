CREATE TABLE IF NOT EXISTS generated_thread_summaries (
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    summary TEXT NOT NULL,
    why_it_mattered TEXT,
    status TEXT NOT NULL,
    topic_tags TEXT[] NOT NULL DEFAULT '{}',
    source_last_activity_ts TEXT NOT NULL,
    model TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, root_ts),
    FOREIGN KEY (channel_id, root_ts)
        REFERENCES thread_summaries (channel_id, root_ts)
        ON DELETE CASCADE
);

ALTER TABLE generated_thread_summaries
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE generated_thread_summaries
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'generated_thread_summaries_channel_id_root_ts_fkey'
    ) THEN
        ALTER TABLE generated_thread_summaries
            ADD CONSTRAINT generated_thread_summaries_channel_id_root_ts_fkey
            FOREIGN KEY (channel_id, root_ts)
            REFERENCES thread_summaries (channel_id, root_ts)
            ON DELETE CASCADE;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS generated_thread_summaries_generated_at_idx
ON generated_thread_summaries (generated_at DESC);

CREATE INDEX IF NOT EXISTS generated_thread_summaries_channel_generated_at_idx
ON generated_thread_summaries (channel_id, generated_at DESC);
