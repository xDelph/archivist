use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use queue::{ProcessEventQueue, QueueError};
use serde::{Deserialize, Serialize};
use slack::{SLACK_SIGNATURE_HEADER, SLACK_TIMESTAMP_HEADER, SlackEnvelope, verify_signature};
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::trace::TraceLayer;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4001;
const DEFAULT_WORKER_BASE_URL: &str = "http://127.0.0.1:4002";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestConfig {
    pub host: String,
    pub port: u16,
    pub worker_base_url: String,
    pub qstash_base_url: Option<String>,
    pub qstash_token: Option<String>,
    pub signing_secret: Option<String>,
}

impl IngestConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("ARCHIVIST_INGEST_HOST")
                .unwrap_or_else(|_| DEFAULT_HOST.to_owned()),
            port: read_port("ARCHIVIST_INGEST_PORT", DEFAULT_PORT),
            worker_base_url: std::env::var("ARCHIVIST_WORKER_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_WORKER_BASE_URL.to_owned()),
            qstash_base_url: std::env::var("UPSTASH_QSTASH_URL").ok(),
            qstash_token: std::env::var("UPSTASH_QSTASH_TOKEN").ok(),
            signing_secret: std::env::var("SLACK_SIGNING_SECRET").ok(),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Clone)]
struct AppState {
    queue: ProcessEventQueue,
    signing_secret: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HealthResponse {
    service: &'static str,
    version: &'static str,
    queue_endpoint: String,
    queue_mode: &'static str,
    signature_verification: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct EventAck {
    ok: bool,
    enqueued: bool,
    reason: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ErrorResponse {
    error: &'static str,
}

#[derive(Debug, Deserialize)]
struct SlashCommandPayload {
    command: String,
    text: Option<String>,
    channel_id: Option<String>,
    user_id: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct SlashCommandResponse {
    ok: bool,
    command: &'static str,
    response_type: &'static str,
    text: String,
}

pub fn build_router(config: IngestConfig) -> Result<Router, QueueError> {
    let queue = ProcessEventQueue::new(
        &config.worker_base_url,
        config.qstash_base_url.as_deref(),
        config.qstash_token.as_deref(),
    )?;

    Ok(Router::new()
        .route("/health", get(health))
        .route("/api/slack/events", post(slack_events))
        .route("/api/slack/commands/{command}", post(slack_command))
        .with_state(AppState {
            queue,
            signing_secret: config.signing_secret,
        })
        .layer(TraceLayer::new_for_http()))
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "ingest",
        version: env!("CARGO_PKG_VERSION"),
        queue_endpoint: state.queue.endpoint().to_owned(),
        queue_mode: state.queue.mode().as_str(),
        signature_verification: state.signing_secret.is_some(),
    })
}

async fn slack_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    validate_signature(state.signing_secret.as_deref(), &headers, &body)?;
    let envelope: SlackEnvelope = serde_json::from_slice(&body).map_err(|_| invalid_payload())?;
    match envelope {
        SlackEnvelope::UrlVerification(challenge) => {
            tracing::info!("responding to slack url verification");
            Ok((StatusCode::OK, Json(challenge)).into_response())
        }
        SlackEnvelope::EventCallback(callback) => {
            if let Some(job) = callback.into_job(current_unix_timestamp()) {
                tracing::info!(
                    event_id = %job.event_id,
                    channel_id = %job.channel_id,
                    queue_mode = %state.queue.mode().as_str(),
                    "received slack event callback"
                );
                state
                    .queue
                    .publish_process_event(&job)
                    .await
                    .map_err(queue_failed)?;
                tracing::info!(
                    event_id = %job.event_id,
                    channel_id = %job.channel_id,
                    queue_mode = %state.queue.mode().as_str(),
                    "enqueued slack event"
                );

                Ok((
                    StatusCode::OK,
                    Json(EventAck {
                        ok: true,
                        enqueued: true,
                        reason: "queued",
                    }),
                )
                    .into_response())
            } else {
                tracing::info!("ignored unsupported or non-public slack event");
                Ok((
                    StatusCode::OK,
                    Json(EventAck {
                        ok: true,
                        enqueued: false,
                        reason: "ignored",
                    }),
                )
                    .into_response())
            }
        }
        SlackEnvelope::Unknown => {
            tracing::info!("received unknown slack envelope");
            Ok((
                StatusCode::OK,
                Json(EventAck {
                    ok: true,
                    enqueued: false,
                    reason: "unknown_event",
                }),
            )
                .into_response())
        }
    }
}

async fn slack_command(
    Path(command): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<SlashCommandResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_signature(state.signing_secret.as_deref(), &headers, &body)?;
    let command_name = slash_command_name(&command).ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "unknown_slash_command",
        }),
    ))?;
    let payload: SlashCommandPayload =
        serde_urlencoded::from_bytes(&body).map_err(|_| invalid_command_payload())?;

    if payload.command != command_name {
        return Err(invalid_command_payload());
    }

    tracing::info!(
        command = command_name,
        channel_id = payload.channel_id.as_deref().unwrap_or(""),
        user_id = payload.user_id.as_deref().unwrap_or(""),
        "received slack slash command"
    );

    Ok(Json(SlashCommandResponse {
        ok: true,
        command: command_name,
        response_type: "ephemeral",
        text: slash_command_text(command_name, &payload),
    }))
}

fn validate_signature(
    signing_secret: Option<&str>,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let Some(signing_secret) = signing_secret else {
        return Ok(());
    };

    let timestamp = header_value(headers, SLACK_TIMESTAMP_HEADER).ok_or({
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing_slack_timestamp",
            }),
        )
    })?;
    let signature = header_value(headers, SLACK_SIGNATURE_HEADER).ok_or({
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing_slack_signature",
            }),
        )
    })?;

    verify_signature(signing_secret, timestamp, body, signature).map_err(|_| {
        tracing::warn!("rejecting slack request with invalid signature");
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid_signature",
            }),
        )
    })
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn invalid_payload() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_payload",
        }),
    )
}

fn invalid_command_payload() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_command_payload",
        }),
    )
}

fn queue_failed(error: QueueError) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(?error, "failed to publish event to queue");
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorResponse {
            error: "queue_publish_failed",
        }),
    )
}

fn read_port(key: &str, fallback: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(fallback)
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

fn slash_command_name(command: &str) -> Option<&'static str> {
    match command {
        "ask-archivist" => Some("/ask-archivist"),
        "recap" => Some("/recap"),
        "save-thread" => Some("/save-thread"),
        _ => None,
    }
}

fn slash_command_text(command_name: &'static str, payload: &SlashCommandPayload) -> String {
    let subject = payload
        .text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or("this request");
    let channel_id = payload.channel_id.as_deref().unwrap_or("this channel");
    let user_id = payload.user_id.as_deref().unwrap_or("unknown user");

    match command_name {
        "/ask-archivist" => format!(
            "Stubbed {command_name} request from {user_id} in {channel_id}. Search-backed answers for {subject} are not wired yet."
        ),
        "/recap" => format!(
            "Stubbed {command_name} request from {user_id} in {channel_id}. Recap generation for {subject} is not wired yet."
        ),
        "/save-thread" => format!(
            "Stubbed {command_name} request from {user_id} in {channel_id}. Saved-thread handling for {subject} is not wired yet."
        ),
        _ => unreachable!("unsupported slash command"),
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
