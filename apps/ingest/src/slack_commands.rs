use crate::{AppState, SlashCommandPayload, SlashCommandResponse};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const LIST_HIGHLIGHTS_COMMAND: &str = "/list-highlights";
const PIN_HIGHLIGHT_COMMAND: &str = "/pin-highlight";
const UNPIN_HIGHLIGHT_COMMAND: &str = "/unpin-highlight";
const SLACK_COMMAND_API_TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Debug, Serialize)]
struct PinHighlightRequest {
    slack_user_id: String,
    thread_id: String,
}

#[derive(Debug, Serialize)]
struct SlackHighlightUserRequest {
    slack_user_id: String,
}

#[derive(Debug, Deserialize)]
struct PinHighlightApiResponse {
    item: PinHighlightResponseItem,
}

#[derive(Debug, Deserialize)]
struct PinHighlightResponseItem {
    thread_id: String,
}

#[derive(Debug, Deserialize)]
struct ListHighlightsApiResponse {
    items: Vec<ListHighlightsResponseItem>,
}

#[derive(Debug, Deserialize)]
struct ListHighlightsResponseItem {
    thread_id: String,
    channel_name: Option<String>,
    title: String,
}

#[derive(Debug, Deserialize)]
struct DeleteHighlightApiResponse {
    ok: bool,
}

#[derive(Debug, Deserialize)]
struct HighlightApiError {
    error: String,
}

pub(super) async fn list_highlights_command_response(
    state: &AppState,
    payload: &SlashCommandPayload,
) -> SlashCommandResponse {
    let Some(user_id) = payload_user_id(payload) else {
        return slash_command_error(
            LIST_HIGHLIGHTS_COMMAND,
            "Missing Slack user id in the command payload.",
        );
    };
    let Some(token) = command_token(state) else {
        return slash_command_error(
            LIST_HIGHLIGHTS_COMMAND,
            "Slack highlight listing is not configured on the ingest service.",
        );
    };
    let response = command_client()
        .post(command_endpoint(
            state,
            "/api/internal/slack/highlights/list",
        ))
        .bearer_auth(token)
        .json(&SlackHighlightUserRequest {
            slack_user_id: user_id.to_owned(),
        })
        .send()
        .await;

    let Ok(response) = response else {
        return slash_command_error(
            LIST_HIGHLIGHTS_COMMAND,
            "Highlight listing failed because the API is unreachable.",
        );
    };
    let status = response.status();
    if status.is_success() {
        return match response.json::<ListHighlightsApiResponse>().await {
            Ok(payload) => render_list_highlights_response(payload),
            Err(_) => slash_command_error(
                LIST_HIGHLIGHTS_COMMAND,
                "Highlight listing succeeded but the API returned an invalid response.",
            ),
        };
    }

    render_highlight_api_error(
        LIST_HIGHLIGHTS_COMMAND,
        response,
        "Highlight listing failed.",
    )
    .await
}

pub(super) async fn pin_highlight_command_response(
    state: &AppState,
    payload: &SlashCommandPayload,
) -> SlashCommandResponse {
    let Some(user_id) = payload_user_id(payload) else {
        return slash_command_error(
            PIN_HIGHLIGHT_COMMAND,
            "Missing Slack user id in the command payload.",
        );
    };
    let Some(thread_id) = payload
        .text
        .as_deref()
        .and_then(parse_highlight_thread_target)
    else {
        return slash_command_error(
            PIN_HIGHLIGHT_COMMAND,
            "Usage: /pin-highlight <thread permalink or channel_id:root_ts>.",
        );
    };
    let Some(token) = command_token(state) else {
        return slash_command_error(
            PIN_HIGHLIGHT_COMMAND,
            "Slack highlight pinning is not configured on the ingest service.",
        );
    };
    let response = command_client()
        .post(command_endpoint(state, "/api/internal/slack/highlights"))
        .bearer_auth(token)
        .json(&PinHighlightRequest {
            slack_user_id: user_id.to_owned(),
            thread_id,
        })
        .send()
        .await;

    let Ok(response) = response else {
        return slash_command_error(
            PIN_HIGHLIGHT_COMMAND,
            "Highlight pinning failed because the API is unreachable.",
        );
    };
    let status = response.status();
    if status.is_success() {
        return match response.json::<PinHighlightApiResponse>().await {
            Ok(payload) => SlashCommandResponse {
                ok: true,
                command: PIN_HIGHLIGHT_COMMAND,
                response_type: "ephemeral",
                text: format!(
                    "Pinned {} as a highlighted thread in the app.",
                    payload.item.thread_id
                ),
            },
            Err(_) => slash_command_error(
                PIN_HIGHLIGHT_COMMAND,
                "Highlight pinning succeeded but the API returned an invalid response.",
            ),
        };
    }

    render_highlight_api_error(PIN_HIGHLIGHT_COMMAND, response, "Highlight pinning failed.").await
}

