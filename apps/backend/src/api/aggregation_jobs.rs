use anyhow::Result;
use serde::Serialize;
use sqlx::{PgPool, Row};
use tracing::{info, warn};

const MAX_ERROR_LEN: usize = 8_000;
const DEFAULT_AGGREGATION_RUNNING_LEASE_MINUTES: i64 = 15;

#[derive(Debug, Default, Serialize)]
pub struct AggregationBatchResult {
    pub claimed: usize,
    pub succeeded: usize,
    pub requeued: usize,
    pub failed: usize,
}

#[derive(Debug)]
struct AggregationJobClaim {
    id: String,
    channel_id: String,
    thread_ts: String,
    attempts: i32,
    max_attempts: i32,
}

pub async fn run_aggregation_batch(pool: &PgPool, max_jobs: i64) -> Result<AggregationBatchResult> {
    expire_stale_running_aggregation_jobs(pool).await?;
    let jobs = claim_next_aggregation_jobs(pool, max_jobs).await?;
    if jobs.is_empty() {
        info!("aggregation batch empty");
        return Ok(AggregationBatchResult::default());
    }

    let mut result = AggregationBatchResult {
        claimed: jobs.len(),
        ..AggregationBatchResult::default()
    };

    for job in jobs {
        match recompute_thread_rollups(pool, &job.channel_id, &job.thread_ts).await {
            Ok(()) => {
                mark_aggregation_job_succeeded(pool, &job.id).await?;
                result.succeeded += 1;
            }
            Err(err) => {
                let err_message = err.to_string();
                let terminal = mark_aggregation_job_failed(pool, &job.id, &err_message).await?;
                if terminal {
                    result.failed += 1;
                    warn!(
                        job_id = %job.id,
                        channel_id = %job.channel_id,
                        thread_ts = %job.thread_ts,
                        attempts = job.attempts,
                        max_attempts = job.max_attempts,
                        error = %err_message,
                        "aggregation job permanently failed"
                    );
                } else {
                    result.requeued += 1;
                    warn!(
                        job_id = %job.id,
                        channel_id = %job.channel_id,
                        thread_ts = %job.thread_ts,
                        attempts = job.attempts,
                        max_attempts = job.max_attempts,
                        error = %err_message,
                        "aggregation job requeued after failure"
                    );
                }
            }
        }
    }

    recompute_workspace_overview_rollup(pool).await?;
    info!(
        claimed = result.claimed,
        succeeded = result.succeeded,
        requeued = result.requeued,
        failed = result.failed,
        "aggregation batch complete"
    );
    Ok(result)
}

pub async fn pending_aggregation_jobs_count(pool: &PgPool) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM aggregation_jobs
        WHERE status = 'queued'
          AND available_at <= NOW()
        "#,
    )
    .fetch_one(pool)
    .await?;
    Ok(count)
}

async fn claim_next_aggregation_jobs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<AggregationJobClaim>> {
    let rows = sqlx::query(
        r#"
        WITH next_jobs AS (
            SELECT id
            FROM aggregation_jobs
            WHERE status = 'queued'
              AND available_at <= NOW()
            ORDER BY available_at ASC, created_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT $1
        )
        UPDATE aggregation_jobs j
        SET
            status = 'running',
            locked_at = NOW(),
            started_at = NOW(),
            attempts = j.attempts + 1,
            last_error = NULL,
            updated_at = NOW()
        FROM next_jobs
        WHERE j.id = next_jobs.id
        RETURNING
            j.id::text AS id,
            j.channel_id,
            j.thread_ts,
            j.attempts,
            j.max_attempts
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AggregationJobClaim {
            id: row.get("id"),
            channel_id: row.get("channel_id"),
            thread_ts: row.get("thread_ts"),
            attempts: row.get("attempts"),
            max_attempts: row.get("max_attempts"),
        })
        .collect())
}

