use std::env;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use vercel_runtime::{Error, Request, Response, ResponseBody};

/// Vercel entry-point — reads config from env and delegates to [`process`].
pub async fn handler(req: Request) -> Result<Response<ResponseBody>, Error> {
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or_default();
    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let (parts, body) = process(&admin_token, req).await?.into_parts();
    Ok(Response::from_parts(parts, ResponseBody::from(body)))
}

/// Core handler logic — token injected for testability.
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
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Bytes::new())?);
    }

    // Backfill logic wired in Phase 8.
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Bytes::from(r#"{"ok":true}"#))?)
}
