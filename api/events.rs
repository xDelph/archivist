use archivist::api::events::handler;
use vercel_runtime::{Error, run};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}
