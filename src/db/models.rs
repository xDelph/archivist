use chrono::{DateTime, Utc};

pub struct SlackEventRecord<'a> {
    pub event_id: &'a str,
    pub team_id: &'a str,
    pub event_time: i64,
    pub payload_json: &'a serde_json::Value,
}

pub struct MessageRecord {
    pub team_id: String,
    pub channel_id: String,
    pub ts: String,
    pub thread_ts: Option<String>,
    pub user_id: Option<String>,
    pub text: String,
    pub subtype: Option<String>,
    pub edited_ts: Option<String>,
    pub deleted: bool,
    pub raw_json: serde_json::Value,
}

pub struct ReactionRecord {
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub user_id: String,
    pub reaction_name: String,
    pub event_ts: String,
}

pub struct UserRecord {
    pub user_id: String,
    pub team_id: String,
    pub display_name: String,
    pub avatar_url: String,
}

pub struct ChannelRecord {
    pub channel_id: String,
    pub team_id: String,
    pub name: String,
}

#[derive(Clone)]
pub struct ThreadSummary {
    pub channel_id: String,
    pub channel_name: String,
    pub thread_ts: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub display_name: String,
    pub avatar_url: String,
    pub reaction_count: i64,
    pub reply_count: i64,
    pub participant_count: i64,
    pub score: i64,
}

#[derive(Clone)]
pub struct ThreadWithWeeklyScore {
    pub thread: ThreadSummary,
    pub score_week: i64,
}

#[derive(Clone)]
pub struct PeriodRankedThread {
    pub thread: ThreadSummary,
    pub rank_score: i64,
    pub rank: i64,
    pub prev_rank: Option<i64>,
}

pub struct ThreadMessage {
    pub ts: String,
    pub text: String,
    pub display_name: String,
    pub avatar_url: String,
    pub reactions: serde_json::Value,
}

pub struct FileRecord {
    pub file_id: String,
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub name: String,
    pub mimetype: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub storage_url: String,
}

pub struct FileRow {
    pub file_id: String,
    pub message_ts: String,
    pub name: String,
    pub mimetype: String,
    pub storage_url: String,
}
