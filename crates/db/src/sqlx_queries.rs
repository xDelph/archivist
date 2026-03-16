pub const fn upsert_message_query() -> &'static str {
    r#"
INSERT INTO messages (channel_id, ts, root_ts, user_id, text, occurred_at)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (channel_id, ts) DO UPDATE
SET root_ts = EXCLUDED.root_ts,
    user_id = EXCLUDED.user_id,
    text = EXCLUDED.text,
    occurred_at = EXCLUDED.occurred_at
"#
}

pub const fn upsert_reaction_query() -> &'static str {
    r#"
INSERT INTO reactions (channel_id, message_ts, user_id, name, occurred_at)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (channel_id, message_ts, user_id, name) DO NOTHING
"#
}

pub const fn upsert_file_query() -> &'static str {
    r#"
INSERT INTO files (id, name, mimetype, permalink, size_bytes)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name,
    mimetype = EXCLUDED.mimetype,
    permalink = EXCLUDED.permalink,
    size_bytes = EXCLUDED.size_bytes
"#
}

pub const fn attach_file_query() -> &'static str {
    r#"
INSERT INTO message_files (channel_id, message_ts, file_id)
VALUES ($1, $2, $3)
ON CONFLICT (channel_id, message_ts, file_id) DO NOTHING
"#
}

pub const fn upsert_channel_query() -> &'static str {
    r#"
INSERT INTO channels (id, kind, name, is_archived)
VALUES ($1, $2, $3, $4)
ON CONFLICT (id) DO UPDATE
SET kind = EXCLUDED.kind,
    name = EXCLUDED.name,
    is_archived = EXCLUDED.is_archived
"#
}

pub const fn upsert_user_query() -> &'static str {
    r#"
INSERT INTO users (id, email, display_name, avatar_url, is_active)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (id) DO UPDATE
SET email = EXCLUDED.email,
    display_name = EXCLUDED.display_name,
    avatar_url = EXCLUDED.avatar_url,
    is_active = EXCLUDED.is_active
"#
}

pub const fn upsert_search_document_query() -> &'static str {
    r#"
INSERT INTO search_documents (
    channel_id,
    root_ts,
    message_ts,
    title,
    body,
    document,
    message_occurred_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (channel_id, message_ts) DO UPDATE
SET title = EXCLUDED.title,
    body = EXCLUDED.body,
    document = EXCLUDED.document,
    message_occurred_at = EXCLUDED.message_occurred_at
"#
}

pub const fn upsert_thread_summary_query() -> &'static str {
    r#"
INSERT INTO thread_summaries (
    channel_id,
    root_ts,
    reply_count,
    participant_count,
    reaction_count,
    file_count,
    root_message_at,
    last_activity_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (channel_id, root_ts) DO UPDATE
SET reply_count = EXCLUDED.reply_count,
    participant_count = EXCLUDED.participant_count,
    reaction_count = EXCLUDED.reaction_count,
    file_count = EXCLUDED.file_count,
    root_message_at = EXCLUDED.root_message_at,
    last_activity_at = EXCLUDED.last_activity_at
"#
}

pub const fn upsert_saved_item_query() -> &'static str {
    r#"
INSERT INTO saved_items (user_id, channel_id, root_ts)
VALUES ($1, $2, $3)
ON CONFLICT (user_id, channel_id, root_ts) DO UPDATE
SET saved_at = now()
"#
}

pub const fn delete_saved_item_query() -> &'static str {
    r#"
DELETE FROM saved_items
WHERE user_id = $1
  AND channel_id = $2
  AND root_ts = $3
"#
}

pub const fn initial_catch_up_query() -> &'static str {
    r#"
SELECT
    thread_summaries.channel_id,
    thread_summaries.root_ts,
    thread_summaries.reply_count,
    thread_summaries.participant_count,
    thread_summaries.reaction_count,
    thread_summaries.file_count,
    thread_summaries.root_message_at,
    thread_summaries.last_activity_at,
    channels.name AS channel_name,
    channels.kind AS channel_kind,
    channels.is_archived
FROM thread_summaries
LEFT JOIN channels
    ON channels.id = thread_summaries.channel_id
WHERE thread_summaries.last_activity_at >= $1
ORDER BY thread_summaries.last_activity_at DESC,
         thread_summaries.channel_id ASC,
         thread_summaries.root_ts ASC
"#
}

pub const fn insert_analytics_event_query() -> &'static str {
    r#"
INSERT INTO analytics_events (event_type, user_id, metadata)
VALUES ($1, $2, $3)
"#
}

#[cfg(test)]
#[path = "sqlx_queries_tests.rs"]
mod tests;
