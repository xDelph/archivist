use super::AppState;
use crate::storage::R2Config;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileStorageLocation {
    pub(crate) storage_key: String,
    pub(crate) public_url: String,
}

#[derive(Debug, Deserialize)]
struct SlackAuthTestResponse {
    ok: bool,
    team_id: Option<String>,
}

pub(crate) async fn resolve_storage_prefix(
    state: &AppState,
    slack_user_token: &str,
) -> Option<String> {
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

pub(crate) fn build_file_storage_location(
    r2_config: &R2Config,
    channel_id: &str,
    file_id: &str,
    filename: &str,
    prefix: Option<&str>,
) -> FileStorageLocation {
    let storage_key = storage_key(channel_id, file_id, filename, prefix);
    let public_url = r2_config.public_url(&storage_key);

    FileStorageLocation {
        storage_key,
        public_url,
    }
}

pub(crate) async fn resolve_file_storage_location(
    state: &AppState,
    slack_user_token: &str,
    channel_id: &str,
    file_id: &str,
    filename: &str,
) -> Option<FileStorageLocation> {
    let r2_config = state.r2_config.as_ref()?;
    let storage_prefix = resolve_storage_prefix(state, slack_user_token).await;
    Some(build_file_storage_location(
        r2_config,
        channel_id,
        file_id,
        filename,
        storage_prefix.as_deref(),
    ))
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

fn storage_key(channel_id: &str, file_id: &str, filename: &str, prefix: Option<&str>) -> String {
    let file_path = format!("{channel_id}/{file_id}/{filename}");

    match prefix {
        Some(prefix) if !prefix.trim().is_empty() => format!("{}/{}", prefix.trim(), file_path),
        _ => file_path,
    }
}
