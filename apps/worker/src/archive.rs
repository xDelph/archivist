use super::{AppState, ErrorResponse};
use crate::storage::R2Client;
use crate::storage_paths::resolve_file_storage_location;
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct ArchiveFileRequest {
    channel_id: String,
    message_ts: String,
    file_id: String,
    filename: String,
    download_url: String,
    mimetype: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ArchiveFileResponse {
    ok: bool,
    storage_key: String,
    public_url: String,
    bytes_uploaded: usize,
}

pub(super) struct ArchiveSlackFile<'a> {
    pub(super) channel_id: &'a str,
    pub(super) message_ts: &'a str,
    pub(super) file_id: &'a str,
    pub(super) filename: &'a str,
    pub(super) download_url: &'a str,
    pub(super) mimetype: Option<&'a str>,
}

pub(super) struct ArchivedSlackFile {
    pub(super) storage_key: String,
    pub(super) public_url: String,
    pub(super) bytes_uploaded: usize,
}

pub(crate) async fn archive_file(
    State(state): State<AppState>,
    Json(request): Json<ArchiveFileRequest>,
) -> Result<Json<ArchiveFileResponse>, (StatusCode, Json<ErrorResponse>)> {
    let slack_user_token = state.slack_user_token.as_deref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_slack_user_token",
        }),
    ))?;
    let archived = archive_slack_file(
        &state,
        slack_user_token,
        ArchiveSlackFile {
            channel_id: &request.channel_id,
            message_ts: &request.message_ts,
            file_id: &request.file_id,
            filename: &request.filename,
            download_url: &request.download_url,
            mimetype: request.mimetype.as_deref(),
        },
    )
    .await?;

    Ok(Json(ArchiveFileResponse {
        ok: true,
        storage_key: archived.storage_key,
        public_url: archived.public_url,
        bytes_uploaded: archived.bytes_uploaded,
    }))
}

pub(super) async fn archive_slack_file(
    state: &AppState,
    slack_user_token: &str,
    request: ArchiveSlackFile<'_>,
) -> Result<ArchivedSlackFile, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        channel_id = %request.channel_id,
        message_ts = %request.message_ts,
        file_id = %request.file_id,
        filename = %request.filename,
        "starting file archive"
    );
    let r2_config = state.r2_config.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_r2_config",
        }),
    ))?;
    let location = resolve_file_storage_location(
        state,
        slack_user_token,
        request.channel_id,
        request.file_id,
        request.filename,
    )
    .await
    .expect("r2 config is present");
    let storage_key = location.storage_key;
    let public_url = location.public_url;

    if remote_object_exists(&public_url).await {
        tracing::info!(
            channel_id = %request.channel_id,
            file_id = %request.file_id,
            storage_key = %storage_key,
            "reusing existing archived file"
        );
        persist_archive_metadata(state, request.file_id, &storage_key, &public_url).await?;
        return Ok(ArchivedSlackFile {
            storage_key,
            public_url,
            bytes_uploaded: 0,
        });
    }

    let response = reqwest::Client::new()
        .get(request.download_url)
        .bearer_auth(slack_user_token)
        .send()
        .await
        .map_err(|error| {
            tracing::error!(
                ?error,
                channel_id = %request.channel_id,
                message_ts = %request.message_ts,
                file_id = %request.file_id,
                "failed to download file from slack"
            );
            file_download_failed()
        })?;
    if !response.status().is_success() {
        tracing::error!(
            status = %response.status(),
            channel_id = %request.channel_id,
            message_ts = %request.message_ts,
            file_id = %request.file_id,
            "slack file download returned non-success status"
        );
        return Err(file_download_failed());
    }

    let response_content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response.bytes().await.map_err(|error| {
        tracing::error!(
            ?error,
            channel_id = %request.channel_id,
            message_ts = %request.message_ts,
            file_id = %request.file_id,
            "failed to read downloaded file bytes"
        );
        file_download_failed()
    })?;
    let content_type = request
        .mimetype
        .map(str::to_owned)
        .or(response_content_type)
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let public_url = R2Client::from_config(r2_config)
        .await
        .upload(&storage_key, bytes.clone(), &content_type)
        .await
        .map_err(|error| {
            tracing::error!(
                ?error,
                channel_id = %request.channel_id,
                message_ts = %request.message_ts,
                file_id = %request.file_id,
                storage_key = %storage_key,
                "failed to upload archived file"
            );
            file_upload_failed()
        })?;

    persist_archive_metadata(state, request.file_id, &storage_key, &public_url).await?;
    tracing::info!(
        channel_id = %request.channel_id,
        message_ts = %request.message_ts,
        file_id = %request.file_id,
        storage_key = %storage_key,
        bytes_uploaded = bytes.len(),
        "completed file archive"
    );

    Ok(ArchivedSlackFile {
        storage_key,
        public_url,
        bytes_uploaded: bytes.len(),
    })
}

pub(super) async fn remote_object_exists(public_url: &str) -> bool {
    match reqwest::Client::new().head(public_url).send().await {
        Ok(response) => {
            let exists = response.status().is_success();
            if exists {
                tracing::info!(%public_url, "found existing archived object");
            }
            exists
        }
        Err(error) => {
            tracing::debug!(?error, %public_url, "failed to probe existing archived object");
            false
        }
    }
}

async fn persist_archive_metadata(
    state: &AppState,
    file_id: &str,
    storage_key: &str,
    public_url: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    state
        .store
        .set_file_archive(file_id, storage_key, public_url)
        .await
        .map_err(|error| {
            tracing::error!(?error, %file_id, %storage_key, "failed to persist archive metadata");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "store_write_failed",
                }),
            )
        })
}

fn file_download_failed() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorResponse {
            error: "file_download_failed",
        }),
    )
}

fn file_upload_failed() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorResponse {
            error: "file_upload_failed",
        }),
    )
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod tests;
