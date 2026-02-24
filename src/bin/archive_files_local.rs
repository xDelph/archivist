/// One-shot script: scan all existing messages in the DB for attached files
/// and upload any not yet archived to Cloudflare R2.
///
/// Pass `--purge` to wipe all existing `files` DB records first (use this to
/// clean up bad uploads from when the HTML-redirect bug was present).
///
/// Safe to run multiple times without `--purge` — already-archived files are skipped.
use std::env;

use archivist::db::pool::create_pool;
use archivist::slack::backfill::archive_files;
use archivist::storage::R2Client;
use sqlx::Row;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let purge = env::args().any(|a| a == "--purge");

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let slack_token = env::var("SLACK_USER_TOKEN")
        .or_else(|_| env::var("SLACK_BOT_TOKEN"))
        .expect("SLACK_USER_TOKEN or SLACK_BOT_TOKEN not set");

    let pool = create_pool(&db_url).await?;

    let storage = R2Client::from_env().await.ok();
    if storage.is_none() {
        warn!("R2 not configured — files will be skipped. Set CLOUDFLARED_R2_* env vars.");
    }

    if purge {
        let deleted = sqlx::query("DELETE FROM files")
            .execute(&pool)
            .await?
            .rows_affected();
        info!(deleted, "--purge: cleared files table");
    }

    // Runtime query (no macro) — no sqlx prepare needed.
    let rows = sqlx::query(
        r#"
        SELECT
            channel_id,
            ts,
            COALESCE(raw_json->>'team', '') AS team_id,
            raw_json->'files'               AS files_json
        FROM messages
        WHERE raw_json->'files' IS NOT NULL
          AND jsonb_array_length(raw_json->'files') > 0
        ORDER BY ts ASC
        "#,
    )
    .fetch_all(&pool)
    .await?;

    info!(total = rows.len(), "messages with files found");

    let mut done = 0usize;
    for row in &rows {
        let channel_id: String = row.try_get("channel_id")?;
        let ts: String = row.try_get("ts")?;
        let team_id: String = row.try_get("team_id")?;
        let files_json: serde_json::Value = row.try_get("files_json")?;

        archive_files(
            &pool,
            storage.as_ref(),
            &slack_token,
            &channel_id,
            &ts,
            &team_id,
            &files_json,
        )
        .await?;

        done += 1;
        if done.is_multiple_of(50) {
            info!(done, total = rows.len(), "progress");
        }
    }

    info!(done, "archive_files_local complete");
    Ok(())
}
