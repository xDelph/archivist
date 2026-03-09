pub const fn upsert_message_query() -> &'static str {
    r#"
INSERT INTO messages (team_id, channel_id, ts, thread_ts, user_id, text)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (team_id, channel_id, ts) DO UPDATE
SET thread_ts = EXCLUDED.thread_ts,
    user_id = EXCLUDED.user_id,
    text = EXCLUDED.text
"#
}

pub const fn upsert_reaction_query() -> &'static str {
    r#"
INSERT INTO reactions (team_id, channel_id, message_ts, user_id, name)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (team_id, channel_id, message_ts, user_id, name) DO NOTHING
"#
}

pub const fn upsert_file_query() -> &'static str {
    r#"
INSERT INTO files (id, team_id, channel_id, message_ts, name, mimetype, permalink, size_bytes)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (team_id, id) DO UPDATE
SET channel_id = EXCLUDED.channel_id,
    message_ts = EXCLUDED.message_ts,
    name = EXCLUDED.name,
    mimetype = EXCLUDED.mimetype,
    permalink = EXCLUDED.permalink,
    size_bytes = EXCLUDED.size_bytes
"#
}

pub const fn upsert_channel_query() -> &'static str {
    r#"
INSERT INTO channels (team_id, id, kind, name, is_archived)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (team_id, id) DO UPDATE
SET kind = EXCLUDED.kind,
    name = EXCLUDED.name,
    is_archived = EXCLUDED.is_archived
"#
}

pub const fn upsert_user_query() -> &'static str {
    r#"
INSERT INTO users (team_id, id, display_name, avatar_url, is_active)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (team_id, id) DO UPDATE
SET display_name = EXCLUDED.display_name,
    avatar_url = EXCLUDED.avatar_url,
    is_active = EXCLUDED.is_active
"#
}

pub const fn upsert_search_document_query() -> &'static str {
    r#"
INSERT INTO search_documents (team_id, channel_id, message_ts, title, body)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (team_id, channel_id, message_ts) DO UPDATE
SET title = EXCLUDED.title,
    body = EXCLUDED.body
"#
}

pub const fn upsert_thread_summary_query() -> &'static str {
    r#"
INSERT INTO thread_summaries (
    team_id,
    channel_id,
    root_ts,
    title,
    preview,
    reply_count,
    participant_count,
    reaction_count,
    file_count,
    last_activity_ts
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
ON CONFLICT (team_id, channel_id, root_ts) DO UPDATE
SET title = EXCLUDED.title,
    preview = EXCLUDED.preview,
    reply_count = EXCLUDED.reply_count,
    participant_count = EXCLUDED.participant_count,
    reaction_count = EXCLUDED.reaction_count,
    file_count = EXCLUDED.file_count,
    last_activity_ts = EXCLUDED.last_activity_ts
"#
}

pub const fn upsert_saved_item_query() -> &'static str {
    r#"
INSERT INTO saved_items (
    team_id,
    slack_user_id,
    thread_id,
    channel_id,
    root_ts,
    title,
    preview,
    last_activity_ts
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (team_id, slack_user_id, thread_id) DO UPDATE
SET title = EXCLUDED.title,
    preview = EXCLUDED.preview,
    last_activity_ts = EXCLUDED.last_activity_ts,
    saved_at = now()
"#
}

pub const fn delete_saved_item_query() -> &'static str {
    r#"
DELETE FROM saved_items
WHERE team_id = $1
  AND slack_user_id = $2
  AND thread_id = $3
"#
}

pub const fn initial_catch_up_query() -> &'static str {
    r#"
SELECT
    thread_summaries.team_id,
    thread_summaries.channel_id,
    thread_summaries.root_ts,
    thread_summaries.title,
    thread_summaries.preview,
    thread_summaries.reply_count,
    thread_summaries.participant_count,
    thread_summaries.reaction_count,
    thread_summaries.file_count,
    thread_summaries.last_activity_ts,
    channels.name AS channel_name,
    channels.kind AS channel_kind,
    channels.is_archived
FROM thread_summaries
LEFT JOIN channels
    ON channels.team_id = thread_summaries.team_id
   AND channels.id = thread_summaries.channel_id
WHERE thread_summaries.team_id = $1
  AND split_part(thread_summaries.last_activity_ts, '.', 1)::bigint >= $2
ORDER BY thread_summaries.last_activity_ts DESC,
         thread_summaries.channel_id ASC,
         thread_summaries.root_ts ASC
"#
}

pub const fn insert_analytics_event_query() -> &'static str {
    r#"
INSERT INTO analytics_events (event_name, subject_id, payload_json)
VALUES ($1, $2, $3)
"#
}

#[cfg(test)]
#[path = "sqlx_queries_tests.rs"]
mod tests;
