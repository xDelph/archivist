use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::models::{
    ChannelRecord, FileRecord, FileRow, MessageRecord, PeriodRankedThread, ReactionRecord,
    SlackEventRecord, ThreadMessage, ThreadSummary, ThreadWithWeeklyScore, UserRecord,
};
use super::repository::Repository;

mod month;
mod recent;
mod top;
mod week;

pub(super) fn row_to_thread_summary(row: &sqlx::postgres::PgRow) -> Result<ThreadSummary> {
    Ok(ThreadSummary {
        channel_id: row.try_get("channel_id")?,
        channel_name: row.try_get("channel_name")?,
        thread_ts: row.try_get("thread_ts")?,
        user_id: row.try_get("user_id")?,
        text: row.try_get("text")?,
        created_at: row.try_get("created_at")?,
        display_name: row.try_get("display_name")?,
        avatar_url: row.try_get("avatar_url")?,
        search_text: row.try_get("search_text")?,
        reaction_count: row.try_get("reaction_count")?,
        reply_count: row.try_get("reply_count")?,
        participant_count: row.try_get("participant_count")?,
        file_count: row.try_get("file_count")?,
        score: row.try_get("score")?,
    })
}

fn parse_read_model_v2_value(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    !matches!(normalized.as_str(), "0" | "false" | "no" | "off")
}

