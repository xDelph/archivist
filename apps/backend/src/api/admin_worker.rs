use std::env;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tracing::{info, warn};
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::api::aggregation_jobs::run_aggregation_batch;
use crate::api::backfill_jobs::{
    claim_next_backfill_job, mark_backfill_job_failed, mark_backfill_job_succeeded,
};
use crate::db::pool::create_pool;
use crate::slack::backfill::{SlackClient, run_backfill};
use crate::storage::R2Client;

static POOL: OnceCell<PgPool> = OnceCell::const_new();

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
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or_default();
    let cron_secret = env::var("CRON_SECRET").ok();
    let slack_token = env::var("SLACK_USER_TOKEN")
        .or_else(|_| env::var("SLACK_BOT_TOKEN"))
        .unwrap_or_default();

    let (parts, body) = req.into_parts();
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

    if let Some(job_id) = claim_next_backfill_job(db_pool)
        .await
        .map_err(|e| Error::from(e.to_string()))?
    {
        backfill_ran = true;
        backfill_job_id = Some(job_id.clone());
        info!(job_id = %job_id, "backfill worker claimed job");

        let storage = R2Client::from_env().await.ok();
        let client = SlackClient::new(slack_token.clone());
        if let Err(err) = run_backfill(db_pool, &client, &slack_token, storage.as_ref()).await {
            let err_string = err.to_string();
            mark_backfill_job_failed(db_pool, &job_id, &err_string)
                .await
                .map_err(|e| Error::from(e.to_string()))?;
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

        mark_backfill_job_succeeded(db_pool, &job_id)
            .await
            .map_err(|e| Error::from(e.to_string()))?;
        info!(job_id = %job_id, "backfill worker completed job");
    }

    let aggregation = run_aggregation_batch(db_pool, 100)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let body = serde_json::json!({
        "ok": true,
        "ranBackfill": backfill_ran,
        "backfillJobId": backfill_job_id,
        "aggregation": {
            "claimed": aggregation.claimed,
            "succeeded": aggregation.succeeded,
            "requeued": aggregation.requeued,
            "failed": aggregation.failed
        },
        "ran": backfill_ran || aggregation.claimed > 0,
    })
    .to_string();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from(body)))?)
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
