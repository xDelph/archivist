use std::env;

use archivist::db::pool::create_pool;
use archivist::slack::backfill::{SlackClient, run_backfill};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let slack_token = env::var("SLACK_USER_TOKEN")
        .or_else(|_| env::var("SLACK_BOT_TOKEN"))
        .expect("SLACK_USER_TOKEN or SLACK_BOT_TOKEN not set");

    let pool = create_pool(&db_url).await?;
    let client = SlackClient::new(slack_token);

    run_backfill(&pool, &client).await?;

    Ok(())
}
