use super::archive::{ArchiveSlackFile, archive_slack_file, remote_object_exists};
use super::backfill_slack::SlackHistoryMessage;
use super::storage_paths::resolve_file_storage_location;
use super::{AppState, ErrorResponse, store_failed};
use axum::{Json, http::StatusCode};
use std::collections::HashSet;

pub(super) async fn persist_backfill_file_archives(
    state: &AppState,
    slack_user_token: &str,
    channel_id: &str,
    messages: &[SlackHistoryMessage],
) -> Result<usize, (StatusCode, Json<ErrorResponse>)> {
    let Some(_) = state.r2_config.as_ref() else {
        return Ok(0);
    };

    let mut seen_file_ids = HashSet::new();
    let mut files = Vec::new();
    for message in messages {
        for file in &message.files {
            if seen_file_ids.contains(&file.id) {
                continue;
            }
            seen_file_ids.insert(file.id.clone());
            files.push((
                message.ts.as_str(),
                file.id.as_str(),
                file.name.as_deref().unwrap_or(file.id.as_str()),
                file.url_private_download
                    .as_deref()
                    .or(file.url_private.as_deref()),
                file.mimetype.as_deref(),
            ));
        }
    }

    if files.is_empty() {
        return Ok(0);
    }

    let mut updated = 0usize;
    for (message_ts, file_id, file_name, download_url, mimetype) in files {
        let location =
            resolve_file_storage_location(state, slack_user_token, channel_id, file_id, file_name)
                .await
                .expect("r2 config is present");
        state
            .store
            .set_file_archive(file_id, &location.storage_key, &location.public_url)
            .await
            .map_err(store_failed)?;

        if download_url.is_none() {
            if remote_object_exists(&location.public_url).await {
                tracing::info!(
                    channel_id,
                    message_ts,
                    file_id,
                    storage_key = %location.storage_key,
                    public_url = %location.public_url,
                    "reused existing archived file without slack download url"
                );
                updated += 1;
                continue;
            }

            tracing::error!(
                channel_id,
                message_ts,
                file_id,
                storage_key = %location.storage_key,
                public_url = %location.public_url,
                "cannot archive file without slack private download url"
            );
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: "missing_file_download_url",
                }),
            ));
        }

        let archived = archive_slack_file(
            state,
            slack_user_token,
            ArchiveSlackFile {
                channel_id,
                message_ts,
                file_id,
                filename: file_name,
                download_url: download_url.expect("checked above"),
                mimetype,
            },
        )
        .await?;
        tracing::info!(
            channel_id,
            message_ts,
            file_id,
            storage_key = %archived.storage_key,
            public_url = %archived.public_url,
            bytes_uploaded = archived.bytes_uploaded,
            "persisted backfill file archive"
        );
        updated += 1;
    }

    tracing::info!(
        channel_id,
        files_archived = updated,
        "completed backfill file archiving"
    );
    Ok(updated)
}
