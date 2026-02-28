use anyhow::Result;
use sqlx::PgPool;

use super::super::models::ThreadSummary;
use super::{read_model_v2_enabled, row_to_thread_summary};

pub(super) async fn get_recent_threads(pool: &PgPool, limit: i64) -> Result<Vec<ThreadSummary>> {
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
        .fetch_all(pool)
        .await?;

        return rows
            .into_iter()
            .map(|row| row_to_thread_summary(&row))
            .collect::<Result<Vec<_>>>();
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
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| row_to_thread_summary(&row))
        .collect::<Result<Vec<_>>>()
}
