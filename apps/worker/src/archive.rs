use super::{AppState, ErrorResponse};
use crate::storage::R2Client;
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct ArchiveFileRequest {
    team_id: String,
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

pub(crate) async fn archive_file(
    State(state): State<AppState>,
    Json(request): Json<ArchiveFileRequest>,
) -> Result<Json<ArchiveFileResponse>, (StatusCode, Json<ErrorResponse>)> {
    let slack_bot_token = state.slack_bot_token.as_deref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_slack_bot_token",
        }),
    ))?;
    let r2_config = state.r2_config.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_r2_config",
        }),
    ))?;
    let response = reqwest::Client::new()
        .get(&request.download_url)
        .bearer_auth(slack_bot_token)
        .send()
        .await
        .map_err(|_| file_download_failed())?;
    if !response.status().is_success() {
        return Err(file_download_failed());
    }

    let response_content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response.bytes().await.map_err(|_| file_download_failed())?;
    let storage_key = storage_key(&request);
    let content_type = request
        .mimetype
        .or(response_content_type)
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let public_url = R2Client::from_config(r2_config)
        .await
        .upload(&storage_key, bytes.clone(), &content_type)
        .await
        .map_err(|_| file_upload_failed())?;

    Ok(Json(ArchiveFileResponse {
        ok: true,
        storage_key,
        public_url,
        bytes_uploaded: bytes.len(),
    }))
}

fn storage_key(request: &ArchiveFileRequest) -> String {
    format!(
        "{}/{}/{}/{}/{}",
        request.team_id,
        request.channel_id,
        request.message_ts,
        request.file_id,
        sanitize_filename(&request.filename)
    )
}

fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '\0' => '_',
            _ => character,
        })
        .collect()
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
