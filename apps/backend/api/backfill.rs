use archivist::api::admin::handler;
use archivist::logging::init_tracing;
use vercel_runtime::{Error, run, service_fn};

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();
    run(service_fn(handler)).await
}
