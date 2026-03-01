use std::env;
use std::time::Duration;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tracing::{error, info, warn};
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::api::aggregation_jobs::run_aggregation_batch;
use crate::api::backfill_jobs::{
    claim_next_backfill_job, mark_backfill_job_failed, mark_backfill_job_succeeded,
    requeue_backfill_job, touch_backfill_job_lease,
};
use crate::api::worker_kick::{infer_base_url_from_headers, trigger_worker_kick};
use crate::db::pool::create_pool;
use crate::slack::backfill::{SlackClient, run_backfill};
use crate::storage::R2Client;

static POOL: OnceCell<PgPool> = OnceCell::const_new();
const DEFAULT_BACKFILL_RUN_TIMEOUT_SECONDS: u64 = 480;

async fn pool() -> Result<&'static PgPool, Error> {
    POOL.get_or_try_init(|| async {
        let url = env::var("DATABASE_URL").unwrap_or_default();
        create_pool(&url)
            .await
            .map_err(|e| Error::from(e.to_string()))
    })
    .await
}

pub async fn handler(req: Request) -> Result<Response<ResponseBody>, Error> {
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().unwrap_or("").to_owned();
    match tokio::spawn(async move { handle_request(req).await }).await {
        Ok(Ok(resp)) => Ok(resp),
        Ok(Err(err)) => {
            error!(method, path, query, error = %err, "admin worker handler failed");
            internal_error_response()
        }
        Err(join_err) => {
            error!(
                method,
                path,
                query,
                is_panic = join_err.is_panic(),
                error = %join_err,
                "admin worker handler task crashed"
            );
            internal_error_response()
        }
    }
}

async fn handle_request(req: Request) -> Result<Response<ResponseBody>, Error> {
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or_default();
    let cron_secret = env::var("CRON_SECRET").ok();
    let slack_token = env::var("SLACK_USER_TOKEN")
        .or_else(|_| env::var("SLACK_BOT_TOKEN"))
        .unwrap_or_default();

    let (parts, body) = req.into_parts();
    let base_url_hint = infer_base_url_from_headers(&parts.headers);
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let resp = process(&admin_token, cron_secret.as_deref(), req).await?;
    if resp.status() != StatusCode::ACCEPTED {
        let (parts, body) = resp.into_parts();
        return Ok(Response::from_parts(parts, ResponseBody::from(body)));
    }

    let db_pool = pool().await?;
    let mut backfill_ran = false;
    let mut backfill_job_id: Option<String> = None;
    let mut backfill_status = "idle";

    if let Some(job_id) = claim_next_backfill_job(db_pool)
        .await
        .map_err(|e| Error::from(e.to_string()))?
    {
        backfill_ran = true;
        backfill_job_id = Some(job_id.clone());
        backfill_status = "running";
        info!(job_id = %job_id, "backfill worker claimed job");

        let lease_pool = db_pool.clone();
        let lease_job_id = job_id.clone();
        let lease_heartbeat = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                if let Err(err) = touch_backfill_job_lease(&lease_pool, &lease_job_id).await {
                    warn!(
                        job_id = %lease_job_id,
                        error = %err,
                        "failed to refresh backfill job lease"
                    );
                }
            }
        });

        let storage = R2Client::from_env().await.ok();
        let client = SlackClient::new(slack_token.clone());
        let backfill_run = tokio::time::timeout(
            Duration::from_secs(backfill_run_timeout_seconds()),
            run_backfill(db_pool, &client, &slack_token, storage.as_ref()),
        )
        .await;
        lease_heartbeat.abort();
        match backfill_run {
            Ok(Ok(())) => {
                mark_backfill_job_succeeded(db_pool, &job_id)
                    .await
                    .map_err(|e| Error::from(e.to_string()))?;
                backfill_status = "succeeded";
                info!(job_id = %job_id, "backfill worker completed job");
            }
            Ok(Err(err)) => {
                let err_string = err.to_string();
                mark_backfill_job_failed(db_pool, &job_id, &err_string)
                    .await
                    .map_err(|e| Error::from(e.to_string()))?;
                backfill_status = "failed";
                warn!(
                    job_id = %job_id,
                    error = %err_string,
                    "backfill worker failed job"
                );
                let body = serde_json::json!({
                    "ok": false,
                    "ran": true,
                    "jobId": job_id,
                    "status": "failed",
                    "error": err_string,
                })
                .to_string();
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .body(ResponseBody::from(Bytes::from(body)))?);
            }
            Err(_) => {
                requeue_backfill_job(
                    db_pool,
                    &job_id,
                    "backfill worker slice timed out; requeued",
                )
                .await
                .map_err(|e| Error::from(e.to_string()))?;
                backfill_status = "requeued_timeout";
                warn!(
                    job_id = %job_id,
                    timeout_seconds = backfill_run_timeout_seconds(),
                    "backfill worker slice timed out; requeued"
                );
            }
        }
    }

    let aggregation = run_aggregation_batch(db_pool, 100)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let body = serde_json::json!({
        "ok": true,
        "ranBackfill": backfill_ran,
        "backfillJobId": backfill_job_id,
        "backfillStatus": backfill_status,
        "aggregation": {
            "claimed": aggregation.claimed,
            "succeeded": aggregation.succeeded,
            "requeued": aggregation.requeued,
            "failed": aggregation.failed
        },
        "ran": backfill_ran || aggregation.claimed > 0,
    })
    .to_string();
    if backfill_ran || aggregation.claimed > 0 {
        let kick_token = cron_secret
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| (!admin_token.is_empty()).then(|| admin_token.clone()));
        trigger_worker_kick(base_url_hint, kick_token, "worker_continue").await;
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from(body)))?)
}

fn backfill_run_timeout_seconds() -> u64 {
    std::env::var("BACKFILL_RUN_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_BACKFILL_RUN_TIMEOUT_SECONDS)
}

fn internal_error_response() -> Result<Response<ResponseBody>, Error> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from_static(
            br#"{"ok":false,"error":"internal server error"}"#,
        )))?)
}

pub(crate) async fn process(
    admin_token: &str,
    cron_secret: Option<&str>,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    if req.method() != http::Method::GET && req.method() != http::Method::POST {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Bytes::new())?);
    }

    let provided = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let admin_ok = !admin_token.is_empty() && provided == format!("Bearer {}", admin_token);
    let cron_ok = cron_secret
        .is_some_and(|secret| !secret.is_empty() && provided == format!("Bearer {}", secret));
    if !admin_ok && !cron_ok {
        warn!("unauthorized backfill worker attempt");
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Bytes::new())?);
    }

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Content-Type", "application/json")
        .body(Bytes::from(r#"{"ok":true}"#))?)
}
