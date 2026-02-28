use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use anyhow::Result;
use uuid::Uuid;

use super::models::{
    ChannelRecord, FileRecord, FileRow, MessageRecord, PeriodRankedThread, ReactionRecord,
    SlackEventRecord, ThreadMessage, ThreadSummary, ThreadWithWeeklyScore, UserRecord,
};
use super::repository::Repository;

type MessageStore = Mutex<HashMap<(String, String), (Uuid, MessageRecord)>>;
type ReactionKey = (String, String, String, String, String);

/// In-memory repository for unit tests. No database or network required.
#[derive(Default)]
pub struct InMemoryRepository {
    event_ids: Mutex<HashSet<String>>,
    // key: (channel_id, ts)
    pub(crate) messages: MessageStore,
    // key: (team_id, channel_id, message_ts, user_id, reaction_name)
    pub(crate) reactions: Mutex<HashSet<ReactionKey>>,
    pub threads: Mutex<Vec<ThreadSummary>>,
    pub weekly_upserts: Mutex<Vec<(String, String)>>,
    pub aggregation_enqueues: Mutex<Vec<(String, String, String)>>,
    pub top_threads_with_weekly: Mutex<Vec<ThreadWithWeeklyScore>>,
    pub weekly_ranked_threads: Mutex<Vec<PeriodRankedThread>>,
    pub monthly_ranked_threads: Mutex<Vec<PeriodRankedThread>>,
    pub fail_calls: Mutex<HashSet<String>>,
}

impl InMemoryRepository {
    fn should_fail(&self, call: &str) -> bool {
        self.fail_calls.lock().unwrap().contains(call)
    }
}

impl Repository for InMemoryRepository {
    async fn event_exists(&self, event_id: &str) -> Result<bool> {
        Ok(self.event_ids.lock().unwrap().contains(event_id))
    }

    async fn insert_slack_event(&self, rec: SlackEventRecord<'_>) -> Result<()> {
        self.event_ids
            .lock()
            .unwrap()
            .insert(rec.event_id.to_owned());
        Ok(())
    }

    async fn upsert_message(&self, msg: &MessageRecord) -> Result<Uuid> {
        let key = (msg.channel_id.clone(), msg.ts.clone());
        let mut store = self.messages.lock().unwrap();
        if let Some((existing_id, existing_msg)) = store.get_mut(&key) {
            existing_msg.text = msg.text.clone();
            existing_msg.edited_ts = msg.edited_ts.clone();
            existing_msg.deleted = msg.deleted;
            existing_msg.raw_json = msg.raw_json.clone();
            Ok(*existing_id)
        } else {
            let id = Uuid::new_v4();
            store.insert(
                key,
                (
                    id,
                    MessageRecord {
                        team_id: msg.team_id.clone(),
                        channel_id: msg.channel_id.clone(),
                        ts: msg.ts.clone(),
                        thread_ts: msg.thread_ts.clone(),
                        user_id: msg.user_id.clone(),
                        text: msg.text.clone(),
                        subtype: msg.subtype.clone(),
                        edited_ts: msg.edited_ts.clone(),
                        deleted: msg.deleted,
                        raw_json: msg.raw_json.clone(),
                    },
                ),
            );
            Ok(id)
        }
    }

    async fn insert_reaction(&self, r: &ReactionRecord) -> Result<()> {
        let key = (
            r.team_id.clone(),
            r.channel_id.clone(),
            r.message_ts.clone(),
            r.user_id.clone(),
            r.reaction_name.clone(),
        );
        self.reactions.lock().unwrap().insert(key);
        Ok(())
    }

    async fn get_last_archived_ts(&self, channel_id: &str) -> Result<Option<String>> {
        let store = self.messages.lock().unwrap();
        let max = store
            .keys()
            .filter(|(ch, _)| ch == channel_id)
            .map(|(_, ts)| ts.clone())
            .max();
        Ok(max)
    }

    async fn upsert_user(&self, _u: &UserRecord) -> Result<()> {
        Ok(())
    }

    async fn upsert_channel(&self, _c: &ChannelRecord) -> Result<()> {
        Ok(())
    }