async fn expire_stale_running_aggregation_jobs(pool: &PgPool) -> Result<()> {
    let lease_minutes = std::env::var("AGGREGATION_RUNNING_LEASE_MINUTES")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_AGGREGATION_RUNNING_LEASE_MINUTES);
    let row = sqlx::query(
        r#"
        UPDATE aggregation_jobs
        SET
            status = CASE
                WHEN attempts >= max_attempts THEN 'failed'
                ELSE 'queued'
            END,
            available_at = NOW(),
            locked_at = NULL,
            started_at = NULL,
            finished_at = CASE
                WHEN attempts >= max_attempts THEN NOW()
                ELSE NULL
            END,
            last_error = 'stale running lease expired',
            updated_at = NOW()
        WHERE status = 'running'
          AND COALESCE(locked_at, started_at, updated_at, created_at)
              < (NOW() - ($1::text || ' minutes')::interval)
        RETURNING id::text AS job_id, status
        "#,
    )
    .bind(lease_minutes)
    .fetch_all(pool)
    .await?;

    if !row.is_empty() {
        warn!(
            recovered = row.len(),
            lease_minutes, "recovered stale running aggregation jobs"
        );
    }
    Ok(())
}

async fn mark_aggregation_job_succeeded(pool: &PgPool, job_id: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE aggregation_jobs
        SET
            status = 'succeeded',
            locked_at = NULL,
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

async fn mark_aggregation_job_failed(pool: &PgPool, job_id: &str, error: &str) -> Result<bool> {
    let truncated_error = if error.len() > MAX_ERROR_LEN {
        &error[..MAX_ERROR_LEN]
    } else {
        error
    };

    let row = sqlx::query(
        r#"
        UPDATE aggregation_jobs
        SET
            status = CASE
                WHEN attempts >= max_attempts THEN 'failed'
                ELSE 'queued'
            END,
            available_at = CASE
                WHEN attempts >= max_attempts THEN available_at
                ELSE NOW() + (LEAST(300, attempts * 15)::text || ' seconds')::interval
            END,
            locked_at = NULL,
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

async fn recompute_thread_rollups(pool: &PgPool, channel_id: &str, thread_ts: &str) -> Result<()> {
    recompute_thread_rollup_row(pool, channel_id, thread_ts).await?;
    rebuild_period_scores_for_thread(pool, channel_id, thread_ts).await?;
    recompute_channel_daily_rollup_for_thread(pool, channel_id, thread_ts).await?;
    Ok(())
}

async fn recompute_thread_rollup_row(
    pool: &PgPool,
    channel_id: &str,
    thread_ts: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        WITH resolved AS (
            SELECT $1::text AS channel_id, $2::text AS thread_ts
        ),
        thread_messages AS (
            SELECT m.*
            FROM messages m
            JOIN resolved r
              ON m.channel_id = r.channel_id
            WHERE m.ts = r.thread_ts OR m.thread_ts = r.thread_ts
        ),
        root_msg AS (
            SELECT tm.user_id, tm.text, tm.created_at
            FROM thread_messages tm
            JOIN resolved r ON tm.ts = r.thread_ts
            ORDER BY tm.created_at ASC
            LIMIT 1
        ),
        reaction_totals AS (
            SELECT COALESCE(SUM((rr_elem->'count')::bigint), 0)::bigint AS reaction_count_total
            FROM thread_messages tm
            CROSS JOIN LATERAL jsonb_array_elements(
                CASE
                    WHEN jsonb_typeof(tm.raw_json->'reactions') = 'array'
                    THEN tm.raw_json->'reactions'
                    ELSE '[]'::jsonb
                END
            ) rr_elem
        ),
        reply_totals AS (
            SELECT COALESCE(COUNT(*) FILTER (WHERE tm.ts <> r.thread_ts), 0)::bigint AS reply_count_total
            FROM thread_messages tm
            CROSS JOIN resolved r
        ),
        participant_totals AS (
            SELECT GREATEST(
                COALESCE(COUNT(DISTINCT tm.user_id) FILTER (WHERE tm.user_id IS NOT NULL), 0),
                CASE WHEN EXISTS (SELECT 1 FROM thread_messages) THEN 1 ELSE 0 END
            )::bigint AS participant_count_total
            FROM thread_messages tm
        ),
        file_totals AS (
            SELECT COALESCE(COUNT(*)::bigint, 0) AS file_count_total
            FROM files f
            JOIN resolved r
              ON f.channel_id = r.channel_id
             AND f.message_ts = r.thread_ts
        ),
        search_texts AS (
            SELECT
                LEFT(
                    TRIM(
                        REGEXP_REPLACE(
                            COALESCE(STRING_AGG(COALESCE(tm.text, ''), ' ' ORDER BY tm.ts), ''),
                            '\s+',
                            ' ',
                            'g'
                        )
                    ),
                    4000
                ) AS search_text
            FROM thread_messages tm
        ),
        source_meta AS (
            SELECT COALESCE(MAX(tm.updated_at), NOW()) AS source_max_updated_at
            FROM thread_messages tm
        )
        INSERT INTO thread_rollups (
            channel_id,
            thread_ts,
            root_user_id,
            root_text,
            root_created_at,
            search_text,
            reaction_count_total,
            reply_count_total,
            participant_count_total,
            file_count_total,
            has_files,
            score_total,
            computed_at,
            source_max_updated_at,
            version
        )
        SELECT
            r.channel_id,
            r.thread_ts,
            rm.user_id,
            COALESCE(rm.text, ''),
            COALESCE(rm.created_at, NOW()),
            st.search_text,
            rt.reaction_count_total,
            rpt.reply_count_total,
            pt.participant_count_total,
            ft.file_count_total,
            (ft.file_count_total > 0),
            (rt.reaction_count_total * 2 + rpt.reply_count_total + pt.participant_count_total)::bigint,
            NOW(),
            sm.source_max_updated_at,
            1
        FROM resolved r
        LEFT JOIN root_msg rm ON TRUE
        CROSS JOIN reaction_totals rt
        CROSS JOIN reply_totals rpt
        CROSS JOIN participant_totals pt
        CROSS JOIN file_totals ft
        CROSS JOIN search_texts st
        CROSS JOIN source_meta sm
        WHERE EXISTS (
            SELECT 1
            FROM messages m
            WHERE m.channel_id = r.channel_id
              AND (m.ts = r.thread_ts OR m.thread_ts = r.thread_ts)
        )
        ON CONFLICT (channel_id, thread_ts)
        DO UPDATE SET
            root_user_id = EXCLUDED.root_user_id,
            root_text = EXCLUDED.root_text,
            root_created_at = EXCLUDED.root_created_at,
            search_text = EXCLUDED.search_text,
            reaction_count_total = EXCLUDED.reaction_count_total,
            reply_count_total = EXCLUDED.reply_count_total,
            participant_count_total = EXCLUDED.participant_count_total,
            file_count_total = EXCLUDED.file_count_total,
            has_files = EXCLUDED.has_files,
            score_total = EXCLUDED.score_total,
            computed_at = NOW(),
            source_max_updated_at = EXCLUDED.source_max_updated_at,
            version = EXCLUDED.version
        "#,
    )
    .bind(channel_id)
    .bind(thread_ts)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM thread_rollups
        WHERE channel_id = $1
          AND thread_ts = $2
          AND NOT EXISTS (
              SELECT 1
              FROM messages m
              WHERE m.channel_id = $1
                AND (m.ts = $2 OR m.thread_ts = $2)
          )
        "#,
    )
    .bind(channel_id)
    .bind(thread_ts)
    .execute(pool)
    .await?;

    Ok(())
}

async fn rebuild_period_scores_for_thread(
    pool: &PgPool,
    channel_id: &str,
    thread_ts: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM thread_period_scores
        WHERE channel_id = $1
          AND thread_ts = $2
          AND period_kind IN ('week', 'month')
        "#,
    )
    .bind(channel_id)
    .bind(thread_ts)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO thread_period_scores (
            period_kind,
            period_start,
            channel_id,
            thread_ts,
            score_period,
            computed_at,
            source_max_updated_at,
            version
        )
        SELECT
            'week',
            date_trunc('week', CURRENT_DATE)::date,
            tr.channel_id,
            tr.thread_ts,
            tr.score_total,
            NOW(),
            tr.source_max_updated_at,
            1
        FROM thread_rollups tr
        WHERE tr.channel_id = $1
          AND tr.thread_ts = $2
          AND tr.thread_ts ~ '^[0-9]+(\.[0-9]+)?$'
          AND to_timestamp(tr.thread_ts::double precision) >= date_trunc('week', CURRENT_DATE)::timestamp
          AND to_timestamp(tr.thread_ts::double precision) < (date_trunc('week', CURRENT_DATE)::timestamp + INTERVAL '7 days')
        ON CONFLICT (period_kind, period_start, channel_id, thread_ts)
        DO UPDATE SET
            score_period = EXCLUDED.score_period,
            computed_at = NOW(),
            source_max_updated_at = EXCLUDED.source_max_updated_at,
            version = EXCLUDED.version
        "#,
    )
    .bind(channel_id)
    .bind(thread_ts)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO thread_period_scores (
            period_kind,
            period_start,
            channel_id,
            thread_ts,
            score_period,
            computed_at,
            source_max_updated_at,
            version
        )
        SELECT
            'month',
            date_trunc('month', CURRENT_DATE)::date,
            tr.channel_id,
            tr.thread_ts,
            tr.score_total,
            NOW(),
            tr.source_max_updated_at,
            1
        FROM thread_rollups tr
        WHERE tr.channel_id = $1
          AND tr.thread_ts = $2
          AND tr.thread_ts ~ '^[0-9]+(\.[0-9]+)?$'
          AND to_timestamp(tr.thread_ts::double precision) >= date_trunc('month', CURRENT_DATE)::timestamp
          AND to_timestamp(tr.thread_ts::double precision) < (date_trunc('month', CURRENT_DATE)::timestamp + INTERVAL '1 month')
        ON CONFLICT (period_kind, period_start, channel_id, thread_ts)
        DO UPDATE SET
            score_period = EXCLUDED.score_period,
            computed_at = NOW(),
            source_max_updated_at = EXCLUDED.source_max_updated_at,
            version = EXCLUDED.version
        "#,
    )
    .bind(channel_id)
    .bind(thread_ts)
    .execute(pool)
    .await?;

    Ok(())
}

