use crate::{
    RepositoryHealth, SearchDocumentRow, StoreError, StoreOutcome, ThreadSummaryRow,
    pg_materialized::{refresh_thread_views, upsert_search_document, upsert_thread_summary},
    pg_support::{
        count_rows, event_type, map_message_row, message_root_ts, normalize_empty,
        parse_channel_kind, thread_root_ts, upsert_channel,
    },
    search_index::{MessageMap, SearchDocumentMap, refresh_search_documents},
    thread_summary_index::build_thread_summaries,
};
use domain::{Channel, EventPayload, File, ProcessEventJob, Reaction};
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
    workspace_id: String,
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

        Ok(Self {
            pool,
            workspace_id: std::env::var("SLACK_WORKSPACE_ID").unwrap_or_default(),
        })
    }

    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    pub async fn health(&self) -> Result<RepositoryHealth, StoreError> {
        Ok(RepositoryHealth {
            tracked_events: count_rows(&self.pool, "app_events").await?,
            tracked_messages: count_rows(&self.pool, "messages").await?,
            tracked_reactions: count_rows(&self.pool, "reactions").await?,
            tracked_files: count_rows(&self.pool, "message_files").await?,
            tracked_channels: count_rows(&self.pool, "channels").await?,
        })
    }

    pub async fn record_process_event(
        &self,
        job: &ProcessEventJob,
    ) -> Result<StoreOutcome, StoreError> {
        let mut tx = self.pool.begin().await.map_err(StoreError::Sqlx)?;
        let inserted = sqlx::query(
            r#"
            INSERT INTO app_events (
                event_id,
                channel_id,
                channel_kind,
                event_type,
                occurred_at,
                received_at,
                payload_json
            )
            VALUES ($1, $2, $3, $4, to_timestamp($5), to_timestamp($6), $7)
            ON CONFLICT (event_id) DO NOTHING
            "#,
        )
        .bind(&job.event_id)
        .bind(&job.channel_id)
        .bind(job.channel_kind.as_str())
        .bind(event_type(&job.payload))
        .bind(job.event_time as f64)
        .bind(job.received_at as f64)
        .bind(json!(&job.payload))
        .execute(&mut *tx)
        .await
        .map_err(StoreError::Sqlx)?
        .rows_affected()
            > 0;
        if !inserted {
            tx.rollback().await.map_err(StoreError::Sqlx)?;
            return Ok(StoreOutcome::Duplicate);
        }

        upsert_channel(&mut tx, &job.channel_id, job.channel_kind, None, false).await?;

        let affected_root_ts = match &job.payload {
            EventPayload::Message {
                user_id,
                text,
                ts,
                thread_ts,
                files,
            } => {
                let root_ts = thread_root_ts(ts, thread_ts.as_deref());
                sqlx::query(
                    r#"
                    INSERT INTO messages (
                        channel_id,
                        ts,
                        root_ts,
                        user_id,
                        text,
                        raw_json,
                        occurred_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7))
                    ON CONFLICT (channel_id, ts) DO UPDATE
                    SET root_ts = EXCLUDED.root_ts,
                        user_id = EXCLUDED.user_id,
                        text = EXCLUDED.text,
                        raw_json = EXCLUDED.raw_json,
                        occurred_at = EXCLUDED.occurred_at,
                        updated_at = now()
                    "#,
                )
                .bind(&job.channel_id)
                .bind(ts)
                .bind(&root_ts)
                .bind(user_id)
                .bind(text.clone().unwrap_or_default())
                .bind(json!(&job.payload))
                .bind(ts.parse::<f64>().unwrap_or_default())
                .execute(&mut *tx)
                .await
                .map_err(StoreError::Sqlx)?;
                for file in files {
                    sqlx::query(
                        r#"
                        INSERT INTO files (
                            id,
                            name,
                            mimetype,
                            permalink,
                            size_bytes,
                            storage_key,
                            storage_url
                        )
                        VALUES ($1, $2, $3, $4, $5, '', '')
                        ON CONFLICT (id) DO UPDATE
                        SET name = EXCLUDED.name,
                            mimetype = EXCLUDED.mimetype,
                            permalink = EXCLUDED.permalink,
                            size_bytes = EXCLUDED.size_bytes,
                            updated_at = now()
                        "#,
                    )
                    .bind(&file.id)
                    .bind(&file.name)
                    .bind(&file.mimetype)
                    .bind(&file.permalink)
                    .bind(file.size.map(|value| value as i64))
                    .execute(&mut *tx)
                    .await
                    .map_err(StoreError::Sqlx)?;
                    sqlx::query(
                        r#"
                        INSERT INTO message_files (channel_id, message_ts, file_id)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (channel_id, message_ts, file_id) DO NOTHING
                        "#,
                    )
                    .bind(&job.channel_id)
                    .bind(ts)
                    .bind(&file.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(StoreError::Sqlx)?;
                }
                Some(root_ts)
            }
            EventPayload::ReactionAdded {
                user_id,
                reaction,
                item_ts,
            } => {
                sqlx::query(
                    r#"
                    INSERT INTO reactions (channel_id, message_ts, user_id, name, occurred_at)
                    VALUES ($1, $2, $3, $4, to_timestamp($5))
                    ON CONFLICT (channel_id, message_ts, user_id, name) DO NOTHING
                    "#,
                )
                .bind(&job.channel_id)
                .bind(item_ts)
                .bind(user_id)
                .bind(reaction)
                .bind(job.event_time as f64)
                .execute(&mut *tx)
                .await
                .map_err(StoreError::Sqlx)?;
                message_root_ts(&mut tx, &job.channel_id, item_ts).await?
            }
            EventPayload::ChannelUpdated { name, is_archived } => {
                upsert_channel(
                    &mut tx,
                    &job.channel_id,
                    job.channel_kind,
                    name.clone(),
                    is_archived.unwrap_or(false),
                )
                .await?;
                None
            }
        };
        if let Some(root_ts) = affected_root_ts {
            refresh_thread_views(&mut tx, &self.workspace_id, &job.channel_id, &root_ts).await?;
        }
        tx.commit().await.map_err(StoreError::Sqlx)?;
        Ok(StoreOutcome::Inserted)
    }

    pub async fn messages(&self) -> Result<Vec<domain::Message>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT channel_id, ts, root_ts, user_id, text
            FROM messages
            ORDER BY channel_id ASC, ts ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| map_message_row(&self.workspace_id, row))
            .collect())
    }

    pub async fn reactions(&self) -> Result<Vec<Reaction>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT channel_id, message_ts, user_id, name
            FROM reactions
            ORDER BY channel_id ASC, message_ts ASC, user_id ASC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| Reaction {
                team_id: self.workspace_id.clone(),
                channel_id: row.get("channel_id"),
                message_ts: row.get("message_ts"),
                user_id: row.get("user_id"),
                name: row.get("name"),
            })
            .collect())
    }

    pub async fn files(&self) -> Result<Vec<File>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT
                message_files.channel_id,
                message_files.message_ts,
                files.id,
                files.name,
                files.mimetype,
                files.permalink,
                files.size_bytes,
                files.storage_url
            FROM message_files
            JOIN files ON files.id = message_files.file_id
            ORDER BY message_files.channel_id ASC, message_files.message_ts ASC, files.id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let storage_url: Option<String> = row.get("storage_url");
                let permalink: Option<String> = row.get("permalink");
                File {
                    id: row.get("id"),
                    team_id: self.workspace_id.clone(),
                    channel_id: row.get("channel_id"),
                    message_ts: row.get("message_ts"),
                    name: row.get("name"),
                    mimetype: row.get("mimetype"),
                    permalink: storage_url.or(permalink).and_then(normalize_empty),
                    size: row
                        .get::<Option<i64>, _>("size_bytes")
                        .and_then(|value| (value > 0).then_some(value as u64)),
                }
            })
            .collect())
    }

    pub async fn channels(&self) -> Result<Vec<Channel>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, name, is_archived
            FROM channels
            ORDER BY id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| Channel {
                team_id: self.workspace_id.clone(),
                id: row.get("id"),
                kind: parse_channel_kind(row.get("kind")),
                name: row.get("name"),
                is_archived: row.get("is_archived"),
            })
            .collect())
    }

    pub async fn search_documents(&self) -> Result<Vec<SearchDocumentRow>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT
                channel_id,
                root_ts,
                message_ts,
                title,
                body,
                to_char(
                    message_occurred_at AT TIME ZONE 'UTC',
                    'YYYY-MM-DD"T"HH24:MI:SS"Z"'
                ) AS message_occurred_at
            FROM search_documents
            ORDER BY channel_id ASC, message_ts ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| SearchDocumentRow {
                team_id: self.workspace_id.clone(),
                channel_id: row.get("channel_id"),
                root_ts: row.get("root_ts"),
                message_ts: row.get("message_ts"),
                title: row.get("title"),
                body: row.get("body"),
                message_occurred_at: row.get("message_occurred_at"),
            })
            .collect())
    }

    pub async fn thread_summaries(&self) -> Result<Vec<ThreadSummaryRow>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT
                channel_id,
                root_ts,
                title,
                preview,
                reply_count,
                participant_count,
                reaction_count,
                file_count,
                CAST(EXTRACT(EPOCH FROM root_message_at) AS bigint)::text AS root_message_at,
                CAST(EXTRACT(EPOCH FROM last_activity_at) AS bigint)::text AS last_activity_ts
            FROM thread_summaries
            ORDER BY channel_id ASC, root_ts ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| ThreadSummaryRow {
                team_id: self.workspace_id.clone(),
                channel_id: row.get("channel_id"),
                root_ts: row.get("root_ts"),
                title: row.get("title"),
                preview: row.get("preview"),
                reply_count: row.get("reply_count"),
                participant_count: row.get("participant_count"),
                reaction_count: row.get("reaction_count"),
                file_count: row.get("file_count"),
                root_message_at: row.get("root_message_at"),
                last_activity_ts: row.get("last_activity_ts"),
            })
            .collect())
    }

    pub async fn refresh_thread_summaries(&self) -> Result<usize, StoreError> {
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
            .collect::<MessageMap>();
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
        let thread_summaries = build_thread_summaries(&message_map, &reaction_set, &file_map);
        let mut search_documents = SearchDocumentMap::new();
        for summary in thread_summaries.values() {
            refresh_search_documents(
                &mut search_documents,
                &message_map,
                &summary.team_id,
                &summary.channel_id,
                &summary.root_ts,
            );
        }

        let mut tx = self.pool.begin().await.map_err(StoreError::Sqlx)?;
        sqlx::query("DELETE FROM search_documents")
            .execute(&mut *tx)
            .await
            .map_err(StoreError::Sqlx)?;
        for row in search_documents.values() {
            upsert_search_document(&mut tx, row).await?;
        }
        sqlx::query("DELETE FROM thread_summaries")
            .execute(&mut *tx)
            .await
            .map_err(StoreError::Sqlx)?;
        for row in thread_summaries.values() {
            upsert_thread_summary(&mut tx, row).await?;
        }
        tx.commit().await.map_err(StoreError::Sqlx)?;
        Ok(thread_summaries.len())
    }
}
