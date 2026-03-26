use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct MessageRow {
    pub channel_id: String,
    pub ts: String,
    pub root_ts: String,
    pub user_id: Option<String>,
    pub text: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ReactionRow {
    pub channel_id: String,
    pub message_ts: String,
    pub user_id: String,
    pub name: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct FileRow {
    pub id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub name: String,
    pub mimetype: Option<String>,
    pub permalink: Option<String>,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ChannelRow {
    pub id: String,
    pub kind: String,
    pub name: Option<String>,
    pub is_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct UserRow {
    pub id: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct SearchDocumentRow {
    pub channel_id: String,
    pub root_ts: String,
    pub message_ts: String,
    pub title: Option<String>,
    pub body: String,
    pub message_occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ThreadSummaryRow {
    pub channel_id: String,
    pub root_ts: String,
    pub reply_count: i64,
    pub participant_count: i64,
    pub reaction_count: i64,
    pub file_count: i64,
    pub root_message_at: String,
    pub last_activity_ts: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct GeneratedThreadSummaryRow {
    pub channel_id: String,
    pub root_ts: String,
    pub summary: String,
    pub full_summary: Option<String>,
    pub why_it_mattered: Option<String>,
    pub status: String,
    pub topic_tags: Vec<String>,
    pub source_last_activity_ts: String,
    pub model: String,
    pub generated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct AnalyticsEventRow {
    pub event_type: String,
    pub user_id: Option<String>,
    pub metadata: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct SavedItemRow {
    pub user_id: String,
    pub channel_id: String,
    pub root_ts: String,
    pub saved_at: String,
}

#[cfg(test)]
#[path = "sqlx_models_tests.rs"]
mod tests;
