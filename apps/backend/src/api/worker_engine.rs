use anyhow::Result;
use serde::Serialize;
use sqlx::{PgPool, Postgres, Row, pool::PoolConnection};
use tracing::{info, warn};

use crate::api::aggregation_jobs::run_aggregation_batch;
use crate::api::file_backfill_jobs::{FileBackfillBatchResult, run_file_backfill_batch};
use crate::slack::backfill::{SlackClient, run_backfill};
use crate::storage::R2Client;

const DEFAULT_WORKER_LOCK_KEY: i64 = 1_048_729;
const DEFAULT_AGGREGATION_JOBS_PER_BATCH: i64 = 200;
const DEFAULT_AGGREGATION_MAX_BATCHES_PER_RUN: usize = 200;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillOutcome {
    pub ran: bool,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationOutcome {
    pub batches: usize,
    pub claimed: usize,
    pub succeeded: usize,
    pub requeued: usize,
    pub failed: usize,
}

pub async fn execute_messages_phase(
    pool: &PgPool,
    slack_token: &str,
    storage: Option<&R2Client>,
) -> Result<BackfillOutcome> {
    let mut backfill = BackfillOutcome {
        ran: false,
        status: "idle".to_owned(),
        error: None,
    };

    if slack_token.is_empty() {
        let error = "missing SLACK_USER_TOKEN/SLACK_BOT_TOKEN";
        backfill.status = "failed".to_owned();
        backfill.error = Some(error.to_owned());
        warn!(error, "backfill messages phase skipped");
        return Ok(backfill);
    }

    backfill.ran = true;
    backfill.status = "running".to_owned();
    let client = SlackClient::new(slack_token.to_owned());
    match run_backfill(pool, &client, slack_token, storage).await {
        Ok(()) => {
            backfill.status = "succeeded".to_owned();
            info!("backfill messages phase completed");
        }
        Err(err) => {
            let err_string = err.to_string();
            backfill.status = "failed".to_owned();
            backfill.error = Some(err_string.clone());
            warn!(error = %err_string, "backfill messages phase failed");
        }
    }
    Ok(backfill)
}

pub async fn execute_files_phase(
    pool: &PgPool,
    slack_token: &str,
    storage: Option<&R2Client>,
) -> Result<FileBackfillBatchResult> {
    if slack_token.is_empty() {
        warn!("files phase skipped: missing SLACK_USER_TOKEN/SLACK_BOT_TOKEN");
        return Ok(FileBackfillBatchResult::default());
    }
    run_file_backfill_batch(pool, slack_token, storage).await
}

pub async fn execute_aggregation_phase(pool: &PgPool) -> Result<AggregationOutcome> {
    drain_aggregation_jobs(pool).await
}

async fn drain_aggregation_jobs(pool: &PgPool) -> Result<AggregationOutcome> {
    let mut out = AggregationOutcome::default();
    for _ in 0..aggregation_max_batches_per_run() {
        let batch = run_aggregation_batch(pool, aggregation_jobs_per_batch()).await?;
        if batch.claimed == 0 {
            break;
        }
        out.batches += 1;
        out.claimed += batch.claimed;
        out.succeeded += batch.succeeded;
        out.requeued += batch.requeued;
        out.failed += batch.failed;
    }
    Ok(out)
}

pub async fn try_acquire_worker_lock(pool: &PgPool) -> Result<Option<PoolConnection<Postgres>>> {
    let mut conn = pool.acquire().await?;
    let row = sqlx::query("SELECT pg_try_advisory_lock($1) AS acquired")
        .bind(worker_lock_key())
        .fetch_one(&mut *conn)
        .await?;
    let acquired: bool = row.get("acquired");
    if acquired { Ok(Some(conn)) } else { Ok(None) }
}

pub async fn release_worker_lock(conn: &mut PoolConnection<Postgres>) -> Result<()> {
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(worker_lock_key())
        .execute(&mut **conn)
        .await?;
    Ok(())
}

fn worker_lock_key() -> i64 {
    std::env::var("BACKFILL_WORKER_LOCK_KEY")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_WORKER_LOCK_KEY)
}

fn aggregation_jobs_per_batch() -> i64 {
    std::env::var("AGGREGATION_JOBS_PER_BATCH")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_AGGREGATION_JOBS_PER_BATCH)
}

fn aggregation_max_batches_per_run() -> usize {
    std::env::var("AGGREGATION_MAX_BATCHES_PER_RUN")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_AGGREGATION_MAX_BATCHES_PER_RUN)
}
