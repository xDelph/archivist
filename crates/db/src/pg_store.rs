use crate::{
    RepositoryHealth, StoreError, StoreOutcome,
    pg_materialized::refresh_thread_views,
    pg_support::{count_rows, event_type, message_root_ts, thread_root_ts, upsert_channel},
};
use domain::{EventPayload, ProcessEventJob};
use serde_json::json;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::str::FromStr;

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
            refresh_thread_views(&mut tx, &job.channel_id, &root_ts).await?;
        }
        tx.commit().await.map_err(StoreError::Sqlx)?;
        Ok(StoreOutcome::Inserted)
    }
}
