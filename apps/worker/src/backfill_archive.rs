use super::backfill_slack::SlackHistoryMessage;
use super::storage_paths::{build_file_storage_location, resolve_storage_prefix};
use super::{AppState, ErrorResponse, store_failed};
use axum::{Json, http::StatusCode};
use std::collections::HashSet;

pub(super) async fn persist_backfill_file_archives(
    state: &AppState,
    slack_user_token: &str,
    channel_id: &str,
    messages: &[SlackHistoryMessage],
) -> Result<usize, (StatusCode, Json<ErrorResponse>)> {
    let Some(r2_config) = state.r2_config.as_ref() else {
        return Ok(0);
    };

    let storage_prefix = resolve_storage_prefix(state, slack_user_token).await;
    let files: HashSet<(String, String)> = messages
        .iter()
        .flat_map(|message| {
            message.files.iter().map(|file| {
                (
                    file.id.clone(),
                    file.name.clone().unwrap_or_else(|| file.id.clone()),
                )
            })
        })
        .collect();

    if files.is_empty() {
        return Ok(0);
    }

    let mut updated = 0usize;
    for (file_id, file_name) in files {
        let location = build_file_storage_location(
            r2_config,
            channel_id,
            &file_id,
            &file_name,
            storage_prefix.as_deref(),
        );
        state
            .store
            .set_file_archive(&file_id, &location.storage_key, &location.public_url)
            .await
            .map_err(|error| {
                tracing::error!(
                    ?error,
                    channel_id,
                    file_id = %file_id,
                    storage_key = %location.storage_key,
                    "failed to persist backfill file archive metadata"
                );
                store_failed(error)
            })?;
        updated += 1;
    }

    tracing::info!(
        channel_id,
        files_archived = updated,
        "persisted backfill file archive metadata"
    );
    Ok(updated)
}
