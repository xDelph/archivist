CREATE TABLE thread_weekly_scores (
    week_start  DATE        NOT NULL,
    channel_id  TEXT        NOT NULL,
    thread_ts   TEXT        NOT NULL,
    score_week  INT         NOT NULL,
    score_total INT         NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (week_start, channel_id, thread_ts)
);

CREATE INDEX idx_thread_weekly_scores_week_score
    ON thread_weekly_scores (week_start, score_week DESC);
