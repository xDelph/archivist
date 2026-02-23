use archivist::api::health::handler;
use vercel_runtime::{Error, run};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}
