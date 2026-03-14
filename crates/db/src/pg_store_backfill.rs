use crate::{
    BackfillBatchStats, PgEventStore, StoreError,
    pg_materialized::refresh_thread_views,
    pg_store_backfill_sql::{
        attach_files, insert_message_events, insert_reaction_events, load_touched_roots,
        upsert_files, upsert_messages, upsert_reactions,
    },
    pg_support::upsert_channel,
};
use domain::{EventPayload, ProcessEventJob, SharedFile};
use serde::Serialize;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PreparedMessage {
    pub(crate) event_id: String,
    pub(crate) channel_id: String,
    pub(crate) channel_kind: String,
    pub(crate) event_time: i64,
    pub(crate) received_at: i64,
    pub(crate) ts: String,
    pub(crate) root_ts: String,
    pub(crate) user_id: Option<String>,
    pub(crate) text: String,
    pub(crate) raw_json: serde_json::Value,
    pub(crate) files: Vec<SharedFile>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PreparedReaction {
    pub(crate) event_id: String,
    pub(crate) channel_id: String,
    pub(crate) channel_kind: String,
    pub(crate) event_time: i64,
    pub(crate) received_at: i64,
    pub(crate) item_ts: String,
    pub(crate) user_id: String,
    pub(crate) reaction: String,
    pub(crate) raw_json: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub(crate) struct PreparedAttachment {
    pub(crate) channel_id: String,
    pub(crate) message_ts: String,
    pub(crate) file_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub(crate) struct PreparedFile {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) mimetype: Option<String>,
    pub(crate) permalink: Option<String>,
    pub(crate) size_bytes: Option<i64>,
}

impl PgEventStore {
    pub async fn backfill_channel_jobs(
        &self,
        channel_job: Option<&ProcessEventJob>,
        message_jobs: &[ProcessEventJob],
        reaction_jobs: &[ProcessEventJob],
    ) -> Result<BackfillBatchStats, StoreError> {
        let mut tx = self.pool().begin().await.map_err(StoreError::Sqlx)?;
        if let Some(job) = channel_job {
            apply_channel_job(&mut tx, job).await?;
        }

        let prepared_messages = prepare_messages(message_jobs);
        let prepared_reactions = prepare_reactions(reaction_jobs);
        let message_duplicates = load_existing_event_ids(
            &mut tx,
            prepared_messages
                .iter()
                .map(|row| row.event_id.clone())
                .collect(),
        )
        .await?;
        let reaction_duplicates = load_existing_event_ids(
            &mut tx,
            prepared_reactions
                .iter()
                .map(|row| row.event_id.clone())
                .collect(),
        )
        .await?;

        let new_messages = prepared_messages
            .iter()
            .filter(|row| !message_duplicates.contains(&row.event_id))
            .cloned()
            .collect::<Vec<_>>();
        let new_reactions = prepared_reactions
            .iter()
            .filter(|row| !reaction_duplicates.contains(&row.event_id))
            .cloned()
            .collect::<Vec<_>>();

        if !new_messages.is_empty() {
            insert_message_events(&mut tx, &new_messages).await?;
            upsert_messages(&mut tx, &new_messages).await?;
            upsert_files(&mut tx, &new_messages).await?;
            attach_files(&mut tx, &new_messages).await?;
        }

        if !new_reactions.is_empty() {
            insert_reaction_events(&mut tx, &new_reactions).await?;
            upsert_reactions(&mut tx, &new_reactions).await?;
        }

        let touched_roots = load_touched_roots(&mut tx, &new_messages, &new_reactions).await?;
        let channel_id = new_messages
            .first()
            .map(|row| row.channel_id.as_str())
            .or_else(|| new_reactions.first().map(|row| row.channel_id.as_str()));
        if let Some(channel_id) = channel_id {
            for root_ts in &touched_roots {
                refresh_thread_views(&mut tx, channel_id, root_ts).await?;
            }
        }

        tx.commit().await.map_err(StoreError::Sqlx)?;
        Ok(BackfillBatchStats {
            messages_inserted: new_messages.len(),
            messages_duplicate: message_duplicates.len(),
            reactions_inserted: new_reactions.len(),
            reactions_duplicate: reaction_duplicates.len(),
            refreshed_threads: touched_roots.len(),
        })
    }
}

fn prepare_messages(jobs: &[ProcessEventJob]) -> Vec<PreparedMessage> {
    jobs.iter()
        .filter_map(|job| match &job.payload {
            EventPayload::Message {
                user_id,
                text,
                ts,
                thread_ts,
                files,
            } => Some(PreparedMessage {
                event_id: job.event_id.clone(),
                channel_id: job.channel_id.clone(),
                channel_kind: job.channel_kind.as_str().to_owned(),
                event_time: job.event_time,
                received_at: job.received_at,
                ts: ts.clone(),
                root_ts: thread_ts.clone().unwrap_or_else(|| ts.clone()),
                user_id: user_id.clone(),
                text: text.clone().unwrap_or_default(),
                raw_json: json!(&job.payload),
                files: files.clone(),
            }),
            _ => None,
        })
        .collect()
}

fn prepare_reactions(jobs: &[ProcessEventJob]) -> Vec<PreparedReaction> {
    jobs.iter()
        .filter_map(|job| match &job.payload {
            EventPayload::ReactionAdded {
                user_id,
                reaction,
                item_ts,
            } => Some(PreparedReaction {
                event_id: job.event_id.clone(),
                channel_id: job.channel_id.clone(),
                channel_kind: job.channel_kind.as_str().to_owned(),
                event_time: job.event_time,
                received_at: job.received_at,
                item_ts: item_ts.clone(),
                user_id: user_id.clone(),
                reaction: reaction.clone(),
                raw_json: json!(&job.payload),
            }),
            _ => None,
        })
        .collect()
}

async fn apply_channel_job(
    tx: &mut Transaction<'_, Postgres>,
    job: &ProcessEventJob,
) -> Result<(), StoreError> {
    if let EventPayload::ChannelUpdated { name, is_archived } = &job.payload {
        upsert_channel(
            tx,
            &job.channel_id,
            job.channel_kind,
            name.clone(),
            is_archived.unwrap_or(false),
        )
        .await?;
    }
    Ok(())
}

async fn load_existing_event_ids(
    tx: &mut Transaction<'_, Postgres>,
    event_ids: Vec<String>,
) -> Result<HashSet<String>, StoreError> {
    if event_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT event_id
        FROM app_events
        WHERE event_id = ANY($1)
        "#,
    )
    .bind(event_ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;

    Ok(rows.into_iter().collect())
}
