use crate::{
    StoreError,
    pg_store_backfill::{PreparedAttachment, PreparedFile, PreparedMessage, PreparedReaction},
};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use std::collections::HashSet;

pub(crate) async fn insert_message_events(
    tx: &mut Transaction<'_, Postgres>,
    rows: &[PreparedMessage],
) -> Result<(), StoreError> {
    if rows.is_empty() {
        return Ok(());
    }

    sqlx::query(
        r#"
        WITH data AS (
            SELECT *
            FROM jsonb_to_recordset($1::jsonb) AS x(
                event_id text,
                channel_id text,
                channel_kind text,
                event_time bigint,
                received_at bigint,
                ts text,
                root_ts text,
                user_id text,
                text text,
                raw_json jsonb
            )
        )
        INSERT INTO app_events (
            event_id,
            channel_id,
            channel_kind,
            event_type,
            occurred_at,
            received_at,
            payload_json
        )
        SELECT
            event_id,
            channel_id,
            channel_kind,
            'message',
            to_timestamp(event_time),
            to_timestamp(received_at),
            raw_json
        FROM data
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(json!(rows))
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;
    Ok(())
}

pub(crate) async fn upsert_messages(
    tx: &mut Transaction<'_, Postgres>,
    rows: &[PreparedMessage],
) -> Result<(), StoreError> {
    if rows.is_empty() {
        return Ok(());
    }

    sqlx::query(
        r#"
        WITH data AS (
            SELECT *
            FROM jsonb_to_recordset($1::jsonb) AS x(
                event_id text,
                channel_id text,
                channel_kind text,
                event_time bigint,
                received_at bigint,
                ts text,
                root_ts text,
                user_id text,
                text text,
                raw_json jsonb
            )
        )
        INSERT INTO messages (channel_id, ts, root_ts, user_id, text, raw_json, occurred_at)
        SELECT
            channel_id,
            ts,
            root_ts,
            user_id,
            text,
            raw_json,
            to_timestamp((ts)::double precision)
        FROM data
        ON CONFLICT (channel_id, ts) DO UPDATE
        SET root_ts = EXCLUDED.root_ts,
            user_id = EXCLUDED.user_id,
            text = EXCLUDED.text,
            raw_json = EXCLUDED.raw_json,
            occurred_at = EXCLUDED.occurred_at,
            updated_at = now()
        "#,
    )
    .bind(json!(rows))
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;
    Ok(())
}

pub(crate) async fn upsert_files(
    tx: &mut Transaction<'_, Postgres>,
    rows: &[PreparedMessage],
) -> Result<(), StoreError> {
    let files = rows
        .iter()
        .flat_map(|message| {
            message.files.iter().map(|file| PreparedFile {
                id: file.id.clone(),
                name: file.name.clone(),
                mimetype: file.mimetype.clone(),
                permalink: file.permalink.clone(),
                size_bytes: file.size.map(|value| value as i64),
            })
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Ok(());
    }

    sqlx::query(
        r#"
        WITH data AS (
            SELECT *
            FROM jsonb_to_recordset($1::jsonb) AS x(
                id text,
                name text,
                mimetype text,
                permalink text,
                size_bytes bigint
            )
        )
        INSERT INTO files (id, name, mimetype, permalink, size_bytes, storage_key, storage_url)
        SELECT id, name, mimetype, permalink, size_bytes, '', ''
        FROM data
        ON CONFLICT (id) DO UPDATE
        SET name = EXCLUDED.name,
            mimetype = EXCLUDED.mimetype,
            permalink = EXCLUDED.permalink,
            size_bytes = EXCLUDED.size_bytes,
            updated_at = now()
        "#,
    )
    .bind(json!(files))
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;
    Ok(())
}

pub(crate) async fn attach_files(
    tx: &mut Transaction<'_, Postgres>,
    rows: &[PreparedMessage],
) -> Result<(), StoreError> {
    let attachments = rows
        .iter()
        .flat_map(|message| {
            message.files.iter().map(|file| PreparedAttachment {
                channel_id: message.channel_id.clone(),
                message_ts: message.ts.clone(),
                file_id: file.id.clone(),
            })
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if attachments.is_empty() {
        return Ok(());
    }

    sqlx::query(
        r#"
        WITH data AS (
            SELECT *
            FROM jsonb_to_recordset($1::jsonb) AS x(
                channel_id text,
                message_ts text,
                file_id text
            )
        )
        INSERT INTO message_files (channel_id, message_ts, file_id)
        SELECT channel_id, message_ts, file_id
        FROM data
        ON CONFLICT (channel_id, message_ts, file_id) DO NOTHING
        "#,
    )
    .bind(json!(attachments))
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;
    Ok(())
}

pub(crate) async fn insert_reaction_events(
    tx: &mut Transaction<'_, Postgres>,
    rows: &[PreparedReaction],
) -> Result<(), StoreError> {
    if rows.is_empty() {
        return Ok(());
    }

    sqlx::query(
        r#"
        WITH data AS (
            SELECT *
            FROM jsonb_to_recordset($1::jsonb) AS x(
                event_id text,
                channel_id text,
                channel_kind text,
                event_time bigint,
                received_at bigint,
                item_ts text,
                user_id text,
                reaction text,
                raw_json jsonb
            )
        )
        INSERT INTO app_events (
            event_id,
            channel_id,
            channel_kind,
            event_type,
            occurred_at,
            received_at,
            payload_json
        )
        SELECT
            event_id,
            channel_id,
            channel_kind,
            'reaction_added',
            to_timestamp(event_time),
            to_timestamp(received_at),
            raw_json
        FROM data
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(json!(rows))
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;
    Ok(())
}

pub(crate) async fn upsert_reactions(
    tx: &mut Transaction<'_, Postgres>,
    rows: &[PreparedReaction],
) -> Result<(), StoreError> {
    if rows.is_empty() {
        return Ok(());
    }

    sqlx::query(
        r#"
        WITH data AS (
            SELECT *
            FROM jsonb_to_recordset($1::jsonb) AS x(
                event_id text,
                channel_id text,
                channel_kind text,
                event_time bigint,
                received_at bigint,
                item_ts text,
                user_id text,
                reaction text,
                raw_json jsonb
            )
        )
        INSERT INTO reactions (channel_id, message_ts, user_id, name, occurred_at)
        SELECT
            channel_id,
            item_ts,
            user_id,
            reaction,
            to_timestamp(event_time)
        FROM data
        ON CONFLICT (channel_id, message_ts, user_id, name) DO NOTHING
        "#,
    )
    .bind(json!(rows))
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;
    Ok(())
}

pub(crate) async fn load_touched_roots(
    tx: &mut Transaction<'_, Postgres>,
    message_rows: &[PreparedMessage],
    reaction_rows: &[PreparedReaction],
) -> Result<Vec<String>, StoreError> {
    let mut roots = message_rows
        .iter()
        .map(|row| row.root_ts.clone())
        .collect::<HashSet<_>>();

    let channel_id = message_rows
        .first()
        .map(|row| row.channel_id.as_str())
        .or_else(|| reaction_rows.first().map(|row| row.channel_id.as_str()));
    if let Some(channel_id) = channel_id {
        let reaction_message_timestamps = reaction_rows
            .iter()
            .map(|row| row.item_ts.clone())
            .collect::<Vec<_>>();
        if !reaction_message_timestamps.is_empty() {
            let reaction_roots = sqlx::query_scalar::<_, String>(
                r#"
                SELECT DISTINCT root_ts
                FROM messages
                WHERE channel_id = $1
                  AND ts = ANY($2)
                "#,
            )
            .bind(channel_id)
            .bind(reaction_message_timestamps)
            .fetch_all(&mut **tx)
            .await
            .map_err(StoreError::Sqlx)?;
            roots.extend(reaction_roots);
        }
    }

    let mut roots = roots.into_iter().collect::<Vec<_>>();
    roots.sort();
    Ok(roots)
}
