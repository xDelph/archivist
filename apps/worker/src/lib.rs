mod ai;
mod ai_openrouter;
mod ai_openrouter_language;
mod archive;
mod backfill;
mod backfill_archive;
mod backfill_files;
mod backfill_range;
mod backfill_slack;
mod backfill_threads;
mod storage;
mod storage_paths;
mod summaries;

use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use db::{EventStore, StoreError, StoreOutcome};
use domain::ProcessEventJob;
use queue::{
    QueueError, SignatureError, UPSTASH_SIGNATURE_HEADER, build_heartbeat_endpoint,
    build_process_event_endpoint, build_refresh_thread_summaries_endpoint, verify_qstash_signature,
};
use serde::Serialize;
use tokio::sync::OnceCell;
use tower_http::trace::TraceLayer;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4002;
const DEFAULT_EVENT_LOG_PATH: &str = "logs/process-events.jsonl";
const DEFAULT_WORKER_BASE_URL: &str = "http://127.0.0.1:4002";
const DEFAULT_SLACK_API_BASE_URL: &str = "https://slack.com/api";
const DEFAULT_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerConfig {
    pub host: String,
    pub port: u16,
    pub event_log_path: String,
    pub worker_base_url: String,
    pub slack_api_base_url: String,
    pub openrouter_base_url: String,
    pub openrouter_api_key: Option<String>,
    pub openrouter_model: Option<String>,
    pub slack_user_token: Option<String>,
    pub r2_account_id: Option<String>,
    pub r2_access_key_id: Option<String>,
    pub r2_secret_access_key: Option<String>,
    pub r2_bucket: Option<String>,
    pub r2_public_url: Option<String>,
    pub r2_endpoint_url: Option<String>,
    pub r2_key_prefix: Option<String>,
    pub current_signing_key: Option<String>,
    pub next_signing_key: Option<String>,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("ARKIVIST_WORKER_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_owned()),
            port: read_port("ARKIVIST_WORKER_PORT", DEFAULT_PORT),
            event_log_path: std::env::var("ARKIVIST_EVENT_LOG_PATH")
                .unwrap_or_else(|_| DEFAULT_EVENT_LOG_PATH.to_owned()),
            worker_base_url: std::env::var("ARKIVIST_WORKER_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_WORKER_BASE_URL.to_owned()),
            slack_api_base_url: std::env::var("SLACK_API_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_SLACK_API_BASE_URL.to_owned()),
            openrouter_base_url: std::env::var("OPENROUTER_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_OPENROUTER_BASE_URL.to_owned()),
            openrouter_api_key: std::env::var("OPENROUTER_API_KEY").ok(),
            openrouter_model: std::env::var("OPENROUTER_MODEL").ok(),
            slack_user_token: std::env::var("SLACK_USER_TOKEN").ok(),
            r2_account_id: env_var_any(&["CLOUDFLARE_R2_ACCOUNT_ID", "CLOUDFLARED_R2_ACCOUNT_ID"]),
            r2_access_key_id: env_var_any(&[
                "CLOUDFLARE_R2_ACCESS_KEY_ID",
                "CLOUDFLARE_R2_ACCESS_KEY",
                "CLOUDFLARED_R2_ACCESS_KEY_ID",
                "CLOUDFLARED_R2_ACCESS_KEY",
            ]),
            r2_secret_access_key: env_var_any(&[
                "CLOUDFLARE_R2_SECRET_ACCESS_KEY",
                "CLOUDFLARE_R2_SECRET_KEY",
                "CLOUDFLARED_R2_SECRET_ACCESS_KEY",
                "CLOUDFLARED_R2_SECRET_KEY",
            ]),
            r2_bucket: env_var_any(&["CLOUDFLARE_R2_BUCKET", "CLOUDFLARED_R2_BUCKET"]),
            r2_public_url: env_var_any(&["CLOUDFLARE_R2_PUBLIC_URL", "CLOUDFLARED_R2_PUBLIC_URL"]),
            r2_endpoint_url: env_var_any(&[
                "CLOUDFLARE_R2_ENDPOINT_URL",
                "CLOUDFLARED_R2_ENDPOINT_URL",
            ]),
            r2_key_prefix: std::env::var("ARKIVIST_R2_KEY_PREFIX").ok(),
            current_signing_key: std::env::var("UPSTASH_QSTASH_CURRENT_SIGNING_KEY").ok(),
            next_signing_key: std::env::var("UPSTASH_QSTASH_NEXT_SIGNING_KEY").ok(),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn env_var_any(names: &[&str]) -> Option<String> {
    env_var_any_with(names, |name| std::env::var(name).ok())
}

fn env_var_any_with<F>(names: &[&str], mut get: F) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    names.iter().find_map(|name| {
        get(name)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

#[derive(Clone)]
struct AppState {
    store: EventStore,
    event_log_path: String,
    process_event_url: String,
    heartbeat_url: String,
    refresh_thread_summaries_url: String,
    queue_signature_verification: bool,
    slack_api_base_url: String,
    openrouter_config: ai::OpenRouterConfig,
    slack_user_token: Option<String>,
    r2_config: Option<storage::R2Config>,
    r2_key_prefix: Option<String>,
    slack_workspace_prefix: std::sync::Arc<OnceCell<Option<String>>>,
    current_signing_key: Option<String>,
    next_signing_key: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HealthResponse {
    service: &'static str,
    version: &'static str,
    repository_mode: &'static str,
    event_log_path: String,
    queue_signature_verification: bool,
    tracked_events: usize,
    tracked_messages: usize,
    tracked_reactions: usize,
    tracked_files: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ProcessEventResponse {
    ok: bool,
    duplicate: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ErrorResponse {
    error: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HeartbeatResponse {
    ok: bool,
    job: &'static str,
}

pub fn build_router(
    store: impl Into<EventStore>,
    config: WorkerConfig,
) -> Result<Router, QueueError> {
    let process_event_url = build_process_event_endpoint(&config.worker_base_url)?;
    let heartbeat_url = build_heartbeat_endpoint(&config.worker_base_url)?;
    let refresh_thread_summaries_url =
        build_refresh_thread_summaries_endpoint(&config.worker_base_url)?;
    let queue_signature_verification = signature_verification_enabled(
        &config.worker_base_url,
        config.current_signing_key.as_deref(),
        config.next_signing_key.as_deref(),
    );

    Ok(Router::new()
        .route("/health", get(health))
        .route("/jobs/process_event", post(process_event))
        .route("/jobs/heartbeat", post(heartbeat))
        .route(
            "/jobs/refresh_thread_summaries",
            post(refresh_thread_summaries),
        )
        .route(
            "/jobs/generate_thread_summaries",
            post(ai::generate_thread_summaries),
        )
        .route("/jobs/backfill_channel", post(backfill::backfill_channel))
        .route("/jobs/backfill_files", post(backfill_files::backfill_files))
        .route("/jobs/archive_file", post(archive::archive_file))
        .with_state(AppState {
            store: store.into(),
            event_log_path: config.event_log_path,
            process_event_url,
            heartbeat_url,
            refresh_thread_summaries_url,
            queue_signature_verification,
            slack_api_base_url: config.slack_api_base_url,
            openrouter_config: ai::OpenRouterConfig {
                api_base_url: config.openrouter_base_url,
                api_key: config.openrouter_api_key,
                model: config.openrouter_model,
            },
            slack_user_token: config.slack_user_token,
            r2_config: storage::R2Config::from_options(
                config.r2_account_id,
                config.r2_access_key_id,
                config.r2_secret_access_key,
                config.r2_bucket,
                config.r2_public_url,
                config.r2_endpoint_url,
            ),
            r2_key_prefix: config
                .r2_key_prefix
                .and_then(|value| (!value.trim().is_empty()).then_some(value)),
            slack_workspace_prefix: std::sync::Arc::new(OnceCell::new()),
            current_signing_key: config.current_signing_key,
            next_signing_key: config.next_signing_key,
        })
        .layer(TraceLayer::new_for_http()))
}

async fn health(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repository_health = state.store.health().await.map_err(store_failed)?;

    Ok(Json(HealthResponse {
        service: "worker",
        version: env!("CARGO_PKG_VERSION"),
        repository_mode: state.store.mode().as_str(),
        event_log_path: state.event_log_path,
        queue_signature_verification: state.queue_signature_verification,
        tracked_events: repository_health.tracked_events,
        tracked_messages: repository_health.tracked_messages,
        tracked_reactions: repository_health.tracked_reactions,
        tracked_files: repository_health.tracked_files,
    }))
}

async fn process_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ProcessEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_qstash_delivery(
        state.queue_signature_verification,
        &headers,
        &body,
        &state.process_event_url,
        state.current_signing_key.as_deref(),
        state.next_signing_key.as_deref(),
    )?;

    let job: ProcessEventJob = serde_json::from_slice(&body).map_err(|_| invalid_payload())?;
    tracing::info!(
        event_id = %job.event_id,
        channel_id = %job.channel_id,
        event_kind = payload_kind(&job),
        "processing worker event"
    );
    let outcome = state
        .store
        .record_process_event(&job)
        .await
        .map_err(store_failed)?;
    let duplicate = matches!(outcome, StoreOutcome::Duplicate);
    tracing::info!(
        event_id = %job.event_id,
        channel_id = %job.channel_id,
        duplicate,
        "processed worker event"
    );

    Ok(Json(ProcessEventResponse {
        ok: true,
        duplicate,
    }))
}

async fn heartbeat(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<HeartbeatResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_qstash_delivery(
        state.queue_signature_verification,
        &headers,
        &body,
        &state.heartbeat_url,
        state.current_signing_key.as_deref(),
        state.next_signing_key.as_deref(),
    )?;
    tracing::debug!("worker heartbeat accepted");

    Ok(Json(HeartbeatResponse {
        ok: true,
        job: "heartbeat",
    }))
}

async fn refresh_thread_summaries(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<summaries::RefreshThreadSummariesResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_qstash_delivery(
        state.queue_signature_verification,
        &headers,
        &body,
        &state.refresh_thread_summaries_url,
        state.current_signing_key.as_deref(),
        state.next_signing_key.as_deref(),
    )?;

    summaries::refresh_thread_summaries(State(state), body).await
}

fn store_failed(error: StoreError) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(?error, "worker store request failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "store_write_failed",
        }),
    )
}

fn validate_qstash_delivery(
    queue_signature_verification: bool,
    headers: &HeaderMap,
    body: &[u8],
    expected_url: &str,
    current_signing_key: Option<&str>,
    next_signing_key: Option<&str>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if !queue_signature_verification {
        return Ok(());
    }

    let signature = header_value(headers, UPSTASH_SIGNATURE_HEADER);
    verify_qstash_signature(
        signature,
        body,
        expected_url,
        current_signing_key,
        next_signing_key,
    )
    .map_err(qstash_signature_failed)
}

fn signature_verification_enabled(
    worker_base_url: &str,
    current_signing_key: Option<&str>,
    next_signing_key: Option<&str>,
) -> bool {
    signing_keys_present(current_signing_key, next_signing_key)
        && !is_loopback_worker_base_url(worker_base_url)
}

fn signing_keys_present(current_signing_key: Option<&str>, next_signing_key: Option<&str>) -> bool {
    [current_signing_key, next_signing_key]
        .into_iter()
        .flatten()
        .any(|value| !value.trim().is_empty())
}

fn is_loopback_worker_base_url(worker_base_url: &str) -> bool {
    reqwest::Url::parse(worker_base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .is_some_and(|host| matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1"))
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

fn qstash_signature_failed(error: SignatureError) -> (StatusCode, Json<ErrorResponse>) {
    tracing::warn!(?error, "worker rejected qstash delivery");
    let error = match error {
        SignatureError::MissingSignatureHeader => "missing_qstash_signature",
        _ => "invalid_qstash_signature",
    };

    (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error }))
}

fn read_port(key: &str, fallback: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(fallback)
}

fn payload_kind(job: &ProcessEventJob) -> &'static str {
    match &job.payload {
        domain::EventPayload::Message { .. } => "message",
        domain::EventPayload::ReactionAdded { .. } => "reaction_added",
        domain::EventPayload::ChannelUpdated { .. } => "channel_updated",
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
