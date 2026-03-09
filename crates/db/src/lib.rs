mod memory;
mod sqlx_models;
mod sqlx_queries;
mod store;

pub use memory::InMemoryEventStore;
pub use sqlx_models::{
    AnalyticsEventRow, ChannelRow, FileRow, MessageRow, ReactionRow, SearchDocumentRow, UserRow,
};
pub use sqlx_queries::{
    insert_analytics_event_query, upsert_channel_query, upsert_file_query, upsert_message_query,
    upsert_reaction_query, upsert_search_document_query, upsert_user_query,
};
pub use store::{JsonlEventStore, RepositoryHealth, RepositoryMode, StoreError, StoreOutcome};
