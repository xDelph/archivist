use std::env;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use vercel_runtime::{Error, Request, Response, ResponseBody};

use tracing::{info, warn};

use crate::db::pool::create_pool;
use crate::slack::backfill::SlackClient;
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

/// Vercel entry-point — reads config from env and delegates to [`process`].
pub async fn handler(req: Request) -> Result<Response<ResponseBody>, Error> {
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or_default();
    // Prefer user token for backfill (full history access); fall back to bot token.
    let slack_token = env::var("SLACK_USER_TOKEN")
        .or_else(|_| env::var("SLACK_BOT_TOKEN"))
        .unwrap_or_default();
    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let pool = match pool().await {
        Ok(p) => p,
        Err(e) => {
            let body = format!(r#"{{"ok":false,"error":"db: {}"}}"#, e);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(ResponseBody::from(Bytes::from(body)))?);
        }
    };
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
    if resp.status() == StatusCode::ACCEPTED {
        let backfill_token = slack_token;
        let backfill_repo = pool;
        let backfill_storage = R2Client::from_env().await.ok();
        tokio::spawn(async move {
            let client = SlackClient::new(backfill_token.clone());
            info!("background backfill started");
            match crate::slack::backfill::run_backfill(
                backfill_repo,
                &client,
                &backfill_token,
                backfill_storage.as_ref(),
            )
            .await
            {
                Ok(()) => info!("background backfill completed"),
                Err(err) => warn!(error = %err, "background backfill failed"),
            }
        });
    }
    let (parts, body) = resp.into_parts();
    Ok(Response::from_parts(parts, ResponseBody::from(body)))
}

/// Core handler logic — auth check only (backfill is launched by [`handler`]).
pub(crate) async fn process(
    admin_token: &str,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    let provided = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != format!("Bearer {}", admin_token) {
        warn!("unauthorized backfill attempt");
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Bytes::new())?);
    }

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Content-Type", "application/json")
        .body(Bytes::from(r#"{"ok":true,"queued":true}"#))?)
}
