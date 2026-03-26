use super::{
    backfill_search_documents_query, create_generated_thread_summaries_table_query,
    create_saved_items_table_query, create_search_documents_table_query,
    create_thread_summaries_table_query,
};

#[test]
fn search_document_schema_creates_tsvector_indexes() {
    let schema = create_search_documents_table_query();

    assert!(schema.contains("CREATE TABLE IF NOT EXISTS search_documents"));
    assert!(schema.contains("document TSVECTOR NOT NULL"));
    assert!(schema.contains("message_occurred_at TIMESTAMPTZ NOT NULL"));
    assert!(schema.contains("USING GIN (document)"));
    assert!(schema.contains("search_documents_channel_ts_idx"));
}

#[test]
fn search_document_backfill_rebuilds_from_messages_and_roots() {
    let query = backfill_search_documents_query();

    assert!(query.contains("WITH root_messages AS"));
    assert!(query.contains("INSERT INTO search_documents"));
    assert!(query.contains("FROM messages"));
    assert!(query.contains("COALESCE(root_messages.root_text, messages.text) AS title"));
    assert!(query.contains("ON CONFLICT (channel_id, message_ts) DO UPDATE"));
}

#[test]
fn thread_summary_schema_creates_activity_indexes() {
    let schema = create_thread_summaries_table_query();

    assert!(schema.contains("CREATE TABLE IF NOT EXISTS thread_summaries"));
    assert!(schema.contains("reply_count BIGINT NOT NULL"));
    assert!(schema.contains("participant_count BIGINT NOT NULL"));
    assert!(schema.contains("root_message_at TIMESTAMPTZ NOT NULL"));
    assert!(schema.contains("last_activity_at TIMESTAMPTZ NOT NULL"));
    assert!(schema.contains("thread_summaries_last_activity_idx"));
    assert!(schema.contains("thread_summaries_channel_activity_idx"));
}

#[test]
fn saved_items_schema_tracks_saved_threads_per_user() {
    let schema = create_saved_items_table_query();

    assert!(schema.contains("CREATE TABLE IF NOT EXISTS saved_items"));
    assert!(schema.contains("user_id TEXT NOT NULL"));
    assert!(schema.contains("saved_at TIMESTAMPTZ NOT NULL DEFAULT now()"));
    assert!(schema.contains("saved_items_user_saved_at_idx"));
}

#[test]
fn generated_thread_summary_schema_tracks_model_output_per_thread() {
    let schema = create_generated_thread_summaries_table_query();

    assert!(schema.contains("CREATE TABLE IF NOT EXISTS generated_thread_summaries"));
    assert!(schema.contains("summary TEXT NOT NULL"));
    assert!(schema.contains("full_summary TEXT"));
    assert!(schema.contains("topic_tags TEXT[] NOT NULL DEFAULT '{}'"));
    assert!(schema.contains("source_last_activity_ts TEXT NOT NULL"));
    assert!(schema.contains("ADD COLUMN IF NOT EXISTS full_summary TEXT"));
    assert!(schema.contains("generated_thread_summaries_generated_at_idx"));
}
