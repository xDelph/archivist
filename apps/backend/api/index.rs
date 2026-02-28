use archivist::logging::init_tracing;
use http::StatusCode;
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
    init_tracing();
    run(service_fn(handler)).await
}
