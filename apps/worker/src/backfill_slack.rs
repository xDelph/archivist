use super::ErrorResponse;
use axum::{Json, http::StatusCode};
use serde::Deserialize;
use tokio::time::{Duration, sleep};

const SLACK_RATE_LIMIT_RETRIES: usize = 8;

#[derive(Debug, Deserialize)]
pub(super) struct SlackHistoryResponse {
    pub(super) ok: bool,
    pub(super) messages: Option<Vec<SlackHistoryMessage>>,
    pub(super) response_metadata: Option<SlackResponseMetadata>,
}

#[derive(Debug, Deserialize)]
struct SlackConversationListResponse {
    ok: bool,
    channels: Option<Vec<SlackConversation>>,
    response_metadata: Option<SlackResponseMetadata>,
}

#[derive(Debug, Deserialize)]
struct SlackUserListResponse<T> {
    ok: bool,
    members: Option<Vec<T>>,
    response_metadata: Option<SlackResponseMetadata>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SlackResponseMetadata {
    pub(super) next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SlackConversation {
    pub(super) id: String,
    pub(super) name: Option<String>,
    pub(super) is_archived: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SlackHistoryMessage {
    pub(super) ts: String,
    pub(super) user: Option<String>,
    pub(super) text: Option<String>,
    pub(super) thread_ts: Option<String>,
    pub(super) reply_count: Option<usize>,
    #[serde(default)]
    pub(super) reactions: Vec<SlackHistoryReaction>,
    #[serde(default)]
    pub(super) files: Vec<SlackHistoryFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SlackHistoryFile {
    pub(super) id: String,
    pub(super) name: Option<String>,
    pub(super) mimetype: Option<String>,
    pub(super) permalink: Option<String>,
    #[serde(default)]
    pub(super) url_private: Option<String>,
    #[serde(default)]
    pub(super) url_private_download: Option<String>,
    pub(super) size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SlackHistoryReaction {
    pub(super) name: String,
    #[serde(default)]
    pub(super) users: Vec<String>,
}

pub(super) async fn fetch_public_channels(
    slack_api_base_url: &str,
    slack_user_token: &str,
) -> Result<Vec<SlackConversation>, (StatusCode, Json<ErrorResponse>)> {
    let endpoint = format!(
        "{}/conversations.list",
        slack_api_base_url.trim_end_matches('/')
    );
    let client = reqwest::Client::new();
    let mut channels = Vec::new();
    let mut cursor = None::<String>;

    loop {
        let response = client
            .get(&endpoint)
            .bearer_auth(slack_user_token)
            .query(&[
                ("exclude_archived", "false"),
                ("types", "public_channel"),
                ("limit", "200"),
                ("cursor", cursor.as_deref().unwrap_or_default()),
            ])
            .send()
            .await
            .map_err(|error| {
                tracing::error!(?error, "failed to list public slack channels");
                slack_channels_failed()
            })?;
        if !response.status().is_success() {
            tracing::error!(
                status = %response.status(),
                "slack channel listing returned non-success status"
            );
            return Err(slack_channels_failed());
        }

        let payload: SlackConversationListResponse = response.json().await.map_err(|error| {
            tracing::error!(?error, "failed to decode slack channel listing");
            slack_channels_failed()
        })?;
        if !payload.ok {
            tracing::error!("slack channel listing returned ok=false");
            return Err(slack_channels_failed());
        }

        let batch = payload.channels.unwrap_or_default();
        tracing::info!(batch_size = batch.len(), "fetched public channel batch");
        channels.extend(
            batch
                .into_iter()
                .filter(|channel| !channel.is_archived.unwrap_or(false)),
        );
        cursor = payload
            .response_metadata
            .and_then(|metadata| metadata.next_cursor)
            .filter(|value| !value.trim().is_empty());
        if cursor.is_none() {
            break;
        }
    }

    channels.sort_by(|left, right| left.id.cmp(&right.id));
    channels.dedup_by(|left, right| left.id == right.id);
    Ok(channels)
}

pub(super) async fn fetch_workspace_users<T>(
    slack_api_base_url: &str,
    slack_user_token: &str,
) -> Result<Vec<T>, (StatusCode, Json<ErrorResponse>)>
where
    T: for<'de> Deserialize<'de>,
{
    let endpoint = format!("{}/users.list", slack_api_base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut users = Vec::new();
    let mut cursor = None::<String>;

    loop {
        let response = client
            .get(&endpoint)
            .bearer_auth(slack_user_token)
            .query(&[
                ("limit", "200"),
                ("cursor", cursor.as_deref().unwrap_or_default()),
            ])
            .send()
            .await
            .map_err(|error| {
                tracing::error!(?error, "failed to list slack users");
                slack_users_failed()
            })?;
        if !response.status().is_success() {
            tracing::error!(
                status = %response.status(),
                "slack user listing returned non-success status"
            );
            return Err(slack_users_failed());
        }

        let payload: SlackUserListResponse<T> = response.json().await.map_err(|error| {
            tracing::error!(?error, "failed to decode slack user listing");
            slack_users_failed()
        })?;
        if !payload.ok {
            tracing::error!("slack user listing returned ok=false");
            return Err(slack_users_failed());
        }

        let batch = payload.members.unwrap_or_default();
        tracing::info!(batch_size = batch.len(), "fetched slack user batch");
        users.extend(batch);
        cursor = payload
            .response_metadata
            .and_then(|metadata| metadata.next_cursor)
            .filter(|value| !value.trim().is_empty());
        if cursor.is_none() {
            break;
        }
    }

    Ok(users)
}

pub(super) async fn fetch_channel_history(
    slack_api_base_url: &str,
    slack_user_token: &str,
    channel_id: &str,
    cursor: Option<&str>,
    oldest_ts: Option<&str>,
    inclusive: bool,
) -> Result<SlackHistoryResponse, (StatusCode, Json<ErrorResponse>)> {
    let endpoint = format!(
        "{}/conversations.history",
        slack_api_base_url.trim_end_matches('/')
    );
    let inclusive = if inclusive { "true" } else { "false" };
    let response = reqwest::Client::new()
        .get(&endpoint)
        .bearer_auth(slack_user_token)
        .query(&[
            ("channel", channel_id),
            ("cursor", cursor.unwrap_or_default()),
            ("oldest", oldest_ts.unwrap_or_default()),
            ("inclusive", inclusive),
        ])
        .send()
        .await
        .map_err(|error| {
            tracing::error!(?error, channel_id, "failed to fetch slack history");
            slack_history_failed()
        })?;
    if !response.status().is_success() {
        tracing::error!(
            status = %response.status(),
            channel_id,
            "slack history returned non-success status"
        );
        return Err(slack_history_failed());
    }

    let history: SlackHistoryResponse = response.json().await.map_err(|error| {
        tracing::error!(?error, channel_id, "failed to decode slack history");
        slack_history_failed()
    })?;
    if !history.ok {
        tracing::error!(channel_id, "slack history returned ok=false");
        return Err(slack_history_failed());
    }

    Ok(history)
}

pub(super) async fn fetch_message_history(
    slack_api_base_url: &str,
    slack_user_token: &str,
    channel_id: &str,
    message_ts: &str,
) -> Result<Option<SlackHistoryMessage>, (StatusCode, Json<ErrorResponse>)> {
    let endpoint = format!(
        "{}/conversations.history",
        slack_api_base_url.trim_end_matches('/')
    );
    let response = reqwest::Client::new()
        .get(&endpoint)
        .bearer_auth(slack_user_token)
        .query(&[
            ("channel", channel_id),
            ("oldest", message_ts),
            ("latest", message_ts),
            ("inclusive", "true"),
            ("limit", "1"),
        ])
        .send()
        .await
        .map_err(|error| {
            tracing::error!(
                ?error,
                channel_id,
                message_ts,
                "failed to fetch slack message history"
            );
            slack_history_failed()
        })?;
    if !response.status().is_success() {
        tracing::error!(
            status = %response.status(),
            channel_id,
            message_ts,
            "slack message history returned non-success status"
        );
        return Err(slack_history_failed());
    }

    let history: SlackHistoryResponse = response.json().await.map_err(|error| {
        tracing::error!(
            ?error,
            channel_id,
            message_ts,
            "failed to decode slack message history"
        );
        slack_history_failed()
    })?;
    if !history.ok {
        tracing::error!(
            channel_id,
            message_ts,
            "slack message history returned ok=false"
        );
        return Err(slack_history_failed());
    }

    Ok(history
        .messages
        .unwrap_or_default()
        .into_iter()
        .find(|message| message.ts == message_ts))
}

pub(super) async fn fetch_thread_replies(
    slack_api_base_url: &str,
    slack_user_token: &str,
    channel_id: &str,
    root_ts: &str,
    cursor: Option<&str>,
    oldest_ts: Option<&str>,
    inclusive: bool,
) -> Result<SlackHistoryResponse, (StatusCode, Json<ErrorResponse>)> {
    let endpoint = format!(
        "{}/conversations.replies",
        slack_api_base_url.trim_end_matches('/')
    );
    let client = reqwest::Client::new();
    let inclusive = if inclusive { "true" } else { "false" };

    for attempt in 0..=SLACK_RATE_LIMIT_RETRIES {
        let response = client
            .get(&endpoint)
            .bearer_auth(slack_user_token)
            .query(&[
                ("channel", channel_id),
                ("ts", root_ts),
                ("cursor", cursor.unwrap_or_default()),
                ("oldest", oldest_ts.unwrap_or_default()),
                ("inclusive", inclusive),
            ])
            .send()
            .await
            .map_err(|error| {
                tracing::error!(
                    ?error,
                    channel_id,
                    root_ts,
                    "failed to fetch slack thread replies"
                );
                slack_history_failed()
            })?;
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1);
            tracing::warn!(
                channel_id,
                root_ts,
                attempt,
                retry_after,
                "slack thread replies rate limited, retrying after backoff"
            );
            if attempt == SLACK_RATE_LIMIT_RETRIES {
                tracing::error!(
                    channel_id,
                    root_ts,
                    retries = SLACK_RATE_LIMIT_RETRIES,
                    "slack thread replies exhausted rate-limit retries"
                );
                return Err(slack_history_failed());
            }
            sleep(Duration::from_secs(retry_after)).await;
            continue;
        }
        if !response.status().is_success() {
            tracing::error!(
                status = %response.status(),
                channel_id,
                root_ts,
                "slack thread replies returned non-success status"
            );
            return Err(slack_history_failed());
        }

        let history: SlackHistoryResponse = response.json().await.map_err(|error| {
            tracing::error!(
                ?error,
                channel_id,
                root_ts,
                "failed to decode slack thread replies"
            );
            slack_history_failed()
        })?;
        if !history.ok {
            tracing::error!(
                channel_id,
                root_ts,
                "slack thread replies returned ok=false"
            );
            return Err(slack_history_failed());
        }

        return Ok(history);
    }

    Err(slack_history_failed())
}

fn slack_history_failed() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorResponse {
            error: "slack_history_failed",
        }),
    )
}

fn slack_channels_failed() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorResponse {
            error: "slack_channels_failed",
        }),
    )
}

fn slack_users_failed() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorResponse {
            error: "slack_users_failed",
        }),
    )
}