    async fn get_top_threads(&self, limit: i64) -> Result<Vec<ThreadSummary>> {
        if self.should_fail("get_top_threads") {
            return Err(anyhow::anyhow!("forced failure: get_top_threads"));
        }
        let threads = self.threads.lock().unwrap();
        Ok(threads.iter().take(limit as usize).cloned().collect())
    }

    async fn get_recent_threads(&self, limit: i64) -> Result<Vec<ThreadSummary>> {
        if self.should_fail("get_recent_threads") {
            return Err(anyhow::anyhow!("forced failure: get_recent_threads"));
        }
        let mut threads = self.threads.lock().unwrap().clone();
        threads.sort_by(|a, b| {
            let a_ts = a.thread_ts.parse::<f64>().unwrap_or(0.0);
            let b_ts = b.thread_ts.parse::<f64>().unwrap_or(0.0);
            b_ts.partial_cmp(&a_ts).unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(threads.into_iter().take(limit as usize).collect())
    }

    async fn upsert_thread_weekly_score(&self, channel_id: &str, message_ts: &str) -> Result<()> {
        self.weekly_upserts
            .lock()
            .unwrap()
            .push((channel_id.to_owned(), message_ts.to_owned()));
        Ok(())
    }

    async fn enqueue_thread_aggregation(
        &self,
        channel_id: &str,
        message_ts: &str,
        requested_by: &str,
    ) -> Result<()> {
        self.aggregation_enqueues.lock().unwrap().push((
            channel_id.to_owned(),
            message_ts.to_owned(),
            requested_by.to_owned(),
        ));
        Ok(())
    }

    async fn get_top_threads_with_weekly(&self, limit: i64) -> Result<Vec<ThreadWithWeeklyScore>> {
        if self.should_fail("get_top_threads_with_weekly") {
            return Err(anyhow::anyhow!(
                "forced failure: get_top_threads_with_weekly"
            ));
        }
        let stored = self.top_threads_with_weekly.lock().unwrap();
        if !stored.is_empty() {
            return Ok(stored.iter().take(limit as usize).cloned().collect());
        }

        let threads = self.threads.lock().unwrap();
        Ok(threads
            .iter()
            .take(limit as usize)
            .cloned()
            .map(|thread| ThreadWithWeeklyScore {
                thread,
                score_week: 0,
            })
            .collect())
    }

    async fn get_weekly_ranked_threads(&self, limit: i64) -> Result<Vec<PeriodRankedThread>> {
        if self.should_fail("get_weekly_ranked_threads") {
            return Err(anyhow::anyhow!("forced failure: get_weekly_ranked_threads"));
        }
        let rows = self.weekly_ranked_threads.lock().unwrap();
        Ok(rows.iter().take(limit as usize).cloned().collect())
    }

    async fn get_monthly_ranked_threads(&self, limit: i64) -> Result<Vec<PeriodRankedThread>> {
        if self.should_fail("get_monthly_ranked_threads") {
            return Err(anyhow::anyhow!(
                "forced failure: get_monthly_ranked_threads"
            ));
        }
        let rows = self.monthly_ranked_threads.lock().unwrap();
        Ok(rows.iter().take(limit as usize).cloned().collect())
    }

    async fn get_thread_messages(
        &self,
        _channel_id: &str,
        _thread_ts: &str,
    ) -> Result<Vec<ThreadMessage>> {
        Ok(vec![])
    }

    async fn file_exists(&self, _file_id: &str) -> Result<bool> {
        Ok(false)
    }

    async fn insert_file(&self, _f: &FileRecord) -> Result<()> {
        Ok(())
    }

    async fn get_files_for_messages(
        &self,
        _channel_id: &str,
        _tss: &[String],
    ) -> Result<Vec<FileRow>> {
        Ok(vec![])
    }

    async fn get_all_users(&self) -> Result<Vec<(String, String)>> {
        Ok(vec![])
    }

    async fn get_all_channels(&self) -> Result<Vec<(String, String)>> {
        Ok(vec![])
    }
}
