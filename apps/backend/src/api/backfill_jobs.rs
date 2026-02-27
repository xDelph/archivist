use sqlx::{PgPool, Row};

#[derive(Debug)]
pub struct EnqueueBackfillJobResult {
    pub job_id: String,
    pub queued_now: bool,
}

pub async fn enqueue_backfill_job(
    pool: &PgPool,
    requested_by: &str,
) -> anyhow::Result<EnqueueBackfillJobResult> {
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
