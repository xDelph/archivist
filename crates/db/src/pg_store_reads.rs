use crate::{
    PgEventStore, SearchDocumentRow, StoreError, ThreadSummaryRow,
    pg_materialized::{upsert_search_document, upsert_thread_summary},
    pg_support::{map_message_row, normalize_empty, parse_channel_kind},
    search_index::{MessageMap, SearchDocumentMap, refresh_search_documents},
    thread_summary_index::build_thread_summaries,
};
use domain::{Channel, File, Reaction};
use sqlx::Row;
use std::collections::{HashMap, HashSet};

type FileKey = (String, String, String);
type ReactionKey = (String, String, String, String);

impl PgEventStore {
    pub async fn latest_message_ts(&self, channel_id: &str) -> Result<Option<String>, StoreError> {
        let row = sqlx::query(
            r#"
            SELECT ts
            FROM messages
            WHERE channel_id = $1
            ORDER BY occurred_at DESC, ts DESC
            LIMIT 1
            "#,
        )
        .bind(channel_id)
        .fetch_optional(&self.pool())
        .await
        .map_err(StoreError::Sqlx)?;

        Ok(row.map(|row| row.get("ts")))
    }

    pub async fn messages(&self) -> Result<Vec<domain::Message>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT channel_id, ts, root_ts, user_id, text
            FROM messages
            ORDER BY channel_id ASC, ts ASC
            "#,
        )
        .fetch_all(&self.pool())
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows.into_iter().map(map_message_row).collect())
    }

    pub async fn reactions(&self) -> Result<Vec<Reaction>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT channel_id, message_ts, user_id, name
            FROM reactions
            ORDER BY channel_id ASC, message_ts ASC, user_id ASC, name ASC
            "#,
        )
        .fetch_all(&self.pool())
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
        .fetch_all(&self.pool())
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

    pub async fn channels(&self) -> Result<Vec<Channel>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, name, is_archived
            FROM channels
            ORDER BY id ASC
            "#,
        )
        .fetch_all(&self.pool())
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| Channel {
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
        .fetch_all(&self.pool())
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| SearchDocumentRow {
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
                reply_count,
                participant_count,
                reaction_count,
                file_count,
                to_char(
                    EXTRACT(EPOCH FROM root_message_at),
                    'FM999999999999999.000000'
                ) AS root_message_at,
                to_char(
                    EXTRACT(EPOCH FROM last_activity_at),
                    'FM999999999999999.000000'
                ) AS last_activity_ts
            FROM thread_summaries
            ORDER BY channel_id ASC, root_ts ASC
            "#,
        )
        .fetch_all(&self.pool())
        .await
        .map_err(StoreError::Sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| ThreadSummaryRow {
                channel_id: row.get("channel_id"),
                root_ts: row.get("root_ts"),
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
            .map(|message| ((message.channel_id.clone(), message.ts.clone()), message))
            .collect::<MessageMap>();
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
        let thread_summaries = build_thread_summaries(&message_map, &reaction_set, &file_map);
        let mut search_documents = SearchDocumentMap::new();
        for summary in thread_summaries.values() {
            refresh_search_documents(
                &mut search_documents,
                &message_map,
                &summary.channel_id,
                &summary.root_ts,
            );
        }

        let mut tx = self.pool().begin().await.map_err(StoreError::Sqlx)?;
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

    pub async fn upsert_user_profile(
        &self,
        user_id: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
        is_active: bool,
    ) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            INSERT INTO users (id, email, display_name, avatar_url, is_active)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE
            SET email = EXCLUDED.email,
                display_name = EXCLUDED.display_name,
                avatar_url = EXCLUDED.avatar_url,
                is_active = EXCLUDED.is_active,
                last_synced_at = now()
            "#,
        )
        .bind(user_id)
        .bind(email)
        .bind(display_name)
        .bind(avatar_url)
        .bind(is_active)
        .execute(&self.pool())
        .await
        .map_err(StoreError::Sqlx)?;

        Ok(())
    }

    pub async fn set_file_archive(
        &self,
        file_id: &str,
        storage_key: &str,
        storage_url: &str,
    ) -> Result<(), StoreError> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE files
            SET storage_key = $2,
                storage_url = $3,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(file_id)
        .bind(storage_key)
        .bind(storage_url)
        .execute(&self.pool())
        .await
        .map_err(StoreError::Sqlx)?
        .rows_affected();
        if rows_affected == 0 {
            return Err(StoreError::Sqlx(sqlx::Error::RowNotFound));
        }

        Ok(())
    }
}
