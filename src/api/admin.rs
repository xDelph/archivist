use std::env;

use vercel_runtime::{Body, Error, Request, Response, StatusCode};

/// Vercel entry-point — reads config from env and delegates to [`process`].
pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or_default();
    process(&admin_token, req).await
}

/// Core handler logic — token injected for testability.
pub(crate) async fn process(admin_token: &str, req: Request) -> Result<Response<Body>, Error> {
    let provided = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != format!("Bearer {}", admin_token) {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::Empty)?);
    }

    // Backfill logic wired in Phase 8.
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::Text(r#"{"ok":true}"#.into()))?)
}
