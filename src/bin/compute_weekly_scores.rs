/// One-shot script: compute and seed historical weekly scores for every thread.
///
/// Safe to re-run: inserts are idempotent (`ON CONFLICT DO UPDATE`).
/// Uses `DATABASE_URL_UNPOOLED` to avoid Neon pooler prepared-statement issues.
use std::env;

use archivist::db::pool::create_pool;
use sqlx::Row;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let db_url = env::var("DATABASE_URL_UNPOOLED").expect("DATABASE_URL_UNPOOLED not set");
    let pool = create_pool(&db_url).await?;

    let channels = sqlx::query(
        r#"
        SELECT DISTINCT channel_id
        FROM messages
        ORDER BY channel_id
        "#,
    )
    .fetch_all(&pool)
    .await?;

    info!(
        channels = channels.len(),
        "starting weekly score computation"
    );

    for channel in channels {
        let channel_id: String = channel.try_get("channel_id")?;
        let thread_rows = sqlx::query(
            r#"
            SELECT DISTINCT
                CASE
                    WHEN m.thread_ts IS NULL OR m.thread_ts = '' THEN m.ts
                    ELSE m.thread_ts
                END AS thread_ts
            FROM messages m
            WHERE m.channel_id = $1
            ORDER BY thread_ts
            "#,
        )
        .bind(&channel_id)
        .fetch_all(&pool)
        .await?;

        info!(
            channel_id = %channel_id,
            threads = thread_rows.len(),
            "computing weekly scores for channel",
        );

        for (idx, row) in thread_rows.iter().enumerate() {
            let thread_ts: String = row.try_get("thread_ts")?;

            sqlx::query(
                r#"
                WITH thread_messages AS (
                    SELECT m.ts, m.thread_ts, m.user_id
                    FROM messages m
                    WHERE m.channel_id = $1
                      AND (m.ts = $2 OR m.thread_ts = $2)
                ),
                weeks AS (
                    SELECT DISTINCT date_trunc('week', to_timestamp(ev_ts::double precision))::date AS week_start
                    FROM (
                        SELECT tm.ts AS ev_ts
                        FROM thread_messages tm
                        WHERE tm.thread_ts = $2
                          AND tm.ts <> $2
                          AND tm.ts ~ '^[0-9]+(\.[0-9]+)?$'
                        UNION ALL
                        SELECT r.event_ts AS ev_ts
                        FROM reactions r
                        JOIN thread_messages tm ON tm.ts = r.message_ts
                        WHERE r.channel_id = $1
                          AND r.event_ts ~ '^[0-9]+(\.[0-9]+)?$'
                    ) events
                ),
                scored AS (
                    SELECT
                        w.week_start,
                        $1::text AS channel_id,
                        $2::text AS thread_ts,
                        (
                            COALESCE((
                                SELECT COUNT(*)::bigint
                                FROM reactions r
                                JOIN thread_messages tm ON tm.ts = r.message_ts
                                WHERE r.channel_id = $1
                                  AND r.event_ts ~ '^[0-9]+(\.[0-9]+)?$'
                                  AND to_timestamp(r.event_ts::double precision) >= w.week_start::timestamp
                                  AND to_timestamp(r.event_ts::double precision) < (w.week_start::timestamp + INTERVAL '7 days')
                            ), 0) * 2
                            + COALESCE((
                                SELECT COUNT(*)::bigint
                                FROM thread_messages tm
                                WHERE tm.thread_ts = $2
                                  AND tm.ts <> $2
                                  AND tm.ts ~ '^[0-9]+(\.[0-9]+)?$'
                                  AND to_timestamp(tm.ts::double precision) >= w.week_start::timestamp
                                  AND to_timestamp(tm.ts::double precision) < (w.week_start::timestamp + INTERVAL '7 days')
                            ), 0)
                        )::int AS score_week,
                        (
                            COALESCE((
                                SELECT COUNT(*)::bigint
                                FROM reactions r
                                JOIN thread_messages tm ON tm.ts = r.message_ts
                                WHERE r.channel_id = $1
                                  AND r.event_ts ~ '^[0-9]+(\.[0-9]+)?$'
                                  AND to_timestamp(r.event_ts::double precision) < (w.week_start::timestamp + INTERVAL '7 days')
                            ), 0) * 2
                            + COALESCE((
                                SELECT COUNT(*)::bigint
                                FROM thread_messages tm
                                WHERE tm.thread_ts = $2
                                  AND tm.ts <> $2
                                  AND tm.ts ~ '^[0-9]+(\.[0-9]+)?$'
                                  AND to_timestamp(tm.ts::double precision) < (w.week_start::timestamp + INTERVAL '7 days')
                            ), 0)
                            + COALESCE((
                                SELECT COUNT(DISTINCT tm.user_id)::bigint + 1
                                FROM thread_messages tm
                                WHERE tm.thread_ts = $2
                                  AND tm.ts <> $2
                                  AND tm.ts ~ '^[0-9]+(\.[0-9]+)?$'
                                  AND to_timestamp(tm.ts::double precision) < (w.week_start::timestamp + INTERVAL '7 days')
                            ), 1)
                        )::int AS score_total
                    FROM weeks w
                )
                INSERT INTO thread_weekly_scores
                    (week_start, channel_id, thread_ts, score_week, score_total, updated_at)
                SELECT
                    s.week_start,
                    s.channel_id,
                    s.thread_ts,
                    s.score_week,
                    s.score_total,
                    NOW()
                FROM scored s
                ON CONFLICT (week_start, channel_id, thread_ts)
                DO UPDATE SET
                    score_week = EXCLUDED.score_week,
                    score_total = EXCLUDED.score_total,
                    updated_at = NOW()
                "#,
            )
            .bind(&channel_id)
            .bind(&thread_ts)
            .execute(&pool)
            .await?;

            if (idx + 1) % 100 == 0 {
                info!(
                    channel_id = %channel_id,
                    done = idx + 1,
                    total = thread_rows.len(),
                    "weekly score progress",
                );
            }
        }
    }

    info!("weekly score computation complete");
    Ok(())
}
