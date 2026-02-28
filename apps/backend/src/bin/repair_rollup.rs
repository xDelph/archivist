use std::env;
use std::time::Duration;

use archivist::api::aggregation_jobs::run_aggregation_batch;
use archivist::db::{Repository, pool::create_pool};
use archivist::logging::init_tracing;
use sqlx::Row;
use tracing::{info, warn};

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
    let channel_id = arg_value(&args, "--channel")
        .or_else(|| arg_value(&args, "-c"))
        .ok_or_else(|| anyhow::anyhow!("missing --channel <CHANNEL_ID>"))?;
    let thread_ts = arg_value(&args, "--ts")
        .or_else(|| arg_value(&args, "-t"))
        .ok_or_else(|| anyhow::anyhow!("missing --ts <THREAD_TS>"))?;
    let batch_size = env::var("ROLLUP_REPAIR_BATCH_SIZE")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(100)
        .max(1);
    let max_rounds = env::var("ROLLUP_REPAIR_MAX_ROUNDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(20)
        .max(1);

    let db_url = env::var("DATABASE_URL_UNPOOLED")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL or DATABASE_URL_UNPOOLED must be set");
    let pool = create_pool(&db_url).await?;

    pool.enqueue_thread_aggregation(&channel_id, &thread_ts, "repair_rollup")
        .await?;
    let dedupe_key = format!("thread_rollup:{channel_id}:{thread_ts}");
    info!(%channel_id, %thread_ts, "repair job queued");

    for round in 1..=max_rounds {
        let batch = run_aggregation_batch(&pool, batch_size).await?;
        let status = sqlx::query_scalar::<_, Option<String>>(
            "SELECT status FROM aggregation_jobs WHERE dedupe_key = $1 LIMIT 1",
        )
        .bind(&dedupe_key)
        .fetch_one(&pool)
        .await?
        .unwrap_or_else(|| "missing".to_owned());

        info!(
            round,
            claimed = batch.claimed,
            succeeded = batch.succeeded,
            requeued = batch.requeued,
            failed = batch.failed,
            status = %status,
            "repair progress"
        );

        match status.as_str() {
            "succeeded" => {
                let rollup_row = sqlx::query(
                    r#"
                    SELECT
                        reaction_count_total,
                        reply_count_total,
                        participant_count_total,
                        file_count_total,
                        score_total
                    FROM thread_rollups
                    WHERE channel_id = $1
                      AND thread_ts = $2
                    "#,
                )
                .bind(&channel_id)
                .bind(&thread_ts)
                .fetch_optional(&pool)
                .await?;

                if let Some(row) = rollup_row {
                    info!(
                        reaction_count_total = row.get::<i64, _>("reaction_count_total"),
                        reply_count_total = row.get::<i64, _>("reply_count_total"),
                        participant_count_total = row.get::<i64, _>("participant_count_total"),
                        file_count_total = row.get::<i64, _>("file_count_total"),
                        score_total = row.get::<i64, _>("score_total"),
                        "repair completed"
                    );
                } else {
                    warn!("job succeeded but no thread_rollups row found");
                }
                return Ok(());
            }
            "failed" => anyhow::bail!("repair job reached permanent failure status"),
            _ => {}
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    anyhow::bail!("repair job did not reach terminal success within max rounds")
}
