CREATE TABLE thread_rollups (
    channel_id             TEXT        NOT NULL,
    thread_ts              TEXT        NOT NULL,
    root_user_id           TEXT,
    root_text              TEXT        NOT NULL DEFAULT '',
    root_created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reaction_count_total   BIGINT      NOT NULL DEFAULT 0,
    reply_count_total      BIGINT      NOT NULL DEFAULT 0,
    participant_count_total BIGINT     NOT NULL DEFAULT 1,
    file_count_total       BIGINT      NOT NULL DEFAULT 0,
    has_files              BOOLEAN     NOT NULL DEFAULT FALSE,
    score_total            BIGINT      NOT NULL DEFAULT 0,
    computed_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source_max_updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version                INT         NOT NULL DEFAULT 1,
    PRIMARY KEY (channel_id, thread_ts)
);

CREATE INDEX idx_thread_rollups_score_total
    ON thread_rollups (score_total DESC);

CREATE INDEX idx_thread_rollups_root_created
    ON thread_rollups (root_created_at DESC);

CREATE INDEX idx_thread_rollups_root_user
    ON thread_rollups (root_user_id);

CREATE TABLE thread_period_scores (
    period_kind            TEXT        NOT NULL CHECK (period_kind IN ('week', 'month')),
    period_start           DATE        NOT NULL,
    channel_id             TEXT        NOT NULL,
    thread_ts              TEXT        NOT NULL,
    score_period           BIGINT      NOT NULL DEFAULT 0,
    computed_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source_max_updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version                INT         NOT NULL DEFAULT 1,
    PRIMARY KEY (period_kind, period_start, channel_id, thread_ts)
);

CREATE INDEX idx_thread_period_scores_rank
    ON thread_period_scores (period_kind, period_start, score_period DESC);

CREATE INDEX idx_thread_period_scores_lookup
    ON thread_period_scores (channel_id, thread_ts);

CREATE TABLE channel_daily_rollups (
    day          DATE        NOT NULL,
    channel_id   TEXT        NOT NULL,
    thread_count BIGINT      NOT NULL DEFAULT 0,
    message_count BIGINT     NOT NULL DEFAULT 0,
    computed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version      INT         NOT NULL DEFAULT 1,
    PRIMARY KEY (day, channel_id)
);

CREATE INDEX idx_channel_daily_rollups_day
    ON channel_daily_rollups (day DESC);

CREATE TABLE workspace_overview_rollups (
    snapshot_key    TEXT        PRIMARY KEY,
    total_messages  BIGINT      NOT NULL DEFAULT 0,
    total_threads   BIGINT      NOT NULL DEFAULT 0,
    total_files     BIGINT      NOT NULL DEFAULT 0,
    total_users     BIGINT      NOT NULL DEFAULT 0,
    messages_change DOUBLE PRECISION NOT NULL DEFAULT 0,
    threads_change  DOUBLE PRECISION NOT NULL DEFAULT 0,
    files_change    DOUBLE PRECISION NOT NULL DEFAULT 0,
    users_change    DOUBLE PRECISION NOT NULL DEFAULT 0,
    computed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version         INT         NOT NULL DEFAULT 1
);

CREATE TABLE aggregation_jobs (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    dedupe_key   TEXT        NOT NULL UNIQUE,
    job_kind     TEXT        NOT NULL CHECK (job_kind IN ('thread_rollup')),
    channel_id   TEXT        NOT NULL,
    thread_ts    TEXT        NOT NULL,
    status       TEXT        NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
    attempts     INT         NOT NULL DEFAULT 0,
    max_attempts INT         NOT NULL DEFAULT 5,
    requested_by TEXT        NOT NULL DEFAULT 'system',
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_at    TIMESTAMPTZ,
    started_at   TIMESTAMPTZ,
    finished_at  TIMESTAMPTZ,
    last_error   TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_aggregation_jobs_status_available
    ON aggregation_jobs (status, available_at ASC, created_at ASC);

CREATE INDEX idx_aggregation_jobs_thread
    ON aggregation_jobs (channel_id, thread_ts);
