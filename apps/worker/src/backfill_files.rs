use super::backfill_archive::persist_backfill_file_archives;
use super::backfill_slack::{SlackHistoryMessage, fetch_message_history};
use super::{AppState, ErrorResponse, store_failed};
use axum::{Json, body::Bytes, extract::State, http::StatusCode};
use domain::File;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Default, Deserialize)]
struct BackfillFilesRequest {
    channel_id: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct BackfillFilesResponse {
    ok: bool,
    archived: usize,
    skipped: usize,
    failed: usize,
}

pub(crate) async fn backfill_files(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<BackfillFilesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request = parse_backfill_files_request(&body)?;
    let channel_id = request
        .channel_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let slack_user_token = state.slack_user_token.as_deref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_slack_user_token",
        }),
    ))?;
    let r2_config = state.r2_config.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_r2_config",
        }),
    ))?;

    let files = state.store.files().await.map_err(store_failed)?;
    let archive_public_prefix = r2_config.public_url("").trim_end_matches('/').to_owned();
    let (groups, skipped) = group_missing_files(files, channel_id, &archive_public_prefix);
    let mut response = BackfillFilesResponse {
        ok: true,
        archived: 0,
        skipped,
        failed: 0,
    };
    tracing::info!(
        channel_id = channel_id.unwrap_or(""),
        candidate_messages = groups.len(),
        skipped = response.skipped,
        "starting file-only backfill"
    );

    for ((channel_id, message_ts), files) in groups {
        let message = match fetch_message_history(
            &state.slack_api_base_url,
            slack_user_token,
            &channel_id,
            &message_ts,
        )
        .await
        {
            Ok(Some(message)) => message,
            Ok(None) => {
                tracing::warn!(
                    channel_id,
                    message_ts,
                    missing_files = files.len(),
                    "file-only backfill could not find slack message"
                );
                response.failed += files.len();
                continue;
            }
            Err(_) => {
                tracing::warn!(
                    channel_id,
                    message_ts,
                    missing_files = files.len(),
                    "file-only backfill failed to fetch slack message"
                );
                response.failed += files.len();
                continue;
            }
        };

        for file in files {
            match archive_single_message_file(
                &state,
                slack_user_token,
                &channel_id,
                &message_ts,
                &file,
                &message,
            )
            .await
            {
                Ok(()) => response.archived += 1,
                Err(_) => {
                    tracing::warn!(
                        channel_id,
                        message_ts,
                        file_id = %file.id,
                        "file-only backfill failed to archive file"
                    );
                    response.failed += 1;
                }
            }
        }
    }

    tracing::info!(
        channel_id = channel_id.unwrap_or(""),
        archived = response.archived,
        skipped = response.skipped,
        failed = response.failed,
        "completed file-only backfill"
    );
    Ok(Json(response))
}

fn parse_backfill_files_request(
    body: &Bytes,
) -> Result<BackfillFilesRequest, (StatusCode, Json<ErrorResponse>)> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(BackfillFilesRequest::default());
    }

    serde_json::from_slice(body).map_err(|error| {
        tracing::warn!(?error, "received invalid file-backfill request body");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_backfill_request",
            }),
        )
    })
}

fn group_missing_files(
    files: Vec<File>,
    channel_id: Option<&str>,
    archive_public_prefix: &str,
) -> (BTreeMap<(String, String), Vec<File>>, usize) {
    let mut grouped = BTreeMap::<(String, String), Vec<File>>::new();
    let mut skipped = 0usize;

    for file in files {
        if channel_id.is_some_and(|value| value != file.channel_id) {
            continue;
        }
        if file
            .permalink
            .as_deref()
            .is_some_and(|value| value.starts_with(archive_public_prefix))
        {
            skipped += 1;
            continue;
        }
        grouped
            .entry((file.channel_id.clone(), file.message_ts.clone()))
            .or_default()
            .push(file);
    }

    (grouped, skipped)
}

async fn archive_single_message_file(
    state: &AppState,
    slack_user_token: &str,
    channel_id: &str,
    message_ts: &str,
    file: &File,
    message: &SlackHistoryMessage,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let Some(slack_file) = message
        .files
        .iter()
        .find(|candidate| candidate.id == file.id)
    else {
        tracing::warn!(
            channel_id,
            message_ts,
            file_id = %file.id,
            "file-only backfill could not find file in slack message payload"
        );
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "slack_file_missing",
            }),
        ));
    };
    let archived = persist_backfill_file_archives(
        state,
        slack_user_token,
        channel_id,
        &[SlackHistoryMessage {
            ts: message_ts.to_owned(),
            user: None,
            text: None,
            thread_ts: None,
            reply_count: None,
            reactions: Vec::new(),
            files: vec![slack_file.clone()],
        }],
    )
    .await?;
    if archived == 0 {
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "file_archive_failed",
            }),
        ));
    }

    Ok(())
}

#[cfg(test)]
#[path = "backfill_files_tests.rs"]
mod tests;
