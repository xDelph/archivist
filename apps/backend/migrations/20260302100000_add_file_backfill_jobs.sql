CREATE TABLE file_backfill_jobs (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    dedupe_key   TEXT        NOT NULL UNIQUE,
    team_id      TEXT        NOT NULL,
    channel_id   TEXT        NOT NULL,
    message_ts   TEXT        NOT NULL,
    files_json   JSONB       NOT NULL,
    status       TEXT        NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
    attempts     INT         NOT NULL DEFAULT 0,
    max_attempts INT         NOT NULL DEFAULT 5,
    started_at   TIMESTAMPTZ,
    finished_at  TIMESTAMPTZ,
    last_error   TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_file_backfill_jobs_status_created
    ON file_backfill_jobs (status, created_at ASC);

CREATE INDEX idx_file_backfill_jobs_channel_message
    ON file_backfill_jobs (channel_id, message_ts);
