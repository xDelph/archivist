use std::env;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tracing::{error, warn};
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::api::worker_engine::execute_worker_slice;
use crate::db::pool::create_pool;
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
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let resp = process(&admin_token, cron_secret.as_deref(), req).await?;
    if resp.status() != StatusCode::ACCEPTED {
        let (parts, body) = resp.into_parts();
        return Ok(Response::from_parts(parts, ResponseBody::from(body)));
    }

    let storage = R2Client::from_env().await.ok();
    let outcome = execute_worker_slice(pool().await?, &slack_token, storage.as_ref())
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let body = serde_json::to_string(&serde_json::json!({
        "ok": true,
        "worker": outcome,
    }))
    .map_err(|e| Error::from(e.to_string()))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
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
