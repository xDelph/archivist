use std::env;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use vercel_runtime::{Error, Request, Response, ResponseBody};

use tracing::{error, info, warn};

use crate::api::backfill_jobs::enqueue_backfill_job;
use crate::db::pool::create_pool;

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

/// Vercel entry-point — reads config from env and delegates to [`process`].
pub async fn handler(req: Request) -> Result<Response<ResponseBody>, Error> {
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().unwrap_or("").to_owned();
    match tokio::spawn(async move { handle_request(req).await }).await {
        Ok(Ok(resp)) => Ok(resp),
        Ok(Err(err)) => {
            error!(method, path, query, error = %err, "admin handler failed");
            internal_error_response()
        }
        Err(join_err) => {
            error!(
                method,
                path,
                query,
                is_panic = join_err.is_panic(),
                error = %join_err,
                "admin handler task crashed"
            );
            internal_error_response()
        }
    }
}

async fn handle_request(req: Request) -> Result<Response<ResponseBody>, Error> {
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or_default();
    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let requested_by = req
        .headers()
        .get("x-trigger-source")
        .or_else(|| req.headers().get("user-agent"))
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("api")
        .to_owned();

    let resp = match process(&admin_token, req).await {
        Ok(r) => r,
        Err(e) => {
            let body = format!(r#"{{"ok":false,"error":"{}"}}"#, e);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(ResponseBody::from(Bytes::from(body)))?);
        }
    };
    if resp.status() != StatusCode::ACCEPTED {
        let (parts, body) = resp.into_parts();
        return Ok(Response::from_parts(parts, ResponseBody::from(body)));
    }

    let enqueue_result = enqueue_backfill_job(pool().await?, &requested_by)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    info!(
        job_id = %enqueue_result.job_id,
        queued_now = enqueue_result.queued_now,
        requested_by = %requested_by,
        "backfill job enqueued"
    );

    let body = serde_json::json!({
        "ok": true,
        "queued": enqueue_result.queued_now,
        "jobId": enqueue_result.job_id,
    })
    .to_string();

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from(body)))?)
}

fn internal_error_response() -> Result<Response<ResponseBody>, Error> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from_static(
            br#"{"ok":false,"error":"internal server error"}"#,
        )))?)
}

/// Core handler logic — auth check only (backfill is launched by [`handler`]).
pub(crate) async fn process(
    admin_token: &str,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    if req.method() != http::Method::POST {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Bytes::new())?);
    }

    let provided = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if admin_token.is_empty() || provided != format!("Bearer {}", admin_token) {
        warn!("unauthorized backfill attempt");
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Bytes::new())?);
    }

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Content-Type", "application/json")
        .body(Bytes::from(r#"{"ok":true}"#))?)
}