async fn recompute_channel_daily_rollup_for_thread(
    pool: &PgPool,
    channel_id: &str,
    thread_ts: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        WITH target_day AS (
            SELECT date_trunc('day', to_timestamp($2::double precision))::date AS day
            WHERE $2 ~ '^[0-9]+(\.[0-9]+)?$'
        ),
        daily AS (
            SELECT
                d.day,
                $1::text AS channel_id,
                COALESCE(COUNT(*)::bigint, 0) AS thread_count,
                COALESCE(SUM(tr.reply_count_total + 1)::bigint, 0) AS message_count
            FROM target_day d
            LEFT JOIN thread_rollups tr
              ON tr.channel_id = $1
             AND tr.thread_ts ~ '^[0-9]+(\.[0-9]+)?$'
             AND date_trunc('day', to_timestamp(tr.thread_ts::double precision))::date = d.day
            GROUP BY d.day
        )
        INSERT INTO channel_daily_rollups (day, channel_id, thread_count, message_count, computed_at, version)
        SELECT day, channel_id, thread_count, message_count, NOW(), 1
        FROM daily
        ON CONFLICT (day, channel_id)
        DO UPDATE SET
            thread_count = EXCLUDED.thread_count,
            message_count = EXCLUDED.message_count,
            computed_at = NOW(),
            version = EXCLUDED.version
        "#,
    )
    .bind(channel_id)
    .bind(thread_ts)
    .execute(pool)
    .await?;

    Ok(())
}

