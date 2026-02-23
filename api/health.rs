use vercel_runtime::{run, Error};
use archivist::api::health::handler;

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}
