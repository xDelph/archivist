use super::{AppState, ErrorResponse};
use crate::storage::R2Client;
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

#[derive(Debug, Deserialize)]
struct SlackAuthTestResponse {
    ok: bool,
    team_id: Option<String>,
}

pub(crate) async fn archive_file(
    State(state): State<AppState>,
    Json(request): Json<ArchiveFileRequest>,
) -> Result<Json<ArchiveFileResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        channel_id = %request.channel_id,
        message_ts = %request.message_ts,
        file_id = %request.file_id,
        filename = %request.filename,
        "starting file archive"
    );
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
    let storage_prefix = resolve_storage_prefix(&state, slack_user_token)
        .await
        .or_else(|| state.r2_key_prefix.clone());
    let storage_key = storage_key(&request, storage_prefix.as_deref());
    let public_url = r2_config.public_url(&storage_key);

    if remote_object_exists(&public_url).await {
        tracing::info!(
            channel_id = %request.channel_id,
            file_id = %request.file_id,
            storage_key = %storage_key,
            "reusing existing archived file"
        );
        persist_archive_metadata(&state, &request.file_id, &storage_key, &public_url).await?;
        return Ok(Json(ArchiveFileResponse {
            ok: true,
            storage_key,
            public_url,
            bytes_uploaded: 0,
        }));
    }

    let response = reqwest::Client::new()
        .get(&request.download_url)
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

    persist_archive_metadata(&state, &request.file_id, &storage_key, &public_url).await?;
    tracing::info!(
        channel_id = %request.channel_id,
        message_ts = %request.message_ts,
        file_id = %request.file_id,
        storage_key = %storage_key,
        bytes_uploaded = bytes.len(),
        "completed file archive"
    );

    Ok(Json(ArchiveFileResponse {
        ok: true,
        storage_key,
        public_url,
        bytes_uploaded: bytes.len(),
    }))
}

async fn resolve_storage_prefix(state: &AppState, slack_user_token: &str) -> Option<String> {
    if let Some(prefix) = state
        .r2_key_prefix
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(prefix.to_owned());
    }

    match state
        .slack_workspace_prefix
        .get_or_try_init(|| async {
            fetch_workspace_prefix(&state.slack_api_base_url, slack_user_token)
                .await
                .map_err(|_| ())
        })
        .await
    {
        Ok(prefix) => prefix.clone(),
        Err(()) => None,
    }
}

async fn fetch_workspace_prefix(
    slack_api_base_url: &str,
    slack_user_token: &str,
) -> Result<Option<String>, ()> {
    let endpoint = format!("{}/auth.test", slack_api_base_url.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .get(endpoint)
        .bearer_auth(slack_user_token)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(
                ?error,
                "failed to resolve slack workspace prefix for R2 keys"
            );
        })?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "slack auth.test returned non-success status");
        return Err(());
    }

    let payload: SlackAuthTestResponse = response.json().await.map_err(|error| {
        tracing::warn!(?error, "failed to decode slack auth.test response");
    })?;
    if !payload.ok {
        tracing::warn!("slack auth.test returned ok=false");
        return Err(());
    }

    Ok(payload
        .team_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned))
}

async fn remote_object_exists(public_url: &str) -> bool {
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

fn storage_key(request: &ArchiveFileRequest, prefix: Option<&str>) -> String {
    let file_path = format!(
        "{}/{}/{}",
        request.channel_id, request.file_id, request.filename
    );

    match prefix {
        Some(prefix) if !prefix.trim().is_empty() => format!("{}/{}", prefix.trim(), file_path),
        _ => file_path,
    }
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
