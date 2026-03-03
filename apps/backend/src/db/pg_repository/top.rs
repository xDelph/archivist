use anyhow::Result;
use sqlx::{PgPool, Row};

use super::super::models::{ThreadSummary, ThreadWithWeeklyScore};
use super::{read_model_v2_enabled, row_to_thread_summary};

pub(super) async fn get_top_threads(pool: &PgPool, limit: i64) -> Result<Vec<ThreadSummary>> {
    if read_model_v2_enabled() {
        let rollup_rows = sqlx::query(
            r#"
            SELECT
                tr.channel_id,
                COALESCE(ch.name, tr.channel_id)         AS channel_name,
                tr.thread_ts,
                COALESCE(tr.root_user_id, '')              AS user_id,
                tr.root_text                              AS text,
                tr.search_text                            AS search_text,
                tr.root_created_at                        AS created_at,
                CASE
                    WHEN COALESCE(u.is_active, TRUE) = FALSE
                      OR COALESCE(u.is_deleted, FALSE) = TRUE
                      OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                    THEN 'Anonymous'
                    ELSE COALESCE(u.display_name, tr.root_user_id, '')
                END                                       AS display_name,
                CASE
                    WHEN COALESCE(u.is_active, TRUE) = FALSE
                      OR COALESCE(u.is_deleted, FALSE) = TRUE
                      OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                    THEN '/placeholder-user.jpg'
                    ELSE COALESCE(u.avatar_url, '')
                END                                       AS avatar_url,
                tr.reaction_count_total                   AS reaction_count,
                tr.reply_count_total                      AS reply_count,
                tr.participant_count_total                AS participant_count,
                tr.file_count_total                       AS file_count,
                tr.score_total                            AS score
            FROM thread_rollups tr
            LEFT JOIN users u
                ON u.user_id = tr.root_user_id
            LEFT JOIN auth_accounts aa
                ON aa.slack_user_id = u.user_id
               AND aa.disabled_at IS NULL
            LEFT JOIN channels ch
                ON ch.channel_id = tr.channel_id
            WHERE COALESCE(ch.name, tr.channel_id) != 'intro'
            ORDER BY tr.score_total DESC, tr.channel_id, tr.thread_ts
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        if !rollup_rows.is_empty() {
            return rollup_rows
                .into_iter()
                .map(|row| row_to_thread_summary(&row))
                .collect::<Result<Vec<_>>>();
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
            COALESCE(ch.name, s.channel_id)         AS channel_name,
            s.thread_ts,
            COALESCE(s.user_id, '')                  AS user_id,
            s.text,
            s.created_at,
            CASE
                WHEN COALESCE(u.is_active, TRUE) = FALSE
                  OR COALESCE(u.is_deleted, FALSE) = TRUE
                  OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                THEN 'Anonymous'
                ELSE COALESCE(u.display_name, s.user_id, '')
            END                                     AS display_name,
            CASE
                WHEN COALESCE(u.is_active, TRUE) = FALSE
                  OR COALESCE(u.is_deleted, FALSE) = TRUE
                  OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                THEN '/placeholder-user.jpg'
                ELSE COALESCE(u.avatar_url, '')
            END                                     AS avatar_url,
            ''::text                                 AS search_text,
            s.reaction_count                        AS reaction_count,
            s.reply_count                           AS reply_count,
            s.participant_count                     AS participant_count,
            -1::bigint                               AS file_count,
            (s.reaction_count * 2
                + s.reply_count
                + s.participant_count)              AS score
        FROM stats s
        LEFT JOIN users    u  ON u.user_id    = s.user_id
        LEFT JOIN auth_accounts aa
            ON aa.slack_user_id = u.user_id
           AND aa.disabled_at IS NULL
        LEFT JOIN channels ch ON ch.channel_id = s.channel_id
        WHERE COALESCE(ch.name, s.channel_id) != 'intro'
        ORDER BY s.reaction_count * 2 + s.reply_count + s.participant_count DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| row_to_thread_summary(&row))
        .collect::<Result<Vec<_>>>()
}

pub(super) async fn get_top_threads_with_weekly(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ThreadWithWeeklyScore>> {
    if read_model_v2_enabled() {
        let rows = sqlx::query(
            r#"
            SELECT
                tr.channel_id,
                COALESCE(ch.name, tr.channel_id)              AS channel_name,
                tr.thread_ts,
                COALESCE(tr.root_user_id, '')                   AS user_id,
                tr.root_text                                   AS text,
                tr.search_text                                 AS search_text,
                tr.root_created_at                             AS created_at,
                CASE
                    WHEN COALESCE(u.is_active, TRUE) = FALSE
                      OR COALESCE(u.is_deleted, FALSE) = TRUE
                      OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                    THEN 'Anonymous'
                    ELSE COALESCE(u.display_name, tr.root_user_id, '')
                END                                            AS display_name,
                CASE
                    WHEN COALESCE(u.is_active, TRUE) = FALSE
                      OR COALESCE(u.is_deleted, FALSE) = TRUE
                      OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                    THEN '/placeholder-user.jpg'
                    ELSE COALESCE(u.avatar_url, '')
                END                                            AS avatar_url,
                tr.reaction_count_total                        AS reaction_count,
                tr.reply_count_total                           AS reply_count,
                tr.participant_count_total                     AS participant_count,
                tr.file_count_total                            AS file_count,
                tr.score_total                                 AS score,
                COALESCE(tps.score_period, 0)::bigint         AS score_week
            FROM thread_rollups tr
            LEFT JOIN users u
                ON u.user_id = tr.root_user_id
            LEFT JOIN auth_accounts aa
                ON aa.slack_user_id = u.user_id
               AND aa.disabled_at IS NULL
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
        .fetch_all(pool)
        .await?;

        if !rows.is_empty() {
            return rows
                .into_iter()
                .map(|row| -> Result<ThreadWithWeeklyScore> {
                    Ok(ThreadWithWeeklyScore {
                        thread: row_to_thread_summary(&row)?,
                        score_week: row.try_get("score_week")?,
                    })
                })
                .collect::<Result<Vec<_>>>();
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
            COALESCE(s.user_id, '')                                    AS user_id,
            s.text,
            ''::text                                                AS search_text,
            s.created_at,
            CASE
                WHEN COALESCE(u.is_active, TRUE) = FALSE
                  OR COALESCE(u.is_deleted, FALSE) = TRUE
                  OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                THEN 'Anonymous'
                ELSE COALESCE(u.display_name, s.user_id, '')
            END                                                        AS display_name,
            CASE
                WHEN COALESCE(u.is_active, TRUE) = FALSE
                  OR COALESCE(u.is_deleted, FALSE) = TRUE
                  OR COALESCE(aa.is_anonymous, FALSE) = TRUE
                THEN '/placeholder-user.jpg'
                ELSE COALESCE(u.avatar_url, '')
            END                                                        AS avatar_url,
            s.reaction_count,
            s.reply_count,
            s.participant_count,
            -1::bigint                                              AS file_count,
            (s.reaction_count * 2 + s.reply_count + s.participant_count) AS score,
            COALESCE(tws.score_week, 0)::bigint                        AS score_week
        FROM stats s
        LEFT JOIN users u
            ON u.user_id = s.user_id
        LEFT JOIN auth_accounts aa
            ON aa.slack_user_id = u.user_id
           AND aa.disabled_at IS NULL
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
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| -> Result<ThreadWithWeeklyScore> {
            Ok(ThreadWithWeeklyScore {
                thread: row_to_thread_summary(&row)?,
                score_week: row.try_get("score_week")?,
            })
        })
        .collect::<Result<Vec<_>>>()
}