pub(super) async fn unpin_highlight_command_response(
    state: &AppState,
    payload: &SlashCommandPayload,
) -> SlashCommandResponse {
    let Some(user_id) = payload_user_id(payload) else {
        return slash_command_error(
            UNPIN_HIGHLIGHT_COMMAND,
            "Missing Slack user id in the command payload.",
        );
    };
    let Some(thread_id) = payload
        .text
        .as_deref()
        .and_then(parse_highlight_thread_target)
    else {
        return slash_command_error(
            UNPIN_HIGHLIGHT_COMMAND,
            "Usage: /unpin-highlight <thread permalink or highlight id>.",
        );
    };
    let Some(token) = command_token(state) else {
        return slash_command_error(
            UNPIN_HIGHLIGHT_COMMAND,
            "Slack highlight unpinning is not configured on the ingest service.",
        );
    };
    let response = command_client()
        .post(command_endpoint(
            state,
            "/api/internal/slack/highlights/unpin",
        ))
        .bearer_auth(token)
        .json(&PinHighlightRequest {
            slack_user_id: user_id.to_owned(),
            thread_id: thread_id.clone(),
        })
        .send()
        .await;

    let Ok(response) = response else {
        return slash_command_error(
            UNPIN_HIGHLIGHT_COMMAND,
            "Highlight unpinning failed because the API is unreachable.",
        );
    };
    let status = response.status();
    if status.is_success() {
        return match response.json::<DeleteHighlightApiResponse>().await {
            Ok(payload) if payload.ok => SlashCommandResponse {
                ok: true,
                command: UNPIN_HIGHLIGHT_COMMAND,
                response_type: "ephemeral",
                text: format!("Unpinned {} from app highlights.", thread_id),
            },
            _ => slash_command_error(
                UNPIN_HIGHLIGHT_COMMAND,
                "Highlight unpinning succeeded but the API returned an invalid response.",
            ),
        };
    }

    render_highlight_api_error(
        UNPIN_HIGHLIGHT_COMMAND,
        response,
        "Highlight unpinning failed.",
    )
    .await
}

fn slash_command_error(command: &'static str, message: &str) -> SlashCommandResponse {
    SlashCommandResponse {
        ok: false,
        command,
        response_type: "ephemeral",
        text: message.to_owned(),
    }
}

fn payload_user_id(payload: &SlashCommandPayload) -> Option<&str> {
    payload
        .user_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn command_token(state: &AppState) -> Option<&str> {
    state.slack_command_token.as_deref()
}

fn command_endpoint(state: &AppState, path: &str) -> String {
    format!("{}{}", state.api_base_url.trim_end_matches('/'), path)
}

fn command_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(SLACK_COMMAND_API_TIMEOUT)
        .build()
        .expect("slack command client should build")
}

