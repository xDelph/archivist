CREATE TABLE backfill_jobs (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    status       TEXT        NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
    requested_by TEXT        NOT NULL DEFAULT 'api',
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at   TIMESTAMPTZ,
    finished_at  TIMESTAMPTZ,
    attempts     INT         NOT NULL DEFAULT 0,
    last_error   TEXT
);

CREATE INDEX idx_backfill_jobs_status_requested_at
    ON backfill_jobs (status, requested_at ASC);
