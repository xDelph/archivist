use std::env;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::db::Repository;
use crate::db::pool::create_pool;
use crate::slack::ingest::handle_event;
use crate::slack::signature::verify_signature;
use crate::slack::types::SlackEnvelope;

/// Shared pool — initialised once per Lambda instance, reused on warm starts.
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

/// Vercel entry-point — collects streaming body and delegates to [`process`].
pub async fn handler(req: Request) -> Result<Response<ResponseBody>, Error> {
    let signing_secret = env::var("SLACK_SIGNING_SECRET").unwrap_or_default();
    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let (parts, body) = process(pool().await?, &signing_secret, req)
        .await?
        .into_parts();
    Ok(Response::from_parts(parts, ResponseBody::from(body)))
}

/// Core handler logic — secrets and body bytes injected for testability.
pub(crate) async fn process<R: Repository>(
    repo: &R,
    signing_secret: &str,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    let raw_bytes = req.body().clone();

    // 1. Extract Slack headers
    let headers = req.headers();
    let timestamp = headers
        .get("x-slack-request-timestamp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let signature = headers
        .get("x-slack-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // 2. Verify HMAC signature — reject early on failure
    if verify_signature(signing_secret, timestamp, &raw_bytes, signature).is_err() {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Bytes::new())?);
    }

    // 3. Parse JSON — bad JSON is a client error
    let raw_value: serde_json::Value = match serde_json::from_slice(&raw_bytes) {
        Ok(v) => v,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Bytes::new())?);
        }
    };
    let envelope: SlackEnvelope = match serde_json::from_value(raw_value.clone()) {
        Ok(e) => e,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Bytes::new())?);
        }
    };

    // 4. Dispatch
    match envelope {
        SlackEnvelope::UrlVerification(uv) => {
            let body = serde_json::to_string(&uv).map_err(|e| Error::from(e.to_string()))?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Bytes::from(body))?)
        }
        SlackEnvelope::EventCallback(cb) => {
            handle_event(repo, *cb, raw_value)
                .await
                .map_err(|e| Error::from(e.to_string()))?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Bytes::new())?)
        }
        SlackEnvelope::Unknown => Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Bytes::new())?),
    }
}
