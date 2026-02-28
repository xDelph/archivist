use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::models::{
    ChannelRecord, FileRecord, FileRow, MessageRecord, PeriodRankedThread, ReactionRecord,
    SlackEventRecord, ThreadMessage, ThreadSummary, ThreadWithWeeklyScore, UserRecord,
};
use super::repository::Repository;

fn row_to_thread_summary(row: &sqlx::postgres::PgRow) -> ThreadSummary {
    ThreadSummary {
        channel_id: row.get("channel_id"),
        channel_name: row.get("channel_name"),
        thread_ts: row.get("thread_ts"),
        text: row.get("text"),
        created_at: row.get("created_at"),
        display_name: row.get("display_name"),
        avatar_url: row.get("avatar_url"),
        search_text: row.get("search_text"),
        reaction_count: row.get("reaction_count"),
        reply_count: row.get("reply_count"),
        participant_count: row.get("participant_count"),
        file_count: row.get("file_count"),
        score: row.get("score"),
    }
}

fn read_model_v2_enabled() -> bool {
    std::env::var("READ_MODEL_V2")
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
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
        if read_model_v2_enabled() {
            let rollup_rows = sqlx::query(
                r#"
                SELECT
                    tr.channel_id,
                    COALESCE(ch.name, tr.channel_id)         AS channel_name,
                    tr.thread_ts,
                    tr.root_text                              AS text,
                    tr.search_text                            AS search_text,
                    tr.root_created_at                        AS created_at,
                    COALESCE(u.display_name, tr.root_user_id, '') AS display_name,
                    COALESCE(u.avatar_url, '')               AS avatar_url,
                    tr.reaction_count_total                   AS reaction_count,
                    tr.reply_count_total                      AS reply_count,
                    tr.participant_count_total                AS participant_count,
                    tr.file_count_total                       AS file_count,
                    tr.score_total                            AS score
                FROM thread_rollups tr
                LEFT JOIN users u
                    ON u.user_id = tr.root_user_id
                LEFT JOIN channels ch
                    ON ch.channel_id = tr.channel_id
                WHERE COALESCE(ch.name, tr.channel_id) != 'intro'
                ORDER BY tr.score_total DESC, tr.channel_id, tr.thread_ts
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(self)
            .await?;

            if !rollup_rows.is_empty() {
                return Ok(rollup_rows
                    .into_iter()
                    .map(|row| row_to_thread_summary(&row))
                    .collect());
            }
        }

        let rows = sqlx::query(
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
                        SELECT SUM((rr_elem->'count')::bigint)::bigint
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
                ''::text                                  AS "search_text!",
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
        )
        .bind(limit)
        .fetch_all(self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ThreadSummary {
                channel_id: r.get("channel_id"),
                channel_name: r.get("channel_name"),
                thread_ts: r.get("thread_ts"),
                text: r.get("text"),
                created_at: r.get("created_at"),
                display_name: r.get("display_name"),
                avatar_url: r.get("avatar_url"),
                search_text: r.get("search_text"),
                reaction_count: r.get("reaction_count"),
                reply_count: r.get("reply_count"),
                participant_count: r.get("participant_count"),
                file_count: -1,
                score: r.get("score"),
            })
            .collect())
    }

    async fn get_recent_threads(&self, limit: i64) -> Result<Vec<ThreadSummary>> {
        if read_model_v2_enabled() {
            let rows = sqlx::query(
                r#"
                SELECT
                    tr.channel_id,
                    COALESCE(ch.name, tr.channel_id)              AS channel_name,
                    tr.thread_ts,
                    tr.root_text                                   AS text,
                    tr.search_text                                 AS search_text,
                    tr.root_created_at                             AS created_at,
                    COALESCE(u.display_name, tr.root_user_id, '') AS display_name,
                    COALESCE(u.avatar_url, '')                    AS avatar_url,
                    tr.reaction_count_total                        AS reaction_count,
                    tr.reply_count_total                           AS reply_count,
                    tr.participant_count_total                     AS participant_count,
                    tr.file_count_total                            AS file_count,
                    tr.score_total                                 AS score
                FROM thread_rollups tr
                LEFT JOIN users u
                    ON u.user_id = tr.root_user_id
                LEFT JOIN channels ch
                    ON ch.channel_id = tr.channel_id
                WHERE COALESCE(ch.name, tr.channel_id) != 'intro'
                ORDER BY tr.thread_ts DESC, tr.channel_id, tr.thread_ts
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(self)
            .await?;

            if !rows.is_empty() {
                return Ok(rows
                    .into_iter()
                    .map(|row| row_to_thread_summary(&row))
                    .collect());
            }
        }

        let rows = sqlx::query(
            r#"
            WITH roots AS (
                SELECT
                    m.channel_id,
                    m.ts                                                  AS thread_ts,
                    m.user_id,
                    m.text,
                    m.created_at,
                    COALESCE((
                        SELECT SUM((rr_elem->'count')::bigint)::bigint
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
                    COALESCE((
                        SELECT COUNT(*)::bigint
                        FROM messages rep
                        WHERE rep.channel_id = m.channel_id
                          AND rep.thread_ts = m.ts
                          AND rep.ts <> m.ts
                    ), 0)                                                 AS reply_count,
                    COALESCE((
                        SELECT COUNT(DISTINCT rep.user_id)::bigint + 1
                        FROM messages rep
                        WHERE rep.channel_id = m.channel_id
                          AND rep.thread_ts = m.ts
                          AND rep.ts <> m.ts
                    ), 1)                                                 AS participant_count
                FROM messages m
                WHERE m.thread_ts = m.ts
                   OR (m.thread_ts IS NULL AND EXISTS (
                       SELECT 1 FROM messages r2
                       WHERE r2.channel_id = m.channel_id
                         AND r2.thread_ts  = m.ts
                         AND r2.ts        != m.ts
                   ))
            )
            SELECT
                r.channel_id,
                COALESCE(ch.name, r.channel_id)                           AS channel_name,
                r.thread_ts,
                r.text,
                ''::text                                                   AS search_text,
                r.created_at,
                COALESCE(u.display_name, r.user_id, '')                   AS display_name,
                COALESCE(u.avatar_url, '')                                AS avatar_url,
                r.reaction_count,
                r.reply_count,
                r.participant_count,
                -1::bigint                                                 AS file_count,
                (r.reaction_count * 2 + r.reply_count + r.participant_count) AS score
            FROM roots r
            LEFT JOIN users u
                ON u.user_id = r.user_id
            LEFT JOIN channels ch
                ON ch.channel_id = r.channel_id
            WHERE COALESCE(ch.name, r.channel_id) != 'intro'
            ORDER BY r.thread_ts DESC, r.channel_id, r.thread_ts
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| row_to_thread_summary(&row))
            .collect())
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
        if read_model_v2_enabled() {
            let rows = sqlx::query(
                r#"
                SELECT
                    tr.channel_id,
                    COALESCE(ch.name, tr.channel_id)              AS channel_name,
                    tr.thread_ts,
                    tr.root_text                                   AS text,
                    tr.search_text                                 AS search_text,
                    tr.root_created_at                             AS created_at,
                    COALESCE(u.display_name, tr.root_user_id, '') AS display_name,
                    COALESCE(u.avatar_url, '')                    AS avatar_url,
                    tr.reaction_count_total                        AS reaction_count,
                    tr.reply_count_total                           AS reply_count,
                    tr.participant_count_total                     AS participant_count,
                    tr.file_count_total                            AS file_count,
                    tr.score_total                                 AS score,
                    COALESCE(tps.score_period, 0)::bigint         AS score_week
                FROM thread_rollups tr
                LEFT JOIN users u
                    ON u.user_id = tr.root_user_id
                LEFT JOIN channels ch
                    ON ch.channel_id = tr.channel_id
                LEFT JOIN thread_period_scores tps
                    ON tps.period_kind = 'week'
                   AND tps.period_start = date_trunc('week', CURRENT_DATE)::date
                   AND tps.channel_id = tr.channel_id
                   AND tps.thread_ts = tr.thread_ts
                WHERE COALESCE(ch.name, tr.channel_id) != 'intro'
                ORDER BY tr.score_total DESC, tr.channel_id, tr.thread_ts
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(self)
            .await?;

            if !rows.is_empty() {
                return Ok(rows
                    .into_iter()
                    .map(|row| ThreadWithWeeklyScore {
                        thread: row_to_thread_summary(&row),
                        score_week: row.get("score_week"),
                    })
                    .collect());
            }
        }

        let rows = sqlx::query(
            r#"
            WITH stats AS (
                SELECT
                    m.channel_id,
                    m.ts                                                  AS thread_ts,
                    m.user_id,
                    m.text,
                    m.created_at,
                    COALESCE((
                        SELECT SUM((rr_elem->'count')::bigint)::bigint
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
                COALESCE(ch.name, s.channel_id)                           AS channel_name,
                s.thread_ts,
                s.text,
                ''::text                                                AS search_text,
                s.created_at,
                COALESCE(u.display_name, s.user_id, '')                   AS display_name,
                COALESCE(u.avatar_url, '')                                 AS avatar_url,
                s.reaction_count,
                s.reply_count,
                s.participant_count,
                -1::bigint                                              AS file_count,
                (s.reaction_count * 2 + s.reply_count + s.participant_count) AS score,
                COALESCE(tws.score_week, 0)::bigint                        AS score_week
            FROM stats s
            LEFT JOIN users u
                ON u.user_id = s.user_id
            LEFT JOIN channels ch
                ON ch.channel_id = s.channel_id
            LEFT JOIN thread_weekly_scores tws
                ON tws.channel_id = s.channel_id
               AND tws.thread_ts = s.thread_ts
               AND tws.week_start = date_trunc('week', CURRENT_DATE)::date
            WHERE COALESCE(ch.name, s.channel_id) != 'intro'
            ORDER BY s.reaction_count * 2 + s.reply_count + s.participant_count DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ThreadWithWeeklyScore {
                thread: row_to_thread_summary(&row),
                score_week: row.get("score_week"),
            })
            .collect())
    }

    async fn get_weekly_ranked_threads(&self, limit: i64) -> Result<Vec<PeriodRankedThread>> {
        if read_model_v2_enabled() {
            let rows = sqlx::query(
                r#"
                SELECT
                    tr.channel_id,
                    COALESCE(ch.name, tr.channel_id)              AS channel_name,
                    tr.thread_ts,
                    tr.root_text                                   AS text,
                    tr.search_text                                 AS search_text,
                    tr.root_created_at                             AS created_at,
                    COALESCE(u.display_name, tr.root_user_id, '') AS display_name,
                    COALESCE(u.avatar_url, '')                    AS avatar_url,
                    tr.reaction_count_total                        AS reaction_count,
                    tr.reply_count_total                           AS reply_count,
                    tr.participant_count_total                     AS participant_count,
                    tr.file_count_total                            AS file_count,
                    tps.score_period                               AS score,
                    tps.score_period                               AS rank_score,
                    RANK() OVER (
                        ORDER BY tps.score_period DESC, tps.channel_id, tps.thread_ts
                    )::bigint                                      AS rank,
                    NULL::bigint                                   AS prev_rank
                FROM thread_period_scores tps
                JOIN thread_rollups tr
                    ON tr.channel_id = tps.channel_id
                   AND tr.thread_ts = tps.thread_ts
                LEFT JOIN users u
                    ON u.user_id = tr.root_user_id
                LEFT JOIN channels ch
                    ON ch.channel_id = tr.channel_id
                WHERE tps.period_kind = 'week'
                  AND tps.period_start = date_trunc('week', CURRENT_DATE)::date
                  AND COALESCE(ch.name, tr.channel_id) != 'intro'
                ORDER BY rank ASC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(self)
            .await?;

            if !rows.is_empty() {
                return Ok(rows
                    .into_iter()
                    .map(|row| PeriodRankedThread {
                        thread: row_to_thread_summary(&row),
                        rank_score: row.get("rank_score"),
                        rank: row.get("rank"),
                        prev_rank: row.get("prev_rank"),
                    })
                    .collect());
            }
        }

        let rows = sqlx::query(
            r#"
            WITH stats AS (
                SELECT
                    m.channel_id,
                    m.ts                                                  AS thread_ts,
                    m.user_id,
                    m.text,
                    m.created_at,
                    COALESCE((
                        SELECT SUM((rr_elem->'count')::bigint)::bigint
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
            ),
            bounds AS (
                SELECT
                    date_trunc('week', CURRENT_DATE)::timestamp AS period_start,
                    (date_trunc('week', CURRENT_DATE)::timestamp + INTERVAL '7 days') AS period_end
            ),
            cur AS (
                SELECT
                    s.channel_id,
                    s.thread_ts,
                    (s.reaction_count * 2 + s.reply_count + s.participant_count)::bigint AS score_period,
                    RANK() OVER (
                        ORDER BY
                            (s.reaction_count * 2 + s.reply_count + s.participant_count) DESC,
                            s.channel_id,
                            s.thread_ts
                    ) AS rank
                FROM stats s
                CROSS JOIN bounds b
                WHERE s.thread_ts ~ '^[0-9]+(\.[0-9]+)?$'
                  AND to_timestamp(s.thread_ts::double precision) >= b.period_start
                  AND to_timestamp(s.thread_ts::double precision) < b.period_end
            )
            SELECT
                cur.channel_id,
                COALESCE(ch.name, cur.channel_id)                         AS channel_name,
                cur.thread_ts,
                s.text,
                ''::text                                                AS search_text,
                s.created_at,
                COALESCE(u.display_name, s.user_id, '')                   AS display_name,
                COALESCE(u.avatar_url, '')                                 AS avatar_url,
                s.reaction_count,
                s.reply_count,
                s.participant_count,
                -1::bigint                                               AS file_count,
                cur.score_period::bigint                                   AS score,
                cur.score_period::bigint                                   AS rank_score,
                cur.rank::bigint                                           AS rank,
                NULL::bigint                                               AS prev_rank
            FROM cur
            JOIN stats s
                ON s.channel_id = cur.channel_id
               AND s.thread_ts = cur.thread_ts
            LEFT JOIN users u
                ON u.user_id = s.user_id
            LEFT JOIN channels ch
                ON ch.channel_id = cur.channel_id
            WHERE COALESCE(ch.name, cur.channel_id) != 'intro'
            ORDER BY cur.rank ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PeriodRankedThread {
                thread: row_to_thread_summary(&row),
                rank_score: row.get("rank_score"),
                rank: row.get("rank"),
                prev_rank: row.get("prev_rank"),
            })
            .collect())
    }

    async fn get_monthly_ranked_threads(&self, limit: i64) -> Result<Vec<PeriodRankedThread>> {
        if read_model_v2_enabled() {
            let rows = sqlx::query(
                r#"
                SELECT
                    tr.channel_id,
                    COALESCE(ch.name, tr.channel_id)              AS channel_name,
                    tr.thread_ts,
                    tr.root_text                                   AS text,
                    tr.search_text                                 AS search_text,
                    tr.root_created_at                             AS created_at,
                    COALESCE(u.display_name, tr.root_user_id, '') AS display_name,
                    COALESCE(u.avatar_url, '')                    AS avatar_url,
                    tr.reaction_count_total                        AS reaction_count,
                    tr.reply_count_total                           AS reply_count,
                    tr.participant_count_total                     AS participant_count,
                    tr.file_count_total                            AS file_count,
                    tps.score_period                               AS score,
                    tps.score_period                               AS rank_score,
                    RANK() OVER (
                        ORDER BY tps.score_period DESC, tps.channel_id, tps.thread_ts
                    )::bigint                                      AS rank,
                    NULL::bigint                                   AS prev_rank
                FROM thread_period_scores tps
                JOIN thread_rollups tr
                    ON tr.channel_id = tps.channel_id
                   AND tr.thread_ts = tps.thread_ts
                LEFT JOIN users u
                    ON u.user_id = tr.root_user_id
                LEFT JOIN channels ch
                    ON ch.channel_id = tr.channel_id
                WHERE tps.period_kind = 'month'
                  AND tps.period_start = date_trunc('month', CURRENT_DATE)::date
                  AND COALESCE(ch.name, tr.channel_id) != 'intro'
                ORDER BY rank ASC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(self)
            .await?;

            if !rows.is_empty() {
                return Ok(rows
                    .into_iter()
                    .map(|row| PeriodRankedThread {
                        thread: row_to_thread_summary(&row),
                        rank_score: row.get("rank_score"),
                        rank: row.get("rank"),
                        prev_rank: row.get("prev_rank"),
                    })
                    .collect());
            }
        }

        let rows = sqlx::query(
            r#"
            WITH stats AS (
                SELECT
                    m.channel_id,
                    m.ts                                                  AS thread_ts,
                    m.user_id,
                    m.text,
                    m.created_at,
                    COALESCE((
                        SELECT SUM((rr_elem->'count')::bigint)::bigint
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
            ),
            bounds AS (
                SELECT
                    date_trunc('month', CURRENT_DATE)::timestamp AS period_start,
                    (date_trunc('month', CURRENT_DATE)::timestamp + INTERVAL '1 month') AS period_end
            ),
            cur AS (
                SELECT
                    s.channel_id,
                    s.thread_ts,
                    (s.reaction_count * 2 + s.reply_count + s.participant_count)::bigint AS score_period,
                    RANK() OVER (
                        ORDER BY
                            (s.reaction_count * 2 + s.reply_count + s.participant_count) DESC,
                            s.channel_id,
                            s.thread_ts
                    ) AS rank
                FROM stats s
                CROSS JOIN bounds b
                WHERE s.thread_ts ~ '^[0-9]+(\.[0-9]+)?$'
                  AND to_timestamp(s.thread_ts::double precision) >= b.period_start
                  AND to_timestamp(s.thread_ts::double precision) < b.period_end
            )
            SELECT
                cur.channel_id,
                COALESCE(ch.name, cur.channel_id)                         AS channel_name,
                cur.thread_ts,
                s.text,
                ''::text                                                AS search_text,
                s.created_at,
                COALESCE(u.display_name, s.user_id, '')                   AS display_name,
                COALESCE(u.avatar_url, '')                                 AS avatar_url,
                s.reaction_count,
                s.reply_count,
                s.participant_count,
                -1::bigint                                               AS file_count,
                cur.score_period::bigint                                   AS score,
                cur.score_period::bigint                                   AS rank_score,
                cur.rank::bigint                                           AS rank,
                NULL::bigint                                               AS prev_rank
            FROM cur
            JOIN stats s
                ON s.channel_id = cur.channel_id
               AND s.thread_ts = cur.thread_ts
            LEFT JOIN users u
                ON u.user_id = s.user_id
            LEFT JOIN channels ch
                ON ch.channel_id = cur.channel_id
            WHERE COALESCE(ch.name, cur.channel_id) != 'intro'
            ORDER BY cur.rank ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PeriodRankedThread {
                thread: row_to_thread_summary(&row),
                rank_score: row.get("rank_score"),
                rank: row.get("rank"),
                prev_rank: row.get("prev_rank"),
            })
            .collect())
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
                m.text,
                COALESCE(u.display_name, m.user_id, '') AS display_name,
                COALESCE(u.avatar_url, '')               AS avatar_url,
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
            WHERE m.channel_id = $1
              AND (m.thread_ts = $2 OR m.ts = $2)
            ORDER BY m.ts ASC
            "#,
        )
        .bind(channel_id)
        .bind(thread_ts)
        .fetch_all(self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ThreadMessage {
                ts: r.get("ts"),
                text: r.get("text"),
                display_name: r.get("display_name"),
                avatar_url: r.get("avatar_url"),
                reactions: r.get("reactions"),
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
