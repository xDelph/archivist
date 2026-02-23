use archivist::api::admin::handler;
use vercel_runtime::{Error, run};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}
