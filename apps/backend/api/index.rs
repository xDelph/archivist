use http::StatusCode;
use tracing_subscriber::EnvFilter;
use vercel_runtime::{Error, Request, Response, ResponseBody, run, service_fn};

const INDEX_HTML: &str = include_str!("../public/index.html");

async fn handler(_req: Request) -> Result<Response<ResponseBody>, Error> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(ResponseBody::from(INDEX_HTML))?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init()
        .ok();
    run(service_fn(handler)).await
}