fn render_list_highlights_response(payload: ListHighlightsApiResponse) -> SlashCommandResponse {
    if payload.items.is_empty() {
        return SlashCommandResponse {
            ok: true,
            command: LIST_HIGHLIGHTS_COMMAND,
            response_type: "ephemeral",
            text: "No highlighted threads are pinned right now.".to_owned(),
        };
    }

    let lines = payload
        .items
        .into_iter()
        .map(|item| {
            let channel = item.channel_name.unwrap_or_else(|| {
                item.thread_id
                    .split(':')
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            });
            format!("• `{}` in #{} — {}", item.thread_id, channel, item.title)
        })
        .collect::<Vec<_>>();

    SlashCommandResponse {
        ok: true,
        command: LIST_HIGHLIGHTS_COMMAND,
        response_type: "ephemeral",
        text: format!("Highlighted threads:\n{}", lines.join("\n")),
    }
}

async fn render_highlight_api_error(
    command: &'static str,
    response: reqwest::Response,
    fallback: &str,
) -> SlashCommandResponse {
    let status = response.status();
    let error = response
        .json::<HighlightApiError>()
        .await
        .map(|payload| payload.error)
        .unwrap_or_default();
    slash_command_error(
        command,
        &map_highlight_command_error(status.as_u16(), &error, fallback),
    )
}

fn map_highlight_command_error(status: u16, error: &str, fallback: &str) -> String {
    match (status, error) {
        (403, "admin_required") => "Only admin users can manage highlighted threads.".to_owned(),
        (404, "thread_not_found") => {
            "The thread could not be found in Archivist yet. Try again once it has been ingested."
                .to_owned()
        }
        (404, "highlight_not_found") => "That thread is not currently highlighted.".to_owned(),
        (401, "invalid_slack_command_token") => {
            "Slack highlight management is misconfigured between ingest and api.".to_owned()
        }
        (503, "slack_command_auth_unavailable") => {
            "Slack highlight management is not configured on the api service.".to_owned()
        }
        _ => fallback.to_owned(),
    }
}

pub(super) fn parse_highlight_thread_target(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    let value = normalize_slack_link(value);
    if let Some((channel_id, root_ts)) = parse_thread_id_literal(value) {
        return Some(format!("{channel_id}:{root_ts}"));
    }
    parse_slack_thread_permalink(value)
}

fn normalize_slack_link(raw: &str) -> &str {
    let raw = raw.trim().trim_matches(['<', '>']);
    raw.split_once('|').map_or(raw, |(url, _)| url)
}

fn parse_thread_id_literal(raw: &str) -> Option<(&str, &str)> {
    let (channel_id, root_ts) = raw.split_once(':')?;
    if channel_id.is_empty()
        || root_ts.is_empty()
        || channel_id.contains('/')
        || !root_ts.contains('.')
        || !root_ts
            .chars()
            .all(|char| char.is_ascii_digit() || char == '.')
    {
        return None;
    }
    Some((channel_id, root_ts))
}

fn parse_slack_thread_permalink(raw: &str) -> Option<String> {
    let url = reqwest::Url::parse(raw).ok()?;
    let channel_id_from_path = url
        .path_segments()?
        .collect::<Vec<_>>()
        .windows(3)
        .find_map(|segment| match segment {
            ["archives", channel_id, _] if !channel_id.is_empty() => Some((*channel_id).to_owned()),
            _ => None,
        })?;
    let query = url.query_pairs().collect::<Vec<_>>();
    let channel_id = query
        .iter()
        .find(|(key, _)| key == "cid")
        .map(|(_, value)| value.to_string())
        .unwrap_or(channel_id_from_path);
    if let Some(thread_ts) = query
        .iter()
        .find(|(key, _)| key == "thread_ts")
        .map(|(_, value)| value.to_string())
    {
        return Some(format!("{channel_id}:{thread_ts}"));
    }
    let message_ts = url
        .path_segments()?
        .next_back()
        .and_then(slack_permalink_message_ts);
    Some(format!("{channel_id}:{}", message_ts?))
}

fn slack_permalink_message_ts(raw: &str) -> Option<String> {
    let digits = raw.strip_prefix('p')?;
    if digits.len() <= 6 || !digits.chars().all(|char| char.is_ascii_digit()) {
        return None;
    }
    let (seconds, micros) = digits.split_at(digits.len() - 6);
    Some(format!("{seconds}.{micros}"))
}
