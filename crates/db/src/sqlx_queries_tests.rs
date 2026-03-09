use super::{
    delete_saved_item_query, initial_catch_up_query, insert_analytics_event_query,
    upsert_channel_query, upsert_file_query, upsert_message_query, upsert_reaction_query,
    upsert_saved_item_query, upsert_search_document_query, upsert_thread_summary_query,
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
    assert!(upsert_thread_summary_query().contains("INSERT INTO thread_summaries"));
    assert!(upsert_saved_item_query().contains("INSERT INTO saved_items"));
    assert!(delete_saved_item_query().contains("DELETE FROM saved_items"));
    assert!(initial_catch_up_query().contains("FROM thread_summaries"));
    assert!(initial_catch_up_query().contains("LEFT JOIN channels"));
    assert!(insert_analytics_event_query().contains("INSERT INTO analytics_events"));
}
