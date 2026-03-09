mod memory;
mod search_index;
mod thread_summary_index;
mod sqlx_models;
mod sqlx_queries;
mod sqlx_schema;
mod store;

pub use memory::InMemoryEventStore;
pub use sqlx_models::{
    AnalyticsEventRow, ChannelRow, FileRow, MessageRow, ReactionRow, SearchDocumentRow,
    ThreadSummaryRow, UserRow,
};
pub use sqlx_queries::{
    insert_analytics_event_query, upsert_channel_query, upsert_file_query, upsert_message_query,
    upsert_reaction_query, upsert_search_document_query, upsert_thread_summary_query,
    upsert_user_query,
};
pub use sqlx_schema::{
    backfill_search_documents_query, create_search_documents_table_query,
    create_thread_summaries_table_query,
};
pub use store::{JsonlEventStore, RepositoryHealth, RepositoryMode, StoreError, StoreOutcome};
