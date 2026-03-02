use std::env;

use archivist::api::file_backfill_jobs::run_file_backfill_batch;
use archivist::db::pool::create_pool;
use archivist::logging::init_tracing;
use archivist::slack::backfill::{SlackClient, run_backfill, sync_all_users};
use archivist::storage::R2Client;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let slack_token = env::var("SLACK_USER_TOKEN")
        .or_else(|_| env::var("SLACK_BOT_TOKEN"))
        .expect("SLACK_USER_TOKEN or SLACK_BOT_TOKEN not set");

    let pool = create_pool(&db_url).await?;
    let client = SlackClient::new(slack_token.clone());
    let storage = R2Client::from_env().await.ok();

    run_backfill(&pool, &client, &slack_token, storage.as_ref()).await?;

    loop {
        let batch = run_file_backfill_batch(&pool, &slack_token, storage.as_ref()).await?;
        info!(
            claimed = batch.claimed,
            succeeded = batch.succeeded,
            failed = batch.failed,
            requeued = batch.requeued,
            "file backfill local batch complete"
        );
        if batch.claimed == 0 {
            break;
        }
    }

    let users_cached = sync_all_users(&pool, &client).await?;
    info!(users_cached, "full users cache complete");

    Ok(())
}
