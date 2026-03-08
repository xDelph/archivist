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

pub const fn insert_analytics_event_query() -> &'static str {
    r#"
INSERT INTO analytics_events (event_name, subject_id, payload_json)
VALUES ($1, $2, $3)
"#
}

#[cfg(test)]
mod tests {
    use super::{
        insert_analytics_event_query, upsert_channel_query, upsert_file_query,
        upsert_message_query, upsert_reaction_query, upsert_search_document_query,
        upsert_user_query,
    };

    #[test]
    fn query_helpers_target_expected_tables() {
        assert!(upsert_message_query().contains("INSERT INTO messages"));
        assert!(upsert_reaction_query().contains("INSERT INTO reactions"));
        assert!(upsert_file_query().contains("INSERT INTO files"));
        assert!(upsert_channel_query().contains("INSERT INTO channels"));
        assert!(upsert_user_query().contains("INSERT INTO users"));
        assert!(upsert_search_document_query().contains("INSERT INTO search_documents"));
        assert!(insert_analytics_event_query().contains("INSERT INTO analytics_events"));
    }
}
