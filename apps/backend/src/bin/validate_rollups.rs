use std::env;

use archivist::db::pool::create_pool;
use archivist::logging::init_tracing;
use sqlx::Row;
use tracing::{error, info, warn};

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.windows(2).find_map(|window| {
        if window[0] == key {
            Some(window[1].clone())
        } else {
            None
        }
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let args: Vec<String> = env::args().collect();
    let strict = args.iter().any(|arg| arg == "--strict");
    let sample_limit = arg_value(&args, "--sample")
        .and_then(|value| value.parse::<i64>().ok())
        .or_else(|| env::var("ROLLUP_VALIDATE_SAMPLE").ok()?.parse::<i64>().ok())
        .unwrap_or(300)
        .max(1);

    let db_url = env::var("DATABASE_URL_UNPOOLED")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL or DATABASE_URL_UNPOOLED must be set");
    let pool = create_pool(&db_url).await?;

    let checked_row = sqlx::query(
        r#"
        WITH roots AS (
            SELECT DISTINCT
                m.channel_id,
                CASE
                    WHEN m.thread_ts IS NULL OR m.thread_ts = '' THEN m.ts
                    ELSE m.thread_ts
                END AS thread_ts
            FROM messages m
            ORDER BY m.channel_id, thread_ts
            LIMIT $1
        )
        SELECT COUNT(*)::bigint AS checked_count
        FROM roots
        "#,
    )
    .bind(sample_limit)
    .fetch_one(&pool)
    .await?;
    let checked_count: i64 = checked_row.get("checked_count");

    let mismatch_rows = sqlx::query(
        r#"
        WITH roots AS (
            SELECT DISTINCT
                m.channel_id,
                CASE
                    WHEN m.thread_ts IS NULL OR m.thread_ts = '' THEN m.ts
                    ELSE m.thread_ts
                END AS thread_ts
            FROM messages m
            ORDER BY m.channel_id, thread_ts
            LIMIT $1
        ),
        thread_messages AS (
            SELECT
                r.channel_id,
                r.thread_ts,
                m.ts,
                m.user_id,
                m.raw_json
            FROM roots r
            JOIN messages m
              ON m.channel_id = r.channel_id
             AND (m.ts = r.thread_ts OR m.thread_ts = r.thread_ts)
        ),
        truth AS (
            SELECT
                r.channel_id,
                r.thread_ts,
                COALESCE((
                    SELECT SUM((rr_elem->'count')::bigint)::bigint
                    FROM thread_messages tm
                    CROSS JOIN LATERAL jsonb_array_elements(
                        CASE
                            WHEN jsonb_typeof(tm.raw_json->'reactions') = 'array'
                            THEN tm.raw_json->'reactions'
                            ELSE '[]'::jsonb
                        END
                    ) rr_elem
                    WHERE tm.channel_id = r.channel_id
                      AND tm.thread_ts = r.thread_ts
                ), 0) AS reaction_count_total,
                COALESCE((
                    SELECT COUNT(*)::bigint
                    FROM thread_messages tm
                    WHERE tm.channel_id = r.channel_id
                      AND tm.thread_ts = r.thread_ts
                      AND tm.ts <> r.thread_ts
                ), 0) AS reply_count_total,
                (
                    COALESCE((
                        SELECT COUNT(DISTINCT tm.user_id)::bigint
                        FROM thread_messages tm
                        WHERE tm.channel_id = r.channel_id
                          AND tm.thread_ts = r.thread_ts
                          AND tm.ts <> r.thread_ts
                    ), 0) + 1
                ) AS participant_count_total,
                COALESCE((
                    SELECT COUNT(*)::bigint
                    FROM files f
                    WHERE f.channel_id = r.channel_id
                      AND f.message_ts = r.thread_ts
                ), 0) AS file_count_total
            FROM roots r
        ),
        compared AS (
            SELECT
                t.channel_id,
                t.thread_ts,
                t.reaction_count_total AS expected_reaction_count,
                t.reply_count_total AS expected_reply_count,
                t.participant_count_total AS expected_participant_count,
                t.file_count_total AS expected_file_count,
                (t.reaction_count_total * 2 + t.reply_count_total + t.participant_count_total)::bigint AS expected_score,
                tr.reaction_count_total AS actual_reaction_count,
                tr.reply_count_total AS actual_reply_count,
                tr.participant_count_total AS actual_participant_count,
                tr.file_count_total AS actual_file_count,
                tr.score_total AS actual_score
            FROM truth t
            LEFT JOIN thread_rollups tr
              ON tr.channel_id = t.channel_id
             AND tr.thread_ts = t.thread_ts
        )
        SELECT *
        FROM compared
        WHERE actual_reaction_count IS NULL
           OR actual_reaction_count <> expected_reaction_count
           OR actual_reply_count <> expected_reply_count
           OR actual_participant_count <> expected_participant_count
           OR actual_file_count <> expected_file_count
           OR actual_score <> expected_score
        ORDER BY channel_id, thread_ts
        LIMIT 50
        "#,
    )
    .bind(sample_limit)
    .fetch_all(&pool)
    .await?;

    let mismatch_count = mismatch_rows.len();
    info!(
        checked_count,
        mismatch_count, sample_limit, "rollup validation complete"
    );

    if mismatch_count == 0 {
        info!("no mismatches found");
        return Ok(());
    }

    for row in mismatch_rows {
        warn!(
            channel_id = %row.get::<String, _>("channel_id"),
            thread_ts = %row.get::<String, _>("thread_ts"),
            expected_reaction_count = row.get::<i64, _>("expected_reaction_count"),
            actual_reaction_count = row.try_get::<i64, _>("actual_reaction_count").ok(),
            expected_reply_count = row.get::<i64, _>("expected_reply_count"),
            actual_reply_count = row.try_get::<i64, _>("actual_reply_count").ok(),
            expected_participant_count = row.get::<i64, _>("expected_participant_count"),
            actual_participant_count = row.try_get::<i64, _>("actual_participant_count").ok(),
            expected_file_count = row.get::<i64, _>("expected_file_count"),
            actual_file_count = row.try_get::<i64, _>("actual_file_count").ok(),
            expected_score = row.get::<i64, _>("expected_score"),
            actual_score = row.try_get::<i64, _>("actual_score").ok(),
            "rollup mismatch"
        );
    }

    if strict {
        error!("strict mode enabled: mismatches detected");
        anyhow::bail!("rollup validation failed");
    }
    Ok(())
}
