use std::env;

use archivist::api::aggregation_jobs::run_aggregation_batch;
use archivist::db::pool::create_pool;
use sqlx::Row;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn arg_present(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = env::args().collect();
    let enqueue_only = arg_present(&args, "--enqueue-only");
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

    if !no_reset {
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

    if enqueue_only {
        info!("enqueue-only mode enabled, stopping after queue population");
        return Ok(());
    }

    let mut total_claimed = 0usize;
    let mut total_succeeded = 0usize;
    let mut total_requeued = 0usize;
    let mut total_failed = 0usize;
    loop {
        let batch = run_aggregation_batch(&pool, batch_size).await?;
        if batch.claimed == 0 {
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
            "processed rollup batch"
        );
    }

    info!(
        total_claimed,
        total_succeeded, total_requeued, total_failed, "rebuild rollups complete"
    );
    Ok(())
}
