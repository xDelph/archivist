use super::{AppState, ErrorResponse, store_failed};
use axum::{Json, extract::State, http::StatusCode};
use db::StoreOutcome;
use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub(crate) struct BackfillChannelRequest {
    team_id: String,
    channel_id: String,
    cursor: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct BackfillChannelResponse {
    ok: bool,
    inserted: usize,
    duplicate: usize,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackHistoryResponse {
    ok: bool,
    messages: Option<Vec<SlackHistoryMessage>>,
    response_metadata: Option<SlackResponseMetadata>,
}

#[derive(Debug, Deserialize)]
struct SlackResponseMetadata {
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackHistoryMessage {
    ts: String,
    user: Option<String>,
    text: Option<String>,
    thread_ts: Option<String>,
    #[serde(default)]
    files: Vec<SlackHistoryFile>,
}

#[derive(Debug, Deserialize)]
struct SlackHistoryFile {
    id: String,
    name: String,
    mimetype: Option<String>,
    permalink: Option<String>,
    size: Option<u64>,
}

pub(crate) async fn backfill_channel(
    State(state): State<AppState>,
    Json(request): Json<BackfillChannelRequest>,
) -> Result<Json<BackfillChannelResponse>, (StatusCode, Json<ErrorResponse>)> {
    let slack_bot_token = state.slack_bot_token.as_deref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_slack_bot_token",
        }),
    ))?;
    let history = fetch_channel_history(
        &state.slack_api_base_url,
        slack_bot_token,
        &request.channel_id,
        request.cursor.as_deref(),
    )
    .await?;
    let mut inserted = 0usize;
    let mut duplicate = 0usize;

    for message in history.messages.unwrap_or_default() {
        let outcome = state
            .store
            .record_process_event(&ProcessEventJob {
                event_id: format!("backfill:{}:{}", request.channel_id, message.ts),
                team_id: request.team_id.clone(),
                event_time: parse_event_time(&message.ts),
                received_at: current_unix_timestamp(),
                channel_id: request.channel_id.clone(),
                channel_kind: ChannelKind::from_channel_id(&request.channel_id),
                payload: EventPayload::Message {
                    user_id: message.user,
                    text: message.text,
                    ts: message.ts.clone(),
                    thread_ts: message
                        .thread_ts
                        .filter(|thread_ts| thread_ts != &message.ts),
                    files: message
                        .files
                        .into_iter()
                        .map(|file| SharedFile {
                            id: file.id,
                            name: file.name,
                            mimetype: file.mimetype,
                            permalink: file.permalink,
                            size: file.size,
                        })
                        .collect(),
                },
            })
            .await
            .map_err(store_failed)?;

        match outcome {
            StoreOutcome::Inserted => inserted += 1,
            StoreOutcome::Duplicate => duplicate += 1,
        }
    }

    Ok(Json(BackfillChannelResponse {
        ok: true,
        inserted,
        duplicate,
        next_cursor: history
            .response_metadata
            .and_then(|metadata| metadata.next_cursor)
            .filter(|cursor| !cursor.trim().is_empty()),
    }))
}

async fn fetch_channel_history(
    slack_api_base_url: &str,
    slack_bot_token: &str,
    channel_id: &str,
    cursor: Option<&str>,
) -> Result<SlackHistoryResponse, (StatusCode, Json<ErrorResponse>)> {
    let endpoint = format!(
        "{}/conversations.history",
        slack_api_base_url.trim_end_matches('/')
    );
    let response = reqwest::Client::new()
        .get(&endpoint)
        .bearer_auth(slack_bot_token)
        .query(&[
            ("channel", channel_id),
            ("cursor", cursor.unwrap_or_default()),
        ])
        .send()
        .await
        .map_err(|_| slack_history_failed())?;
    if !response.status().is_success() {
        return Err(slack_history_failed());
    }

    let history: SlackHistoryResponse =
        response.json().await.map_err(|_| slack_history_failed())?;
    if !history.ok {
        return Err(slack_history_failed());
    }

    Ok(history)
}

fn parse_event_time(ts: &str) -> i64 {
    ts.split('.')
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

fn slack_history_failed() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorResponse {
            error: "slack_history_failed",
        }),
    )
}

#[cfg(test)]
#[path = "backfill_tests.rs"]
mod tests;
