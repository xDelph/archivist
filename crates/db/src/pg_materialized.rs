use crate::{
    SearchDocumentRow, StoreError, ThreadSummaryRow,
    pg_support::{map_message_row, normalize_empty, slack_ts_seconds},
    search_index::{MessageMap, SearchDocumentMap, refresh_search_documents},
    thread_summary_index::build_thread_summaries,
};
use domain::{File, Reaction};
use sqlx::{Row, Transaction};
use std::collections::{HashMap, HashSet};

type FileKey = (String, String, String);
type ReactionKey = (String, String, String, String);

pub(crate) async fn refresh_thread_views(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    channel_id: &str,
    root_ts: &str,
) -> Result<(), StoreError> {
    let messages = load_thread_messages(tx, channel_id, root_ts).await?;
    let reactions = load_thread_reactions(tx, channel_id, root_ts).await?;
    let files = load_thread_files(tx, channel_id, root_ts).await?;
    let message_map = messages
        .into_iter()
        .map(|message| ((message.channel_id.clone(), message.ts.clone()), message))
        .collect::<MessageMap>();
    let root_key = (channel_id.to_owned(), root_ts.to_owned());
    let reaction_set = reactions
        .into_iter()
        .map(|reaction| {
            (
                reaction.channel_id,
                reaction.message_ts,
                reaction.user_id,
                reaction.name,
            )
        })
        .collect::<HashSet<ReactionKey>>();
    let file_map = files
        .into_iter()
        .map(|file| {
            (
                (
                    file.channel_id.clone(),
                    file.message_ts.clone(),
                    file.id.clone(),
                ),
                file,
            )
        })
        .collect::<HashMap<FileKey, _>>();

    // Thread read models are root-keyed, so replies arriving before the root must not
    // materialize rows that would violate root-message foreign keys.
    if !message_map.contains_key(&root_key) {
        tracing::debug!(
            channel_id = %channel_id,
            root_ts = %root_ts,
            "skipping thread view materialization until root message exists"
        );
        sqlx::query(
            r#"
            DELETE FROM search_documents
            WHERE channel_id = $1 AND root_ts = $2
            "#,
        )
        .bind(channel_id)
        .bind(root_ts)
        .execute(&mut **tx)
        .await
        .map_err(StoreError::Sqlx)?;
        sqlx::query(
            r#"
            DELETE FROM thread_summaries
            WHERE channel_id = $1 AND root_ts = $2
            "#,
        )
        .bind(channel_id)
        .bind(root_ts)
        .execute(&mut **tx)
        .await
        .map_err(StoreError::Sqlx)?;
        return Ok(());
    }

    let mut search_documents = SearchDocumentMap::new();
    refresh_search_documents(&mut search_documents, &message_map, channel_id, root_ts);
    let summary = build_thread_summaries(&message_map, &reaction_set, &file_map)
        .remove(&(channel_id.to_owned(), root_ts.to_owned()));

    sqlx::query(
        r#"
        DELETE FROM search_documents
        WHERE channel_id = $1 AND root_ts = $2
        "#,
    )
    .bind(channel_id)
    .bind(root_ts)
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;
    for row in search_documents.values() {
        upsert_search_document(tx, row).await?;
    }

    sqlx::query(
        r#"
        DELETE FROM thread_summaries
        WHERE channel_id = $1 AND root_ts = $2
        "#,
    )
    .bind(channel_id)
    .bind(root_ts)
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;
    if let Some(row) = summary.as_ref() {
        upsert_thread_summary(tx, row).await?;
    }

    Ok(())
}

