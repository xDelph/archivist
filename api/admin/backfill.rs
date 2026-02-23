use vercel_runtime::{run, Error};
use archivist::api::admin::handler;

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}
