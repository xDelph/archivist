use anyhow::Result;
use uuid::Uuid;

use super::models::{
    ChannelRecord, FileRecord, FileRow, MessageRecord, PeriodRankedThread, ReactionRecord,
    SlackEventRecord, ThreadMessage, ThreadSummary, ThreadWithWeeklyScore, UserRecord,
};

/// All DB operations needed by the application.
///
/// `PgPool` is the production implementation.
/// `InMemoryRepository` is used in unit tests (no network required).
/// Never used as `dyn Repository`, so `Send` bounds on futures are not required.
#[allow(async_fn_in_trait)]
pub trait Repository {
    async fn event_exists(&self, event_id: &str) -> Result<bool>;
    async fn insert_slack_event(&self, rec: SlackEventRecord<'_>) -> Result<()>;
    async fn upsert_message(&self, msg: &MessageRecord) -> Result<Uuid>;
    async fn insert_reaction(&self, r: &ReactionRecord) -> Result<()>;
    async fn get_last_archived_ts(&self, channel_id: &str) -> Result<Option<String>>;
    async fn upsert_user(&self, u: &UserRecord) -> Result<()>;
    async fn upsert_channel(&self, c: &ChannelRecord) -> Result<()>;
    async fn get_top_threads(&self, limit: i64) -> Result<Vec<ThreadSummary>>;
    async fn get_recent_threads(&self, limit: i64) -> Result<Vec<ThreadSummary>>;
    async fn upsert_thread_weekly_score(&self, channel_id: &str, message_ts: &str) -> Result<()>;
    async fn enqueue_thread_aggregation(
        &self,
        channel_id: &str,
        message_ts: &str,
        requested_by: &str,
    ) -> Result<()>;
    async fn get_top_threads_with_weekly(&self, limit: i64) -> Result<Vec<ThreadWithWeeklyScore>>;
    async fn get_weekly_ranked_threads(&self, limit: i64) -> Result<Vec<PeriodRankedThread>>;
    async fn get_monthly_ranked_threads(&self, limit: i64) -> Result<Vec<PeriodRankedThread>>;
    async fn get_thread_messages(
        &self,
        channel_id: &str,
        thread_ts: &str,
    ) -> Result<Vec<ThreadMessage>>;
    async fn file_exists(&self, file_id: &str) -> Result<bool>;
    async fn insert_file(&self, f: &FileRecord) -> Result<()>;
    async fn enqueue_file_backfill_job(
        &self,
        team_id: &str,
        channel_id: &str,
        message_ts: &str,
        files_json: &serde_json::Value,
    ) -> Result<()>;
    async fn get_files_for_messages(
        &self,
        channel_id: &str,
        tss: &[String],
    ) -> Result<Vec<FileRow>>;
    /// Returns all known users as `(user_id, display_name)` pairs.
    async fn get_all_users(&self) -> Result<Vec<(String, String)>>;
    /// Returns all known channels as `(channel_id, name)` pairs.
    async fn get_all_channels(&self) -> Result<Vec<(String, String)>>;
}
