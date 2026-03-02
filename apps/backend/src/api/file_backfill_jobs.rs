use anyhow::Result;
use serde::Serialize;
use sqlx::{PgPool, Row};
use tracing::{info, warn};

use crate::slack::backfill::archive_files;
use crate::storage::R2Client;

const MAX_ERROR_LEN: usize = 8_000;
const DEFAULT_FILE_BACKFILL_RUNNING_LEASE_MINUTES: i64 = 15;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBackfillBatchResult {
    pub claimed: usize,
    pub succeeded: usize,
    pub requeued: usize,
    pub failed: usize,
}

#[derive(Debug)]
struct FileBackfillJobClaim {
    id: String,
    team_id: String,
    channel_id: String,
    message_ts: String,
    files_json: serde_json::Value,
    attempts: i32,
    max_attempts: i32,
}

pub async fn run_file_backfill_batch(
    pool: &PgPool,
    slack_token: &str,
    storage: Option<&R2Client>,
) -> Result<FileBackfillBatchResult> {
    if slack_token.is_empty() {
        return Ok(FileBackfillBatchResult::default());
    }

    expire_stale_running_file_backfill_jobs(pool).await?;
    let jobs = claim_next_file_backfill_jobs(pool).await?;
    if jobs.is_empty() {
        return Ok(FileBackfillBatchResult::default());
    }

    let mut result = FileBackfillBatchResult {
        claimed: jobs.len(),
        ..FileBackfillBatchResult::default()
    };

    for job in jobs {
        let process = archive_files(
            pool,
            storage,
            slack_token,
            &job.channel_id,
            &job.message_ts,
            &job.team_id,
            &job.files_json,
        )
        .await;
        match process {
            Ok(()) => {
                mark_file_backfill_job_succeeded(pool, &job.id).await?;
                result.succeeded += 1;
            }
            Err(err) => {
                let err_message = err.to_string();
                let terminal = mark_file_backfill_job_failed(pool, &job.id, &err_message).await?;
                if terminal {
                    result.failed += 1;
                    warn!(
                        job_id = %job.id,
                        channel_id = %job.channel_id,
                        message_ts = %job.message_ts,
                        attempts = job.attempts,
                        max_attempts = job.max_attempts,
                        error = %err_message,
                        "file backfill job permanently failed"
                    );
                } else {
                    result.requeued += 1;
                    warn!(
                        job_id = %job.id,
                        channel_id = %job.channel_id,
                        message_ts = %job.message_ts,
                        attempts = job.attempts,
                        max_attempts = job.max_attempts,
                        error = %err_message,
                        "file backfill job requeued after failure"
                    );
                }
            }
        }
    }

    info!(
        claimed = result.claimed,
        succeeded = result.succeeded,
        requeued = result.requeued,
        failed = result.failed,
        "file backfill batch complete"
    );
    Ok(result)
}

async fn claim_next_file_backfill_jobs(pool: &PgPool) -> Result<Vec<FileBackfillJobClaim>> {
    let rows = sqlx::query(
        r#"
        WITH next_jobs AS (
            SELECT id
            FROM file_backfill_jobs
            WHERE status = 'queued'
            ORDER BY created_at ASC
            FOR UPDATE SKIP LOCKED
        )
        UPDATE file_backfill_jobs j
        SET
            status = 'running',
            started_at = NOW(),
            attempts = j.attempts + 1,
            last_error = NULL,
            updated_at = NOW()
        FROM next_jobs
        WHERE j.id = next_jobs.id
        RETURNING
            j.id::text AS id,
            j.team_id,
            j.channel_id,
            j.message_ts,
            j.files_json,
            j.attempts,
            j.max_attempts
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| FileBackfillJobClaim {
            id: row.get("id"),
            team_id: row.get("team_id"),
            channel_id: row.get("channel_id"),
            message_ts: row.get("message_ts"),
            files_json: row.get("files_json"),
            attempts: row.get("attempts"),
            max_attempts: row.get("max_attempts"),
        })
        .collect())
}

async fn expire_stale_running_file_backfill_jobs(pool: &PgPool) -> Result<()> {
    let lease_minutes = std::env::var("FILE_BACKFILL_RUNNING_LEASE_MINUTES")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_FILE_BACKFILL_RUNNING_LEASE_MINUTES);
    let rows = sqlx::query(
        r#"
        UPDATE file_backfill_jobs
        SET
            status = CASE
                WHEN attempts >= max_attempts THEN 'failed'
                ELSE 'queued'
            END,
            started_at = NULL,
            finished_at = CASE
                WHEN attempts >= max_attempts THEN NOW()
                ELSE NULL
            END,
            last_error = 'stale running lease expired',
            updated_at = NOW()
        WHERE status = 'running'
          AND COALESCE(started_at, updated_at, created_at)
              < (NOW() - ($1::text || ' minutes')::interval)
        RETURNING id
        "#,
    )
    .bind(lease_minutes)
    .fetch_all(pool)
    .await?;

    if !rows.is_empty() {
        warn!(
            recovered = rows.len(),
            lease_minutes, "recovered stale running file backfill jobs"
        );
    }
    Ok(())
}

async fn mark_file_backfill_job_succeeded(pool: &PgPool, job_id: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE file_backfill_jobs
        SET
            status = 'succeeded',
            finished_at = NOW(),
            last_error = NULL,
            updated_at = NOW()
        WHERE id = $1::uuid
        "#,
    )
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn mark_file_backfill_job_failed(pool: &PgPool, job_id: &str, error: &str) -> Result<bool> {
    let truncated_error = if error.len() > MAX_ERROR_LEN {
        &error[..MAX_ERROR_LEN]
    } else {
        error
    };

    let row = sqlx::query(
        r#"
        UPDATE file_backfill_jobs
        SET
            status = CASE
                WHEN attempts >= max_attempts THEN 'failed'
                ELSE 'queued'
            END,
            finished_at = CASE
                WHEN attempts >= max_attempts THEN NOW()
                ELSE NULL
            END,
            last_error = $2,
            updated_at = NOW()
        WHERE id = $1::uuid
        RETURNING status
        "#,
    )
    .bind(job_id)
    .bind(truncated_error)
    .fetch_one(pool)
    .await?;

    let status: String = row.get("status");
    Ok(status == "failed")
}
