pub const fn create_search_documents_table_query() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS search_documents (
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    message_ts TEXT NOT NULL,
    title TEXT,
    body TEXT NOT NULL,
    document TSVECTOR NOT NULL,
    message_occurred_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (channel_id, message_ts)
);

CREATE INDEX IF NOT EXISTS search_documents_document_idx
ON search_documents
USING GIN (document);

CREATE INDEX IF NOT EXISTS search_documents_channel_ts_idx
ON search_documents (channel_id, root_ts, message_occurred_at DESC);
"#
}

pub const fn create_thread_summaries_table_query() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS thread_summaries (
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    reply_count BIGINT NOT NULL,
    participant_count BIGINT NOT NULL,
    reaction_count BIGINT NOT NULL,
    file_count BIGINT NOT NULL,
    root_message_at TIMESTAMPTZ NOT NULL,
    last_activity_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (channel_id, root_ts)
);

CREATE INDEX IF NOT EXISTS thread_summaries_last_activity_idx
ON thread_summaries (last_activity_at DESC);

CREATE INDEX IF NOT EXISTS thread_summaries_channel_activity_idx
ON thread_summaries (channel_id, last_activity_at DESC);
"#
}

pub const fn create_generated_thread_summaries_table_query() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS generated_thread_summaries (
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    summary TEXT NOT NULL,
    full_summary TEXT,
    why_it_mattered TEXT,
    status TEXT NOT NULL,
    topic_tags TEXT[] NOT NULL DEFAULT '{}',
    source_last_activity_ts TEXT NOT NULL,
    model TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, root_ts)
);

ALTER TABLE generated_thread_summaries
ADD COLUMN IF NOT EXISTS full_summary TEXT;

UPDATE generated_thread_summaries
SET full_summary = summary
WHERE full_summary IS NULL;

CREATE INDEX IF NOT EXISTS generated_thread_summaries_generated_at_idx
ON generated_thread_summaries (generated_at DESC);

CREATE INDEX IF NOT EXISTS generated_thread_summaries_channel_generated_at_idx
ON generated_thread_summaries (channel_id, generated_at DESC);
"#
}

pub const fn create_saved_items_table_query() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS saved_items (
    user_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    saved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, channel_id, root_ts)
);

CREATE INDEX IF NOT EXISTS saved_items_user_saved_at_idx
ON saved_items (user_id, saved_at DESC);
"#
}

pub const fn backfill_search_documents_query() -> &'static str {
    r#"
WITH root_messages AS (
    SELECT channel_id, ts AS root_ts, text AS root_text
    FROM messages
    WHERE root_ts = ts
)
INSERT INTO search_documents (
    channel_id,
    root_ts,
    message_ts,
    title,
    body,
    document,
    message_occurred_at
)
SELECT
    messages.channel_id,
    messages.root_ts,
    messages.ts AS message_ts,
    COALESCE(root_messages.root_text, messages.text) AS title,
    messages.text AS body,
    to_tsvector(
        'english',
        btrim(
            coalesce(COALESCE(root_messages.root_text, messages.text), '')
            || ' '
            || messages.text
        )
    ) AS document,
    messages.occurred_at AS message_occurred_at
FROM messages
LEFT JOIN root_messages
    ON root_messages.channel_id = messages.channel_id
   AND root_messages.root_ts = messages.root_ts
ON CONFLICT (channel_id, message_ts) DO UPDATE
SET title = EXCLUDED.title,
    body = EXCLUDED.body,
    document = EXCLUDED.document,
    message_occurred_at = EXCLUDED.message_occurred_at
"#
}

#[cfg(test)]
#[path = "sqlx_schema_tests.rs"]
mod tests;