pub(super) fn read_model_v2_enabled() -> bool {
    std::env::var("READ_MODEL_V2")
        .map(|value| parse_read_model_v2_value(&value))
        .unwrap_or(true)
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

    async fn get_recent_thread_roots(
        &self,
        channel_id: &str,
        oldest_ts: &str,
    ) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT m.thread_ts
            FROM messages m
            WHERE m.channel_id = $1
              AND m.ts >= $2
              AND m.thread_ts IS NOT NULL
              AND m.thread_ts <> ''
            "#,
        )
        .bind(channel_id)
        .bind(oldest_ts)
        .fetch_all(self)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("thread_ts")).collect())
    }

    async fn upsert_user(&self, u: &UserRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO users (
                user_id,
                team_id,
                display_name,
                avatar_url,
                email_ciphertext,
                email_lookup_hash,
                is_active,
                is_deleted,
                disabled_at,
                last_synced_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CASE WHEN $7 = FALSE OR $8 = TRUE THEN NOW() ELSE NULL END, NOW())
            ON CONFLICT (user_id) DO UPDATE SET
                display_name = EXCLUDED.display_name,
                avatar_url = EXCLUDED.avatar_url,
                email_ciphertext = COALESCE(EXCLUDED.email_ciphertext, users.email_ciphertext),
                email_lookup_hash = COALESCE(EXCLUDED.email_lookup_hash, users.email_lookup_hash),
                is_active = EXCLUDED.is_active,
                is_deleted = EXCLUDED.is_deleted,
                disabled_at = CASE
                    WHEN EXCLUDED.is_active = FALSE OR EXCLUDED.is_deleted = TRUE
                        THEN COALESCE(users.disabled_at, NOW())
                    ELSE NULL
                END,
                cached_at = NOW(),
                last_synced_at = NOW()
            "#,
        )
        .bind(&u.user_id)
        .bind(&u.team_id)
        .bind(&u.display_name)
        .bind(&u.avatar_url)
        .bind(&u.email_ciphertext)
        .bind(&u.email_lookup_hash)
        .bind(u.is_active)
        .bind(u.is_deleted)
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
        top::get_top_threads(self, limit).await
    }

    async fn get_recent_threads(&self, limit: i64) -> Result<Vec<ThreadSummary>> {
        recent::get_recent_threads(self, limit).await
    }

    async fn upsert_thread_weekly_score(&self, channel_id: &str, message_ts: &str) -> Result<()> {
        sqlx::query(
            r#"
            WITH resolved AS (
                SELECT
                    $1::text AS channel_id,
                    COALESCE(
                        (
                            SELECT CASE
                                WHEN m.thread_ts IS NULL OR m.thread_ts = '' THEN m.ts
                                ELSE m.thread_ts
                            END
                            FROM messages m
                            WHERE m.channel_id = $1
                              AND m.ts = $2
                            LIMIT 1
                        ),
                        $2::text
                    ) AS thread_ts,
                    date_trunc('week', CURRENT_DATE)::date AS week_start
            ),
            thread_messages AS (
                SELECT m.ts, m.thread_ts, m.user_id
                FROM messages m
                JOIN resolved r ON m.channel_id = r.channel_id
                WHERE m.ts = r.thread_ts OR m.thread_ts = r.thread_ts
            ),
            parts AS (
                SELECT
                    r.week_start,
                    r.channel_id,
                    r.thread_ts,
                    COALESCE((
                        SELECT COUNT(*)::bigint
                        FROM reactions re
                        JOIN thread_messages tm ON tm.ts = re.message_ts
                        WHERE re.channel_id = r.channel_id
                          AND re.event_ts ~ '^[0-9]+(\.[0-9]+)?$'
                          AND to_timestamp(re.event_ts::double precision) >= r.week_start::timestamp
                          AND to_timestamp(re.event_ts::double precision) < (r.week_start::timestamp + INTERVAL '7 days')
                    ), 0) AS reaction_week,
                    COALESCE((
                        SELECT COUNT(*)::bigint
                        FROM thread_messages tm
                        WHERE tm.thread_ts = r.thread_ts
                          AND tm.ts <> r.thread_ts
                          AND tm.ts ~ '^[0-9]+(\.[0-9]+)?$'
                          AND to_timestamp(tm.ts::double precision) >= r.week_start::timestamp
                          AND to_timestamp(tm.ts::double precision) < (r.week_start::timestamp + INTERVAL '7 days')
                    ), 0) AS reply_week,
                    COALESCE((
                        SELECT SUM((rr_elem->'count')::bigint)::bigint
                        FROM messages rr
                        CROSS JOIN LATERAL jsonb_array_elements(
                            CASE
                                WHEN jsonb_typeof(rr.raw_json->'reactions') = 'array'
                                THEN rr.raw_json->'reactions'
                                ELSE '[]'::jsonb
                            END
                        ) AS rr_elem
                        WHERE rr.channel_id = r.channel_id
                          AND (rr.thread_ts = r.thread_ts OR rr.ts = r.thread_ts)
                    ), 0) AS reaction_total,
                    COALESCE((
                        SELECT COUNT(*)::bigint
                        FROM thread_messages tm
                        WHERE tm.thread_ts = r.thread_ts
                          AND tm.ts <> r.thread_ts
                    ), 0) AS reply_total,
                    COALESCE((
                        SELECT COUNT(DISTINCT tm.user_id)::bigint + 1
                        FROM thread_messages tm
                        WHERE tm.thread_ts = r.thread_ts
                          AND tm.ts <> r.thread_ts
                    ), 1) AS participant_total
                FROM resolved r
            )
            INSERT INTO thread_weekly_scores
                (week_start, channel_id, thread_ts, score_week, score_total, updated_at)
            SELECT
                p.week_start,
                p.channel_id,
                p.thread_ts,
                (p.reaction_week * 2 + p.reply_week)::int,
                (p.reaction_total * 2 + p.reply_total + p.participant_total)::int,
                NOW()
            FROM parts p
            WHERE EXISTS (
                SELECT 1
                FROM messages m
                WHERE m.channel_id = p.channel_id
                  AND (m.ts = p.thread_ts OR m.thread_ts = p.thread_ts)
            )
            ON CONFLICT (week_start, channel_id, thread_ts)
            DO UPDATE SET
                score_week = EXCLUDED.score_week,
                score_total = EXCLUDED.score_total,
                updated_at = NOW()
            "#,
        )
        .bind(channel_id)
        .bind(message_ts)
        .execute(self)
        .await?;
        Ok(())
    }

    async fn enqueue_thread_aggregation(
        &self,
        channel_id: &str,
        message_ts: &str,
        requested_by: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            WITH resolved AS (
                SELECT
                    $1::text AS channel_id,
                    COALESCE(
                        (
                            SELECT CASE
                                WHEN m.thread_ts IS NULL OR m.thread_ts = '' THEN m.ts
                                ELSE m.thread_ts
                            END
                            FROM messages m
                            WHERE m.channel_id = $1
                              AND m.ts = $2
                            LIMIT 1
                        ),
                        $2::text
                    ) AS thread_ts
            )
            INSERT INTO aggregation_jobs
                (dedupe_key, job_kind, channel_id, thread_ts, status, requested_by, available_at, updated_at)
            SELECT
                ('thread_rollup:' || r.channel_id || ':' || r.thread_ts),
                'thread_rollup',
                r.channel_id,
                r.thread_ts,
                'queued',
                $3,
                NOW(),
                NOW()
            FROM resolved r
            ON CONFLICT (dedupe_key)
            DO UPDATE SET
                status = CASE
                    WHEN aggregation_jobs.status = 'running' THEN aggregation_jobs.status
                    ELSE 'queued'
                END,
                requested_by = EXCLUDED.requested_by,
                available_at = CASE
                    WHEN aggregation_jobs.status = 'running' THEN aggregation_jobs.available_at
                    ELSE NOW()
                END,
                finished_at = NULL,
                last_error = NULL,
                updated_at = NOW()
            "#,
        )
        .bind(channel_id)
        .bind(message_ts)
        .bind(requested_by)
        .execute(self)
        .await?;
        Ok(())
    }

    async fn get_top_threads_with_weekly(&self, limit: i64) -> Result<Vec<ThreadWithWeeklyScore>> {
        top::get_top_threads_with_weekly(self, limit).await
    }

    async fn get_weekly_ranked_threads(&self, limit: i64) -> Result<Vec<PeriodRankedThread>> {
        week::get_weekly_ranked_threads(self, limit).await
    }

    async fn get_monthly_ranked_threads(&self, limit: i64) -> Result<Vec<PeriodRankedThread>> {
        month::get_monthly_ranked_threads(self, limit).await
    }

    async fn get_thread_messages(
        &self,
        channel_id: &str,
        thread_ts: &str,
    ) -> Result<Vec<ThreadMessage>> {
        let rows = sqlx::query(
            r#"
            SELECT
                m.ts,
                COALESCE(m.user_id, '')                     AS user_id,
                m.text,
                CASE
                    WHEN COALESCE(u.is_active, TRUE) = FALSE
                      OR COALESCE(u.is_deleted, FALSE) = TRUE
                      OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                    THEN 'Anonymous'
                    ELSE COALESCE(u.display_name, m.user_id, '')
                END                                      AS display_name,
                CASE
                    WHEN COALESCE(u.is_active, TRUE) = FALSE
                      OR COALESCE(u.is_deleted, FALSE) = TRUE
                      OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                    THEN '/placeholder-user.jpg'
                    ELSE COALESCE(u.avatar_url, '')
                END                                      AS avatar_url,
                COALESCE(
                    (SELECT jsonb_agg(
                                jsonb_build_object('name', reaction_name, 'count', cnt)
                                ORDER BY cnt DESC
                            )
                     FROM (
                         SELECT reaction_name, COUNT(*)::int AS cnt
                         FROM   reactions
                         WHERE  channel_id = m.channel_id
                           AND  message_ts = m.ts
                         GROUP  BY reaction_name
                     ) rc),
                    (SELECT jsonb_agg(
                                jsonb_build_object('name', rr.name, 'count', rr.cnt)
                                ORDER BY rr.cnt DESC
                            )
                     FROM (
                         SELECT
                             rr_elem->>'name' AS name,
                             CASE
                                 WHEN (rr_elem->>'count') ~ '^[0-9]+$'
                                 THEN (rr_elem->>'count')::int
                                 ELSE 0
                             END AS cnt
                         FROM jsonb_array_elements(
                             CASE
                                 WHEN jsonb_typeof(m.raw_json->'reactions') = 'array'
                                 THEN m.raw_json->'reactions'
                                 ELSE '[]'::jsonb
                             END
                         ) AS rr_elem
                     ) rr
                     WHERE COALESCE(rr.name, '') <> ''
                       AND rr.cnt > 0),
                    '[]'::jsonb
                )                                        AS reactions
            FROM messages m
            LEFT JOIN users u ON u.user_id = m.user_id
            LEFT JOIN auth_accounts aa
                ON aa.slack_user_id = u.user_id
               AND aa.disabled_at IS NULL
            WHERE m.channel_id = $1
              AND (m.thread_ts = $2 OR m.ts = $2)
            ORDER BY m.ts ASC
            "#,
        )
        .bind(channel_id)
        .bind(thread_ts)
        .fetch_all(self)
        .await?;

        rows.into_iter()
            .map(|row| -> Result<ThreadMessage> {
                Ok(ThreadMessage {
                    ts: row.try_get("ts")?,
                    user_id: row.try_get("user_id")?,
                    text: row.try_get("text")?,
                    display_name: row.try_get("display_name")?,
                    avatar_url: row.try_get("avatar_url")?,
                    reactions: row.try_get("reactions")?,
                })
            })
            .collect::<Result<Vec<_>>>()
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

    async fn enqueue_file_backfill_job(
        &self,
        team_id: &str,
        channel_id: &str,
        message_ts: &str,
        files_json: &serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO file_backfill_jobs
                (dedupe_key, team_id, channel_id, message_ts, files_json, status, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, 'queued', NOW())
            ON CONFLICT (dedupe_key)
            DO UPDATE SET
                team_id = EXCLUDED.team_id,
                files_json = EXCLUDED.files_json,
                status = CASE
                    WHEN file_backfill_jobs.status = 'running' THEN file_backfill_jobs.status
                    ELSE 'queued'
                END,
                finished_at = NULL,
                last_error = NULL,
                updated_at = NOW()
            "#,
        )
        .bind(format!("{channel_id}:{message_ts}"))
        .bind(team_id)
        .bind(channel_id)
        .bind(message_ts)
        .bind(files_json)
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

#[cfg(test)]
mod tests {
    use super::parse_read_model_v2_value;

    #[test]
    fn parse_read_model_v2_value_handles_truthy_values() {
        assert!(parse_read_model_v2_value("true"));
        assert!(parse_read_model_v2_value("1"));
        assert!(parse_read_model_v2_value("yes"));
        assert!(parse_read_model_v2_value("on"));
        assert!(parse_read_model_v2_value("  TRUE  "));
        assert!(parse_read_model_v2_value("unexpected"));
    }

    #[test]
    fn parse_read_model_v2_value_handles_falsy_values() {
        assert!(!parse_read_model_v2_value("false"));
        assert!(!parse_read_model_v2_value("0"));
        assert!(!parse_read_model_v2_value("no"));
        assert!(!parse_read_model_v2_value("off"));
        assert!(!parse_read_model_v2_value("  Off  "));
    }
}
