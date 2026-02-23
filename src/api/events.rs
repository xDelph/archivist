use vercel_runtime::{Body, Error, Request, Response, StatusCode};

/// POST /api/slack/events — placeholder, implemented in Phase 5.
pub async fn handler(_req: Request) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::Empty)?)
}
