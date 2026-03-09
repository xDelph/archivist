use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct MessageRow {
    pub team_id: String,
    pub channel_id: String,
    pub ts: String,
    pub thread_ts: Option<String>,
    pub user_id: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ReactionRow {
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub user_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct FileRow {
    pub id: String,
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub name: String,
    pub mimetype: Option<String>,
    pub permalink: Option<String>,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ChannelRow {
    pub team_id: String,
    pub id: String,
    pub kind: String,
    pub name: Option<String>,
    pub is_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct UserRow {
    pub team_id: String,
    pub id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct SearchDocumentRow {
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub title: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ThreadSummaryRow {
    pub team_id: String,
    pub channel_id: String,
    pub root_ts: String,
    pub title: String,
    pub preview: String,
    pub reply_count: i64,
    pub participant_count: i64,
    pub reaction_count: i64,
    pub file_count: i64,
    pub last_activity_ts: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct AnalyticsEventRow {
    pub event_name: String,
    pub subject_id: Option<String>,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct SavedItemRow {
    pub team_id: String,
    pub slack_user_id: String,
    pub thread_id: String,
    pub channel_id: String,
    pub root_ts: String,
    pub title: String,
    pub preview: String,
    pub last_activity_ts: String,
    pub saved_at: String,
}

#[cfg(test)]
#[path = "sqlx_models_tests.rs"]
mod tests;