async fn recompute_workspace_overview_rollup(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        WITH totals AS (
            SELECT
                COALESCE(SUM(reply_count_total + 1), 0)::bigint AS total_messages,
                COALESCE(COUNT(*), 0)::bigint AS total_threads,
                COALESCE(SUM(file_count_total), 0)::bigint AS total_files,
                COALESCE(COUNT(DISTINCT root_user_id), 0)::bigint AS total_users
            FROM thread_rollups
        )
        INSERT INTO workspace_overview_rollups (
            snapshot_key,
            total_messages,
            total_threads,
            total_files,
            total_users,
            messages_change,
            threads_change,
            files_change,
            users_change,
            computed_at,
            version
        )
        SELECT
            'latest',
            total_messages,
            total_threads,
            total_files,
            total_users,
            0,
            0,
            0,
            0,
            NOW(),
            1
        FROM totals
        ON CONFLICT (snapshot_key)
        DO UPDATE SET
            total_messages = EXCLUDED.total_messages,
            total_threads = EXCLUDED.total_threads,
            total_files = EXCLUDED.total_files,
            total_users = EXCLUDED.total_users,
            messages_change = EXCLUDED.messages_change,
            threads_change = EXCLUDED.threads_change,
            files_change = EXCLUDED.files_change,
            users_change = EXCLUDED.users_change,
            computed_at = NOW(),
            version = EXCLUDED.version
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
