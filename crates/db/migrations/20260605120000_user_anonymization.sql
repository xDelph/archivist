ALTER TABLE users
    ADD COLUMN IF NOT EXISTS is_anonymized BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS anonymized_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS anonymized_by TEXT;

CREATE INDEX IF NOT EXISTS users_is_anonymized_idx
    ON users (is_anonymized)
    WHERE is_anonymized = TRUE;

DELETE FROM generated_thread_summaries;
