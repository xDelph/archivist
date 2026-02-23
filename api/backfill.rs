use archivist::api::admin::handler;
use vercel_runtime::{Error, run, service_fn};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handler)).await
}
