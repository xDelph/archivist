mod backfill_batch;
mod local_store;
mod memory;
mod pg_materialized;
mod pg_store_backfill;
mod pg_store;
mod pg_store_reads;
mod pg_support;
mod search_index;
mod sqlx_models;
mod sqlx_queries;
mod sqlx_schema;
mod store;
mod thread_summary_index;

pub use backfill_batch::BackfillBatchStats;
pub use local_store::JsonlEventStore;
pub use memory::InMemoryEventStore;
pub use pg_store::PgEventStore;
pub use sqlx_models::{
    AnalyticsEventRow, ChannelRow, FileRow, MessageRow, ReactionRow, SavedItemRow,
    SearchDocumentRow, ThreadSummaryRow, UserRow,
};
pub use sqlx_queries::{
    attach_file_query, delete_saved_item_query, initial_catch_up_query,
    insert_analytics_event_query, upsert_channel_query, upsert_file_query, upsert_message_query,
    upsert_reaction_query, upsert_saved_item_query, upsert_search_document_query,
    upsert_thread_summary_query, upsert_user_query,
};
pub use sqlx_schema::{
    backfill_search_documents_query, create_saved_items_table_query,
    create_search_documents_table_query, create_thread_summaries_table_query,
};
pub use store::{EventStore, RepositoryHealth, RepositoryMode, StoreError, StoreOutcome};
