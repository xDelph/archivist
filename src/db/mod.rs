pub mod pool;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use anyhow::Result;
use chrono::{DateTime, Utc};
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
    async fn upsert_user(&self, u: &UserRecord) -> Result<()>;
    async fn upsert_channel(&self, c: &ChannelRecord) -> Result<()>;
    async fn get_top_threads(&self, limit: i64) -> Result<Vec<ThreadSummary>>;
    async fn get_thread_messages(
        &self,
        channel_id: &str,
        thread_ts: &str,
    ) -> Result<Vec<ThreadMessage>>;
    async fn file_exists(&self, file_id: &str) -> Result<bool>;
    async fn insert_file(&self, f: &FileRecord) -> Result<()>;
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

    async fn upsert_user(&self, u: &UserRecord) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO users (user_id, team_id, display_name, avatar_url)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE SET
                display_name = EXCLUDED.display_name,
                avatar_url   = EXCLUDED.avatar_url,
                cached_at    = NOW()
            "#,
            u.user_id,
            u.team_id,
            u.display_name,
            u.avatar_url,
        )
        .execute(self)
        .await?;
        Ok(())
    }

    async fn upsert_channel(&self, c: &ChannelRecord) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO channels (channel_id, team_id, name)
            VALUES ($1, $2, $3)
            ON CONFLICT (channel_id) DO UPDATE SET
                name      = EXCLUDED.name,
                cached_at = NOW()
            "#,
            c.channel_id,
            c.team_id,
            c.name,
        )
        .execute(self)
        .await?;
        Ok(())
    }

    async fn get_top_threads(&self, limit: i64) -> Result<Vec<ThreadSummary>> {
        let rows = sqlx::query!(
            r#"
            WITH stats AS (
                SELECT
                    m.channel_id,
                    m.ts                                                  AS thread_ts,
                    m.user_id,
                    m.text,
                    m.created_at,
                    -- Count reactions across every message in the thread (root + replies).
                    -- Uses raw_json['reactions'] (backfill data). Alias 'rr' avoids
                    -- conflict with the outer LEFT JOIN alias 'rep'.
                    -- jsonb_typeof guard prevents passing non-arrays to jsonb_array_elements.
                    -- Uses jsonb cast (rr_elem->'count') not text cast (->>'count') so
                    -- fractional JSON numbers don't cause a cast error.
                    COALESCE((
                        SELECT SUM((rr_elem->'count')::bigint)
                        FROM messages rr
                        CROSS JOIN LATERAL jsonb_array_elements(
                            CASE WHEN jsonb_typeof(rr.raw_json->'reactions') = 'array'
                                 THEN rr.raw_json->'reactions'
                                 ELSE '[]'::jsonb
                            END
                        ) AS rr_elem
                        WHERE rr.channel_id = m.channel_id
                          AND (rr.thread_ts = m.ts OR rr.ts = m.ts)
                    ), 0)                                                 AS reaction_count,
                    COALESCE(COUNT(DISTINCT rep.ts)::bigint, 0)           AS reply_count,
                    COALESCE(COUNT(DISTINCT rep.user_id)::bigint + 1, 1)  AS participant_count
                FROM messages m
                LEFT JOIN messages rep
                    ON rep.channel_id = m.channel_id
                   AND rep.thread_ts  = m.ts
                   AND rep.ts        != m.ts
                WHERE m.thread_ts = m.ts
                   OR (m.thread_ts IS NULL AND EXISTS (
                       SELECT 1 FROM messages r2
                       WHERE r2.channel_id = m.channel_id
                         AND r2.thread_ts  = m.ts
                         AND r2.ts        != m.ts
                   ))
                GROUP BY m.channel_id, m.ts, m.user_id, m.text, m.created_at
            )
            SELECT
                s.channel_id,
                COALESCE(ch.name, s.channel_id)         AS "channel_name!",
                s.thread_ts,
                s.text,
                s.created_at,
                COALESCE(u.display_name, s.user_id, '') AS "display_name!",
                COALESCE(u.avatar_url, '')               AS "avatar_url!",
                s.reaction_count                         AS "reaction_count!: i64",
                s.reply_count                            AS "reply_count!: i64",
                s.participant_count                      AS "participant_count!: i64",
                (s.reaction_count * 2
                    + s.reply_count
                    + s.participant_count)               AS "score!: i64"
            FROM stats s
            LEFT JOIN users    u  ON u.user_id    = s.user_id
            LEFT JOIN channels ch ON ch.channel_id = s.channel_id
            WHERE COALESCE(ch.name, s.channel_id) != 'intro'
            ORDER BY s.reaction_count * 2 + s.reply_count + s.participant_count DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ThreadSummary {
                channel_id: r.channel_id,
                channel_name: r.channel_name,
                thread_ts: r.thread_ts,
                text: r.text,
                created_at: r.created_at,
                display_name: r.display_name,
                avatar_url: r.avatar_url,
                reaction_count: r.reaction_count,
                reply_count: r.reply_count,
                participant_count: r.participant_count,
                score: r.score,
            })
            .collect())
    }

    async fn get_thread_messages(
        &self,
        channel_id: &str,
        thread_ts: &str,
    ) -> Result<Vec<ThreadMessage>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                m.ts,
                m.text,
                COALESCE(u.display_name, m.user_id, '') AS "display_name!",
                COALESCE(u.avatar_url, '')               AS "avatar_url!",
                COALESCE(
                    (SELECT json_agg(
                                jsonb_build_object('name', reaction_name, 'count', cnt)
                                ORDER BY cnt DESC
                            )
                     FROM (
                         SELECT reaction_name, COUNT(*)::int AS cnt
                         FROM   reactions
                         WHERE  channel_id = m.channel_id
                           AND  message_ts = m.ts
                         GROUP  BY reaction_name
                     ) rc)::jsonb,
                    '[]'::jsonb
                )                                        AS "reactions!: serde_json::Value"
            FROM messages m
            LEFT JOIN users u ON u.user_id = m.user_id
            WHERE m.channel_id = $1
              AND (m.thread_ts = $2 OR m.ts = $2)
            ORDER BY m.ts ASC
            "#,
            channel_id,
            thread_ts,
        )
        .fetch_all(self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ThreadMessage {
                ts: r.ts,
                text: r.text,
                display_name: r.display_name,
                avatar_url: r.avatar_url,
                reactions: r.reactions,
            })
            .collect())
    }

    async fn file_exists(&self, file_id: &str) -> Result<bool> {
        let row: Option<bool> = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM files WHERE file_id = $1)",
            file_id
        )
        .fetch_one(self)
        .await?;
        Ok(row.unwrap_or(false))
    }

    async fn insert_file(&self, f: &FileRecord) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO files
                (file_id, team_id, channel_id, message_ts, name, mimetype, size_bytes, storage_key, storage_url)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (file_id) DO NOTHING
            "#,
            f.file_id,
            f.team_id,
            f.channel_id,
            f.message_ts,
            f.name,
            f.mimetype,
            f.size_bytes,
            f.storage_key,
            f.storage_url,
        )
        .execute(self)
        .await?;
        Ok(())
    }

    async fn get_files_for_messages(
        &self,
        channel_id: &str,
        tss: &[String],
    ) -> Result<Vec<FileRow>> {
        let rows = sqlx::query!(
            r#"
            SELECT file_id, message_ts, name, mimetype, storage_url
            FROM files
            WHERE channel_id = $1 AND message_ts = ANY($2)
            ORDER BY cached_at ASC
            "#,
            channel_id,
            tss,
        )
        .fetch_all(self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| FileRow {
                file_id: r.file_id,
                message_ts: r.message_ts,
                name: r.name,
                mimetype: r.mimetype,
                storage_url: r.storage_url,
            })
            .collect())
    }

    async fn get_all_users(&self) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query!("SELECT user_id, display_name FROM users ORDER BY display_name")
            .fetch_all(self)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.user_id, r.display_name))
            .collect())
    }

    async fn get_all_channels(&self) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query!("SELECT channel_id, name FROM channels ORDER BY name")
            .fetch_all(self)
            .await?;
        Ok(rows.into_iter().map(|r| (r.channel_id, r.name)).collect())
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

    async fn upsert_user(&self, _u: &UserRecord) -> Result<()> {
        Ok(())
    }

    async fn upsert_channel(&self, _c: &ChannelRecord) -> Result<()> {
        Ok(())
    }

    async fn get_top_threads(&self, _limit: i64) -> Result<Vec<ThreadSummary>> {
        Ok(vec![])
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

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
