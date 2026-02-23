pub mod pool;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

// ── Records ──────────────────────────────────────────────────────────────────

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

// ── Repository trait ─────────────────────────────────────────────────────────

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
}

// ── PgPool implementation ─────────────────────────────────────────────────────

impl Repository for PgPool {
    async fn event_exists(&self, event_id: &str) -> Result<bool> {
        let row: Option<bool> = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM slack_events WHERE event_id = $1)",
            event_id
        )
        .fetch_one(self)
        .await?;
        Ok(row.unwrap_or(false))
    }

    async fn insert_slack_event(&self, rec: SlackEventRecord<'_>) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO slack_events (event_id, team_id, event_time, payload_json)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (event_id) DO NOTHING
            "#,
            rec.event_id,
            rec.team_id,
            rec.event_time,
            rec.payload_json,
        )
        .execute(self)
        .await?;
        Ok(())
    }

    async fn upsert_message(&self, msg: &MessageRecord) -> Result<Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO messages
                (team_id, channel_id, ts, thread_ts, user_id, text, subtype, edited_ts, deleted, raw_json)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (channel_id, ts) DO UPDATE SET
                text       = EXCLUDED.text,
                edited_ts  = EXCLUDED.edited_ts,
                deleted    = EXCLUDED.deleted,
                raw_json   = EXCLUDED.raw_json,
                updated_at = NOW()
            RETURNING id
            "#,
            msg.team_id,
            msg.channel_id,
            msg.ts,
            msg.thread_ts,
            msg.user_id,
            msg.text,
            msg.subtype,
            msg.edited_ts,
            msg.deleted,
            msg.raw_json,
        )
        .fetch_one(self)
        .await?;
        Ok(id)
    }

    async fn insert_reaction(&self, r: &ReactionRecord) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO reactions
                (team_id, channel_id, message_ts, user_id, reaction_name, event_ts)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (team_id, channel_id, message_ts, user_id, reaction_name) DO NOTHING
            "#,
            r.team_id,
            r.channel_id,
            r.message_ts,
            r.user_id,
            r.reaction_name,
            r.event_ts,
        )
        .execute(self)
        .await?;
        Ok(())
    }

    async fn get_last_archived_ts(&self, channel_id: &str) -> Result<Option<String>> {
        let ts = sqlx::query_scalar!(
            "SELECT MAX(ts) FROM messages WHERE channel_id = $1",
            channel_id
        )
        .fetch_one(self)
        .await?;
        Ok(ts)
    }
}

// ── InMemoryRepository (test double) ─────────────────────────────────────────

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
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
