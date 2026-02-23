use std::env;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::db::Repository;
use crate::db::pool::create_pool;
use crate::slack::backfill::{SlackApi, SlackClient};

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
    let bot_token = env::var("SLACK_BOT_TOKEN").unwrap_or_default();
    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let client = SlackClient::new(bot_token);
    let (parts, body) = process(&admin_token, req, pool().await?, &client)
        .await?
        .into_parts();
    Ok(Response::from_parts(parts, ResponseBody::from(body)))
}

/// Core handler logic — token, repo, and client injected for testability.
pub(crate) async fn process<R, S>(
    admin_token: &str,
    req: http::Request<Bytes>,
    repo: &R,
    client: &S,
) -> Result<Response<Bytes>, Error>
where
    R: Repository,
    S: SlackApi,
{
    let provided = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != format!("Bearer {}", admin_token) {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Bytes::new())?);
    }

    crate::slack::backfill::run_backfill(repo, client)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Bytes::from(r#"{"ok":true}"#))?)
}
