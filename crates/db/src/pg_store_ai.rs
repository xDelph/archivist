use crate::{GeneratedThreadSummaryRow, PgEventStore, StoreError};

impl PgEventStore {
    pub async fn generated_thread_summaries(
        &self,
    ) -> Result<Vec<GeneratedThreadSummaryRow>, StoreError> {
        sqlx::query_as::<_, GeneratedThreadSummaryRow>(
            r#"
            SELECT
                channel_id,
                root_ts,
                summary,
                full_summary,
                why_it_mattered,
                status,
                topic_tags,
                source_last_activity_ts,
                model,
                CAST(EXTRACT(EPOCH FROM generated_at) AS bigint) AS generated_at
            FROM generated_thread_summaries
            ORDER BY channel_id ASC, root_ts ASC
            "#,
        )
        .fetch_all(&self.pool())
        .await
        .map_err(StoreError::Sqlx)
    }

    pub async fn upsert_generated_thread_summary(
        &self,
        row: &GeneratedThreadSummaryRow,
    ) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            INSERT INTO generated_thread_summaries (
                channel_id,
                root_ts,
                summary,
                full_summary,
                why_it_mattered,
                status,
                topic_tags,
                source_last_activity_ts,
                model,
                generated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, to_timestamp($10))
            ON CONFLICT (channel_id, root_ts) DO UPDATE
            SET summary = EXCLUDED.summary,
                full_summary = EXCLUDED.full_summary,
                why_it_mattered = EXCLUDED.why_it_mattered,
                status = EXCLUDED.status,
                topic_tags = EXCLUDED.topic_tags,
                source_last_activity_ts = EXCLUDED.source_last_activity_ts,
                model = EXCLUDED.model,
                generated_at = EXCLUDED.generated_at,
                updated_at = now()
            "#,
        )
        .bind(&row.channel_id)
        .bind(&row.root_ts)
        .bind(&row.summary)
        .bind(&row.full_summary)
        .bind(&row.why_it_mattered)
        .bind(&row.status)
        .bind(&row.topic_tags)
        .bind(&row.source_last_activity_ts)
        .bind(&row.model)
        .bind(row.generated_at as f64)
        .execute(&self.pool())
        .await
        .map_err(StoreError::Sqlx)?;

        Ok(())
    }
}