pub(crate) async fn upsert_search_document(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    row: &SearchDocumentRow,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
        INSERT INTO search_documents (
            channel_id,
            root_ts,
            message_ts,
            title,
            body,
            document,
            message_occurred_at
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            to_tsvector('english', btrim(coalesce($4, '') || ' ' || $5)),
            to_timestamp($6)
        )
        ON CONFLICT (channel_id, message_ts) DO UPDATE
        SET root_ts = EXCLUDED.root_ts,
            title = EXCLUDED.title,
            body = EXCLUDED.body,
            document = EXCLUDED.document,
            message_occurred_at = EXCLUDED.message_occurred_at,
            updated_at = now()
        "#,
    )
    .bind(&row.channel_id)
    .bind(&row.root_ts)
    .bind(&row.message_ts)
    .bind(&row.title)
    .bind(&row.body)
    .bind(slack_ts_seconds(&row.message_ts))
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;

    Ok(())
}

pub(crate) async fn upsert_thread_summary(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    row: &ThreadSummaryRow,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
        INSERT INTO thread_summaries (
            channel_id,
            root_ts,
            title,
            preview,
            reply_count,
            participant_count,
            reaction_count,
            file_count,
            root_message_at,
            last_activity_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($9), to_timestamp($10))
        ON CONFLICT (channel_id, root_ts) DO UPDATE
        SET title = EXCLUDED.title,
            preview = EXCLUDED.preview,
            reply_count = EXCLUDED.reply_count,
            participant_count = EXCLUDED.participant_count,
            reaction_count = EXCLUDED.reaction_count,
            file_count = EXCLUDED.file_count,
            root_message_at = EXCLUDED.root_message_at,
            last_activity_at = EXCLUDED.last_activity_at,
            updated_at = now()
        "#,
    )
    .bind(&row.channel_id)
    .bind(&row.root_ts)
    .bind(&row.title)
    .bind(&row.preview)
    .bind(row.reply_count)
    .bind(row.participant_count)
    .bind(row.reaction_count)
    .bind(row.file_count)
    .bind(slack_ts_seconds(&row.root_ts))
    .bind(slack_ts_seconds(&row.last_activity_ts))
    .execute(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;

    Ok(())
}

async fn load_thread_messages(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    channel_id: &str,
    root_ts: &str,
) -> Result<Vec<domain::Message>, StoreError> {
    let rows = sqlx::query(
        r#"
        SELECT channel_id, ts, root_ts, user_id, text
        FROM messages
        WHERE channel_id = $1 AND root_ts = $2
        ORDER BY ts ASC
        "#,
    )
    .bind(channel_id)
    .bind(root_ts)
    .fetch_all(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;

    Ok(rows.into_iter().map(map_message_row).collect())
}

async fn load_thread_reactions(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    channel_id: &str,
    root_ts: &str,
) -> Result<Vec<Reaction>, StoreError> {
    let rows = sqlx::query(
        r#"
        SELECT reactions.channel_id, reactions.message_ts, reactions.user_id, reactions.name
        FROM reactions
        JOIN messages
            ON messages.channel_id = reactions.channel_id
           AND messages.ts = reactions.message_ts
        WHERE reactions.channel_id = $1
          AND messages.root_ts = $2
        ORDER BY reactions.message_ts ASC, reactions.user_id ASC, reactions.name ASC
        "#,
    )
    .bind(channel_id)
    .bind(root_ts)
    .fetch_all(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;

    Ok(rows
        .into_iter()
        .map(|row| Reaction {
            channel_id: row.get("channel_id"),
            message_ts: row.get("message_ts"),
            user_id: row.get("user_id"),
            name: row.get("name"),
        })
        .collect())
}

async fn load_thread_files(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    channel_id: &str,
    root_ts: &str,
) -> Result<Vec<File>, StoreError> {
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
        JOIN messages
            ON messages.channel_id = message_files.channel_id
           AND messages.ts = message_files.message_ts
        WHERE message_files.channel_id = $1
          AND messages.root_ts = $2
        ORDER BY message_files.message_ts ASC, files.id ASC
        "#,
    )
    .bind(channel_id)
    .bind(root_ts)
    .fetch_all(&mut **tx)
    .await
    .map_err(StoreError::Sqlx)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let storage_url: Option<String> = row.get("storage_url");
            let permalink: Option<String> = row.get("permalink");
            File {
                id: row.get("id"),
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
