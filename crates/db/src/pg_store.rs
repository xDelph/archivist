use crate::{
    RepositoryHealth, SearchDocumentRow, StoreError, StoreOutcome, ThreadSummaryRow,
    thread_summary_index::build_thread_summaries,
};
use domain::{Channel, ChannelKind, EventPayload, File, Message, ProcessEventJob, Reaction};
use serde_json::json;
use sqlx::{
    PgPool, Row,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

type FileKey = (String, String);
type ReactionKey = (String, String, String, String, String);

#[derive(Debug, Clone)]
pub struct PgEventStore {
    pool: PgPool,
}

impl PgEventStore {
    pub async fn open(database_url: &str) -> Result<Self, StoreError> {
        let options = PgConnectOptions::from_str(database_url)
            .map_err(StoreError::Sqlx)?
            .statement_cache_capacity(0);
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(StoreError::Sqlx)?;

        Ok(Self { pool })
    }

    pub async fn health(&self) -> Result<RepositoryHealth, StoreError> {
        Ok(RepositoryHealth {
            tracked_events: count_rows(&self.pool, "slack_events").await?,
            tracked_messages: count_rows(&self.pool, "messages").await?,
            tracked_reactions: count_rows(&self.pool, "reactions").await?,
            tracked_files: count_rows(&self.pool, "files").await?,
            tracked_channels: count_rows(&self.pool, "channels").await?,
        })
    }

    pub async fn record_process_event(
        &self,
        job: &ProcessEventJob,
    ) -> Result<StoreOutcome, StoreError> {
        let has_slack_events = table_exists(&self.pool, "slack_events").await?;
        let mut tx = self.pool.begin().await.map_err(StoreError::Sqlx)?;
        let payload_json = json!(job);
        let inserted = if has_slack_events {
            sqlx::query(
                r#"
                INSERT INTO slack_events (event_id, team_id, event_time, payload_json)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (event_id) DO NOTHING
                "#,
            )
            .bind(&job.event_id)
            .bind(&job.team_id)
            .bind(job.event_time)
            .bind(payload_json)
            .execute(&mut *tx)
            .await
            .map_err(StoreError::Sqlx)?
            .rows_affected()
                > 0
        } else {
            true
        };

        if !inserted {
            tx.rollback().await.map_err(StoreError::Sqlx)?;
            return Ok(StoreOutcome::Duplicate);
        }

        match &job.payload {
            EventPayload::Message {
                user_id,
                text,
                ts,
                thread_ts,
                files,
            } => {
                sqlx::query(
                    r#"
                    INSERT INTO messages (
                        team_id,
                        channel_id,
                        ts,
                        thread_ts,
                        user_id,
                        text,
                        subtype,
                        edited_ts,
                        deleted,
                        raw_json
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, NULL, NULL, FALSE, $7)
                    ON CONFLICT (channel_id, ts) DO UPDATE
                    SET thread_ts = EXCLUDED.thread_ts,
                        user_id = EXCLUDED.user_id,
                        text = EXCLUDED.text,
                        raw_json = EXCLUDED.raw_json,
                        updated_at = NOW()
                    "#,
                )
                .bind(&job.team_id)
                .bind(&job.channel_id)
                .bind(ts)
                .bind(thread_ts)
                .bind(user_id)
                .bind(text.clone().unwrap_or_default())
                .bind(json!(job.payload))
                .execute(&mut *tx)
                .await
                .map_err(StoreError::Sqlx)?;

                for file in files {
                    sqlx::query(
                        r#"
                        INSERT INTO files (
                            file_id,
                            team_id,
                            channel_id,
                            message_ts,
                            name,
                            mimetype,
                            size_bytes,
                            storage_key,
                            storage_url
                        )
                        VALUES ($1, $2, $3, $4, $5, $6, $7, '', $8)
                        ON CONFLICT (file_id) DO UPDATE
                        SET team_id = EXCLUDED.team_id,
                            channel_id = EXCLUDED.channel_id,
                            message_ts = EXCLUDED.message_ts,
                            name = EXCLUDED.name,
                            mimetype = EXCLUDED.mimetype,
                            size_bytes = EXCLUDED.size_bytes,
                            storage_url = EXCLUDED.storage_url,
                            cached_at = NOW()
                        "#,
                    )
                    .bind(&file.id)
                    .bind(&job.team_id)
                    .bind(&job.channel_id)
                    .bind(ts)
                    .bind(&file.name)
                    .bind(file.mimetype.clone().unwrap_or_default())
                    .bind(file.size.unwrap_or_default() as i64)
                    .bind(file.permalink.clone().unwrap_or_default())
                    .execute(&mut *tx)
                    .await
                    .map_err(StoreError::Sqlx)?;
                }
            }
            EventPayload::ReactionAdded {
                user_id,
                reaction,
                item_ts,
            } => {
                sqlx::query(
                    r#"
                    INSERT INTO reactions (
                        team_id,
                        channel_id,
                        message_ts,
                        user_id,
                        reaction_name,
                        event_ts
                    )
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (team_id, channel_id, message_ts, user_id, reaction_name) DO NOTHING
                    "#,
                )
                .bind(&job.team_id)
                .bind(&job.channel_id)
                .bind(item_ts)
                .bind(user_id)
                .bind(reaction)
                .bind(job.event_time.to_string())
                .execute(&mut *tx)
                .await
                .map_err(StoreError::Sqlx)?;
            }
            EventPayload::ChannelUpdated { name, .. } => {
                sqlx::query(
                    r#"
                    INSERT INTO channels (channel_id, team_id, name)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (channel_id) DO UPDATE
                    SET team_id = EXCLUDED.team_id,
                        name = EXCLUDED.name,
                        cached_at = NOW()
                    "#,
                )
                .bind(&job.channel_id)
                .bind(&job.team_id)
                .bind(name.clone().unwrap_or_default())
                .execute(&mut *tx)
                .await
                .map_err(StoreError::Sqlx)?;
            }
        }

        tx.commit().await.map_err(StoreError::Sqlx)?;
        Ok(StoreOutcome::Inserted)
    }

    pub async fn messages(&self) -> Result<Vec<Message>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT team_id, channel_id, ts, thread_ts, user_id, text
            FROM messages
            ORDER BY channel_id ASC, ts ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;

        Ok(rows
            .into_iter()
            .map(|row| Message {
                team_id: row.get("team_id"),
                channel_id: row.get("channel_id"),
                ts: row.get("ts"),
                thread_ts: row.get("thread_ts"),
                user_id: row.get("user_id"),
                text: row.get("text"),
            })
            .collect())
    }

    pub async fn reactions(&self) -> Result<Vec<Reaction>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT team_id, channel_id, message_ts, user_id, reaction_name
            FROM reactions
            ORDER BY channel_id ASC, message_ts ASC, user_id ASC, reaction_name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;

        Ok(rows
            .into_iter()
            .map(|row| Reaction {
                team_id: row.get("team_id"),
                channel_id: row.get("channel_id"),
                message_ts: row.get("message_ts"),
                user_id: row.get("user_id"),
                name: row.get("reaction_name"),
            })
            .collect())
    }

    pub async fn files(&self) -> Result<Vec<File>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT file_id, team_id, channel_id, message_ts, name, mimetype, size_bytes, storage_url
            FROM files
            ORDER BY channel_id ASC, message_ts ASC, file_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let storage_url: String = row.get("storage_url");
                let mimetype: String = row.get("mimetype");
                let size_bytes: i64 = row.get("size_bytes");

                File {
                    id: row.get("file_id"),
                    team_id: row.get("team_id"),
                    channel_id: row.get("channel_id"),
                    message_ts: row.get("message_ts"),
                    name: row.get("name"),
                    mimetype: normalize_empty(mimetype),
                    permalink: normalize_empty(storage_url),
                    size: (size_bytes > 0).then_some(size_bytes as u64),
                }
            })
            .collect())
    }

    pub async fn channels(&self) -> Result<Vec<Channel>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT channel_id, team_id, name
            FROM channels
            ORDER BY channel_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let channel_id: String = row.get("channel_id");
                let name: String = row.get("name");

                Channel {
                    team_id: row.get("team_id"),
                    id: channel_id.clone(),
                    kind: ChannelKind::from_channel_id(&channel_id),
                    name: normalize_empty(name),
                    is_archived: false,
                }
            })
            .collect())
    }

    pub async fn search_documents(&self) -> Result<Vec<SearchDocumentRow>, StoreError> {
        let rows = sqlx::query(
            r#"
            WITH root_messages AS (
                SELECT team_id, channel_id, ts AS root_ts, text AS root_text
                FROM messages
                WHERE thread_ts IS NULL
            )
            SELECT
                messages.team_id,
                messages.channel_id,
                messages.ts AS message_ts,
                COALESCE(root_messages.root_text, messages.text) AS title,
                messages.text AS body
            FROM messages
            LEFT JOIN root_messages
                ON root_messages.team_id = messages.team_id
               AND root_messages.channel_id = messages.channel_id
               AND root_messages.root_ts = COALESCE(messages.thread_ts, messages.ts)
            ORDER BY messages.channel_id ASC, messages.ts ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;

        Ok(rows
            .into_iter()
            .map(|row| SearchDocumentRow {
                team_id: row.get("team_id"),
                channel_id: row.get("channel_id"),
                message_ts: row.get("message_ts"),
                title: normalize_empty(row.get::<String, _>("title")),
                body: row.get("body"),
            })
            .collect())
    }

    pub async fn thread_summaries(&self) -> Result<Vec<ThreadSummaryRow>, StoreError> {
        let messages = self.messages().await?;
        let reactions = self.reactions().await?;
        let files = self.files().await?;
        let message_map = messages
            .into_iter()
            .map(|message| {
                (
                    (
                        message.team_id.clone(),
                        message.channel_id.clone(),
                        message.ts.clone(),
                    ),
                    message,
                )
            })
            .collect::<HashMap<_, _>>();
        let reaction_set = reactions
            .into_iter()
            .map(|reaction| {
                (
                    reaction.team_id,
                    reaction.channel_id,
                    reaction.message_ts,
                    reaction.user_id,
                    reaction.name,
                )
            })
            .collect::<HashSet<ReactionKey>>();
        let file_map = files
            .into_iter()
            .map(|file| ((file.team_id.clone(), file.id.clone()), file))
            .collect::<HashMap<FileKey, _>>();
        let mut summaries = build_thread_summaries(&message_map, &reaction_set, &file_map)
            .into_values()
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| {
            (&left.channel_id, &left.root_ts).cmp(&(&right.channel_id, &right.root_ts))
        });
        Ok(summaries)
    }

    pub async fn refresh_thread_summaries(&self) -> Result<usize, StoreError> {
        Ok(self.thread_summaries().await?.len())
    }
}

async fn count_rows(pool: &PgPool, table: &str) -> Result<usize, StoreError> {
    if !table_exists(pool, table).await? {
        return Ok(0);
    }

    let query = format!("SELECT COUNT(*)::bigint AS count FROM {table}");
    let count = sqlx::query_scalar::<_, i64>(&query)
        .fetch_one(pool)
        .await
        .map_err(StoreError::Sqlx)?;
    Ok(count.max(0) as usize)
}

async fn table_exists(pool: &PgPool, table: &str) -> Result<bool, StoreError> {
    sqlx::query_scalar::<_, bool>("SELECT to_regclass($1) IS NOT NULL")
        .bind(table)
        .fetch_one(pool)
        .await
        .map_err(StoreError::Sqlx)
}

fn normalize_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}
