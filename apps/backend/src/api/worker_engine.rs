use std::time::Duration;

use anyhow::Result;
use serde::Serialize;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::api::aggregation_jobs::{AggregationBatchResult, run_aggregation_batch};
use crate::api::backfill_jobs::{
    claim_next_backfill_job, mark_backfill_job_failed, mark_backfill_job_succeeded,
};
use crate::slack::backfill::{
    BackfillSliceConfig, BackfillSliceResult, SlackClient, run_backfill_slice,
};
use crate::storage::R2Client;

const DEFAULT_BACKFILL_SLICE_TIMEOUT_SECONDS: u64 = 8;
const DEFAULT_BACKFILL_CHANNELS_PER_SLICE: usize = 3;
const DEFAULT_USERS_PAGES_PER_SLICE: usize = 2;
const DEFAULT_USERS_SYNC_INTERVAL_MINUTES: i64 = 720;
const DEFAULT_AGGREGATION_JOBS_PER_SLICE: i64 = 25;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSliceOutcome {
    pub backfill: BackfillOutcome,
    pub aggregation: AggregationBatchResult,
    pub ran: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillOutcome {
    pub ran: bool,
    pub job_id: Option<String>,
    pub status: String,
    pub slice: Option<BackfillSliceResult>,
    pub error: Option<String>,
}

pub async fn execute_worker_slice(
    pool: &PgPool,
    slack_token: &str,
    storage: Option<&R2Client>,
) -> Result<WorkerSliceOutcome> {
    let mut backfill = BackfillOutcome {
        ran: false,
        job_id: None,
        status: "idle".to_owned(),
        slice: None,
        error: None,
    };

    if let Some(job_id) = claim_next_backfill_job(pool).await? {
        backfill.ran = true;
        backfill.job_id = Some(job_id.clone());
        backfill.status = "running".to_owned();
        info!(job_id = %job_id, "backfill worker claimed job");

        if slack_token.is_empty() {
            let error = "missing SLACK_USER_TOKEN/SLACK_BOT_TOKEN";
            mark_backfill_job_failed(pool, &job_id, error).await?;
            backfill.status = "failed".to_owned();
            backfill.error = Some(error.to_owned());
            warn!(job_id = %job_id, error, "backfill worker failed job");
        } else {
            let client = SlackClient::new(slack_token.to_owned());
            let config = BackfillSliceConfig {
                max_channels: backfill_channels_per_slice(),
                users_pages_per_slice: users_pages_per_slice(),
                users_sync_interval_minutes: users_sync_interval_minutes(),
            };
            let slice_run = tokio::time::timeout(
                Duration::from_secs(backfill_slice_timeout_seconds()),
                run_backfill_slice(pool, &client, slack_token, storage, &config),
            )
            .await;

            match slice_run {
                Ok(Ok(slice)) => {
                    mark_backfill_job_succeeded(pool, &job_id).await?;
                    backfill.status = "succeeded".to_owned();
                    backfill.slice = Some(slice);
                    info!(job_id = %job_id, "backfill worker completed job");
                }
                Ok(Err(err)) => {
                    let err_string = err.to_string();
                    mark_backfill_job_failed(pool, &job_id, &err_string).await?;
                    backfill.status = "failed".to_owned();
                    backfill.error = Some(err_string.clone());
                    warn!(job_id = %job_id, error = %err_string, "backfill worker failed job");
                }
                Err(_) => {
                    let err_string = format!(
                        "backfill slice timed out after {} seconds",
                        backfill_slice_timeout_seconds()
                    );
                    mark_backfill_job_failed(pool, &job_id, &err_string).await?;
                    backfill.status = "timed_out".to_owned();
                    backfill.error = Some(err_string.clone());
                    warn!(
                        job_id = %job_id,
                        timeout_seconds = backfill_slice_timeout_seconds(),
                        "backfill worker slice timed out"
                    );
                }
            }
        }
    }

    let aggregation = run_aggregation_batch(pool, aggregation_jobs_per_slice()).await?;
    Ok(WorkerSliceOutcome {
        ran: backfill.ran || aggregation.claimed > 0,
        backfill,
        aggregation,
    })
}

fn backfill_slice_timeout_seconds() -> u64 {
    std::env::var("BACKFILL_SLICE_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_BACKFILL_SLICE_TIMEOUT_SECONDS)
}

fn backfill_channels_per_slice() -> usize {
    std::env::var("BACKFILL_CHANNELS_PER_SLICE")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_BACKFILL_CHANNELS_PER_SLICE)
}

fn users_pages_per_slice() -> usize {
    std::env::var("BACKFILL_USERS_PAGES_PER_SLICE")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_USERS_PAGES_PER_SLICE)
}

fn users_sync_interval_minutes() -> i64 {
    std::env::var("BACKFILL_USERS_SYNC_INTERVAL_MINUTES")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_USERS_SYNC_INTERVAL_MINUTES)
}

fn aggregation_jobs_per_slice() -> i64 {
    std::env::var("AGGREGATION_JOBS_PER_SLICE")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_AGGREGATION_JOBS_PER_SLICE)
}
