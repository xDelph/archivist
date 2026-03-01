use chrono::{Duration, Utc};
use sqlx::{PgPool, Row};
use tracing::warn;

const DEFAULT_RUNNING_LEASE_MINUTES: i64 = 10;

#[derive(Debug)]
pub struct EnqueueBackfillJobResult {
    pub job_id: String,
    pub queued_now: bool,
}

pub async fn enqueue_backfill_job(
    pool: &PgPool,
    requested_by: &str,
) -> anyhow::Result<EnqueueBackfillJobResult> {
    expire_stale_running_backfill_jobs(pool).await?;
    let row = sqlx::query(
        r#"
        WITH existing AS (
            SELECT id::text AS job_id
            FROM backfill_jobs
            WHERE status IN ('queued', 'running')
            ORDER BY requested_at ASC
            LIMIT 1
        ),
        inserted AS (
            INSERT INTO backfill_jobs (status, requested_by)
            SELECT 'queued', $1
            WHERE NOT EXISTS (SELECT 1 FROM existing)
            RETURNING id::text AS job_id
        )
        SELECT job_id, TRUE AS queued_now FROM inserted
        UNION ALL
        SELECT job_id, FALSE AS queued_now FROM existing
        LIMIT 1
        "#,
    )
    .bind(requested_by)
    .fetch_one(pool)
    .await?;

    Ok(EnqueueBackfillJobResult {
        job_id: row.try_get("job_id")?,
        queued_now: row.try_get("queued_now")?,
    })
}

pub async fn claim_next_backfill_job(pool: &PgPool) -> anyhow::Result<Option<String>> {
    expire_stale_running_backfill_jobs(pool).await?;
    let row = sqlx::query(
        r#"
        WITH next_job AS (
            SELECT id
            FROM backfill_jobs
            WHERE status = 'queued'
              AND NOT EXISTS (
                  SELECT 1
                  FROM backfill_jobs
                  WHERE status = 'running'
              )
            ORDER BY requested_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        UPDATE backfill_jobs j
        SET
            status = 'running',
            started_at = NOW(),
            finished_at = NULL,
            attempts = j.attempts + 1,
            last_error = NULL
        FROM next_job
        WHERE j.id = next_job.id
        RETURNING j.id::text AS job_id
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.try_get("job_id")).transpose()?)
}

async fn expire_stale_running_backfill_jobs(pool: &PgPool) -> anyhow::Result<()> {
    let cutoff = Utc::now() - Duration::minutes(running_lease_minutes());
    let row = sqlx::query(
        r#"
        UPDATE backfill_jobs
        SET
            status = 'failed',
            finished_at = NOW(),
            last_error = 'stale running lease expired'
        WHERE status = 'running'
          AND COALESCE(started_at, requested_at) < $1
        RETURNING id::text AS job_id
        "#,
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await?;

    if !row.is_empty() {
        warn!(
            recovered = row.len(),
            cutoff = %cutoff,
            "expired stale running backfill jobs"
        );
    }
    Ok(())
}

fn running_lease_minutes() -> i64 {
    std::env::var("BACKFILL_RUNNING_LEASE_MINUTES")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_RUNNING_LEASE_MINUTES)
}

pub async fn mark_backfill_job_succeeded(pool: &PgPool, job_id: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE backfill_jobs
        SET
            status = 'succeeded',
            finished_at = NOW(),
            last_error = NULL
        WHERE id = $1::uuid
        "#,
    )
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn touch_backfill_job_lease(pool: &PgPool, job_id: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE backfill_jobs
        SET started_at = NOW()
        WHERE id = $1::uuid
          AND status = 'running'
        "#,
    )
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn requeue_backfill_job(pool: &PgPool, job_id: &str, reason: &str) -> anyhow::Result<()> {
    let truncated_reason = if reason.len() > 8_000 {
        &reason[..8_000]
    } else {
        reason
    };
    sqlx::query(
        r#"
        UPDATE backfill_jobs
        SET
            status = 'queued',
            started_at = NULL,
            finished_at = NULL,
            last_error = $2
        WHERE id = $1::uuid
          AND status = 'running'
        "#,
    )
    .bind(job_id)
    .bind(truncated_reason)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_backfill_job_failed(
    pool: &PgPool,
    job_id: &str,
    error: &str,
) -> anyhow::Result<()> {
    let truncated_error = if error.len() > 8_000 {
        &error[..8_000]
    } else {
        error
    };
    sqlx::query(
        r#"
        UPDATE backfill_jobs
        SET
            status = 'failed',
            finished_at = NOW(),
            last_error = $2
        WHERE id = $1::uuid
        "#,
    )
    .bind(job_id)
    .bind(truncated_error)
    .execute(pool)
    .await?;
    Ok(())
}
