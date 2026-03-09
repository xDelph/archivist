pub const fn create_search_documents_table_query() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS search_documents (
    team_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    message_ts TEXT NOT NULL,
    title TEXT,
    body TEXT NOT NULL,
    document TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('english', trim(concat_ws(' ', coalesce(title, ''), body)))
    ) STORED,
    PRIMARY KEY (team_id, channel_id, message_ts)
);

CREATE INDEX IF NOT EXISTS search_documents_document_idx
ON search_documents
USING GIN (document);

CREATE INDEX IF NOT EXISTS search_documents_channel_ts_idx
ON search_documents (team_id, channel_id, message_ts DESC);
"#
}

pub const fn create_thread_summaries_table_query() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS thread_summaries (
    team_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    title TEXT NOT NULL,
    preview TEXT NOT NULL,
    reply_count BIGINT NOT NULL,
    participant_count BIGINT NOT NULL,
    reaction_count BIGINT NOT NULL,
    file_count BIGINT NOT NULL,
    last_activity_ts TEXT NOT NULL,
    PRIMARY KEY (team_id, channel_id, root_ts)
);

CREATE INDEX IF NOT EXISTS thread_summaries_last_activity_idx
ON thread_summaries (team_id, last_activity_ts DESC);

CREATE INDEX IF NOT EXISTS thread_summaries_channel_activity_idx
ON thread_summaries (team_id, channel_id, last_activity_ts DESC);
"#
}

pub const fn create_saved_items_table_query() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS saved_items (
    team_id TEXT NOT NULL,
    slack_user_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    root_ts TEXT NOT NULL,
    title TEXT NOT NULL,
    preview TEXT NOT NULL,
    last_activity_ts TEXT NOT NULL,
    saved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, slack_user_id, thread_id)
);

CREATE INDEX IF NOT EXISTS saved_items_user_saved_at_idx
ON saved_items (team_id, slack_user_id, saved_at DESC);
"#
}

pub const fn backfill_search_documents_query() -> &'static str {
    r#"
WITH root_messages AS (
    SELECT team_id, channel_id, ts AS root_ts, text AS root_text
    FROM messages
    WHERE thread_ts IS NULL
)
INSERT INTO search_documents (team_id, channel_id, message_ts, title, body)
SELECT
    messages.team_id,
    messages.channel_id,
    messages.ts AS message_ts,
    COALESCE(root_messages.root_text, messages.text) AS title,
    messages.text AS body
FROM messages
LEFT JOIN root_messages
    ON root_messages.team_id = messages.team_id
   AND root_messages.channel_id = messages.channel_id
   AND root_messages.root_ts = COALESCE(messages.thread_ts, messages.ts)
ON CONFLICT (team_id, channel_id, message_ts) DO UPDATE
SET title = EXCLUDED.title,
    body = EXCLUDED.body
"#
}

#[cfg(test)]
#[path = "sqlx_schema_tests.rs"]
mod tests;
