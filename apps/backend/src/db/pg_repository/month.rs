use anyhow::Result;
use sqlx::{PgPool, Row};

use super::super::models::PeriodRankedThread;
use super::{read_model_v2_enabled, row_to_thread_summary};

pub(super) async fn get_monthly_ranked_threads(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<PeriodRankedThread>> {
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
            LEFT JOIN auth_accounts aa
                ON aa.slack_user_id = u.user_id
               AND aa.disabled_at IS NULL
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
        .fetch_all(pool)
        .await?;

        return rows
            .into_iter()
            .map(|row| -> Result<PeriodRankedThread> {
                Ok(PeriodRankedThread {
                    thread: row_to_thread_summary(&row)?,
                    rank_score: row.try_get("rank_score")?,
                    rank: row.try_get("rank")?,
                    prev_rank: row.try_get("prev_rank")?,
                })
            })
            .collect::<Result<Vec<_>>>();
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
            COALESCE(s.user_id, '')                                   AS user_id,
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
        LEFT JOIN auth_accounts aa
            ON aa.slack_user_id = u.user_id
           AND aa.disabled_at IS NULL
        LEFT JOIN channels ch
            ON ch.channel_id = cur.channel_id
        WHERE COALESCE(ch.name, cur.channel_id) != 'intro'
        ORDER BY cur.rank ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| -> Result<PeriodRankedThread> {
            Ok(PeriodRankedThread {
                thread: row_to_thread_summary(&row)?,
                rank_score: row.try_get("rank_score")?,
                rank: row.try_get("rank")?,
                prev_rank: row.try_get("prev_rank")?,
            })
        })
        .collect::<Result<Vec<_>>>()
}
