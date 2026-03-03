use std::env;
use std::time::Instant;

use archivist::api::aggregation_jobs::run_aggregation_batch;
use archivist::db::pool::create_pool;
use archivist::logging::init_tracing;
use sqlx::Row;
use tracing::info;

fn arg_present(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let args: Vec<String> = env::args().collect();
    let enqueue_only = arg_present(&args, "--enqueue-only");
    let drain_only = arg_present(&args, "--drain-only");
    let no_reset = arg_present(&args, "--no-reset");
    let batch_size = env::var("ROLLUP_BATCH_SIZE")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(200)
        .max(1);

    let db_url = env::var("DATABASE_URL_UNPOOLED")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL or DATABASE_URL_UNPOOLED must be set");
    let pool = create_pool(&db_url).await?;

    if !drain_only && !no_reset {
        info!("resetting read-model tables before rebuild");
        sqlx::query(
            r#"
            TRUNCATE TABLE
                thread_period_scores,
                thread_rollups,
                channel_daily_rollups,
                workspace_overview_rollups
            "#,
        )
        .execute(&pool)
        .await?;
    }

    if !drain_only {
        info!("resetting aggregation queue state for rebuild");
        sqlx::query("DELETE FROM aggregation_jobs WHERE job_kind = 'thread_rollup'")
            .execute(&pool)
            .await?;

        let enqueue_row = sqlx::query(
            r#"
            WITH roots AS (
                SELECT DISTINCT
                    m.channel_id,
                    CASE
                        WHEN m.thread_ts IS NULL OR m.thread_ts = '' THEN m.ts
                        ELSE m.thread_ts
                    END AS thread_ts
                FROM messages m
            ),
            inserted AS (
                INSERT INTO aggregation_jobs (
                    dedupe_key,
                    job_kind,
                    channel_id,
                    thread_ts,
                    status,
                    requested_by,
                    available_at,
                    created_at,
                    updated_at
                )
                SELECT
                    ('thread_rollup:' || roots.channel_id || ':' || roots.thread_ts),
                    'thread_rollup',
                    roots.channel_id,
                    roots.thread_ts,
                    'queued',
                    'rebuild_rollups',
                    NOW(),
                    NOW(),
                    NOW()
                FROM roots
                ON CONFLICT (dedupe_key)
                DO UPDATE SET
                    status = 'queued',
                    requested_by = EXCLUDED.requested_by,
                    available_at = NOW(),
                    finished_at = NULL,
                    last_error = NULL,
                    updated_at = NOW()
                RETURNING 1
            )
            SELECT COUNT(*)::bigint AS queued_count FROM inserted
            "#,
        )
        .fetch_one(&pool)
        .await?;
        let queued_count: i64 = enqueue_row.get("queued_count");
        info!(queued_count, "thread rollup jobs queued");
    } else {
        info!("drain-only mode: processing existing aggregation queue without reset");
    }

    if enqueue_only {
        info!("enqueue-only mode enabled, stopping after queue population");
        return Ok(());
    }

    let mut total_claimed = 0usize;
    let mut total_succeeded = 0usize;
    let mut total_requeued = 0usize;
    let mut total_failed = 0usize;
    let rebuild_started_at = Instant::now();
    let mut batch_index = 0usize;
    loop {
        batch_index += 1;
        let batch_started_at = Instant::now();
        info!(batch_index, batch_size, "starting rollup batch");
        let batch = run_aggregation_batch(&pool, batch_size).await?;
        let batch_elapsed_ms = batch_started_at.elapsed().as_millis();
        if batch.claimed == 0 {
            info!(
                batch_index,
                batch_elapsed_ms,
                total_elapsed_ms = rebuild_started_at.elapsed().as_millis(),
                "rollup batch returned empty queue"
            );
            break;
        }
        total_claimed += batch.claimed;
        total_succeeded += batch.succeeded;
        total_requeued += batch.requeued;
        total_failed += batch.failed;
        info!(
            claimed = batch.claimed,
            succeeded = batch.succeeded,
            requeued = batch.requeued,
            failed = batch.failed,
            batch_index,
            batch_elapsed_ms,
            total_elapsed_ms = rebuild_started_at.elapsed().as_millis(),
            "processed rollup batch"
        );
    }

    info!(
        total_claimed,
        total_succeeded,
        total_requeued,
        total_failed,
        batch_count = batch_index.saturating_sub(1),
        total_elapsed_ms = rebuild_started_at.elapsed().as_millis(),
        "rebuild rollups complete"
    );
    Ok(())
}
