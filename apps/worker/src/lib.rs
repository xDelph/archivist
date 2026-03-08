use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use db::{JsonlEventStore, RepositoryMode, StoreError, StoreOutcome};
use domain::ProcessEventJob;
use queue::{
    QueueError, SignatureError, UPSTASH_SIGNATURE_HEADER, build_heartbeat_endpoint,
    build_process_event_endpoint, verify_qstash_signature,
};
use serde::Serialize;
use tower_http::trace::TraceLayer;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4002;
const DEFAULT_EVENT_LOG_PATH: &str = "logs/process-events.jsonl";
const DEFAULT_WORKER_BASE_URL: &str = "http://127.0.0.1:4002";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerConfig {
    pub host: String,
    pub port: u16,
    pub event_log_path: String,
    pub worker_base_url: String,
    pub current_signing_key: Option<String>,
    pub next_signing_key: Option<String>,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("ARCHIVIST_WORKER_HOST")
                .unwrap_or_else(|_| DEFAULT_HOST.to_owned()),
            port: read_port("ARCHIVIST_WORKER_PORT", DEFAULT_PORT),
            event_log_path: std::env::var("ARCHIVIST_EVENT_LOG_PATH")
                .unwrap_or_else(|_| DEFAULT_EVENT_LOG_PATH.to_owned()),
            worker_base_url: std::env::var("ARCHIVIST_WORKER_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_WORKER_BASE_URL.to_owned()),
            current_signing_key: std::env::var("UPSTASH_QSTASH_CURRENT_SIGNING_KEY").ok(),
            next_signing_key: std::env::var("UPSTASH_QSTASH_NEXT_SIGNING_KEY").ok(),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Clone)]
struct AppState {
    store: JsonlEventStore,
    event_log_path: String,
    process_event_url: String,
    heartbeat_url: String,
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

pub fn build_router(store: JsonlEventStore, config: WorkerConfig) -> Result<Router, QueueError> {
    let process_event_url = build_process_event_endpoint(&config.worker_base_url)?;
    let heartbeat_url = build_heartbeat_endpoint(&config.worker_base_url)?;

    Ok(Router::new()
        .route("/health", get(health))
        .route("/jobs/process_event", post(process_event))
        .route("/jobs/heartbeat", post(heartbeat))
        .with_state(AppState {
            store,
            event_log_path: config.event_log_path,
            process_event_url,
            heartbeat_url,
            current_signing_key: config.current_signing_key,
            next_signing_key: config.next_signing_key,
        })
        .layer(TraceLayer::new_for_http()))
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let repository_health = state.store.health().await;

    Json(HealthResponse {
        service: "worker",
        version: env!("CARGO_PKG_VERSION"),
        repository_mode: RepositoryMode::LocalJsonlMock.as_str(),
        event_log_path: state.event_log_path,
        queue_signature_verification: signature_verification_enabled(
            state.current_signing_key.as_deref(),
            state.next_signing_key.as_deref(),
        ),
        tracked_events: repository_health.tracked_events,
        tracked_messages: repository_health.tracked_messages,
        tracked_reactions: repository_health.tracked_reactions,
    })
}

async fn process_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ProcessEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_qstash_delivery(
        &headers,
        &body,
        &state.process_event_url,
        state.current_signing_key.as_deref(),
        state.next_signing_key.as_deref(),
    )?;

    let job: ProcessEventJob = serde_json::from_slice(&body).map_err(|_| invalid_payload())?;
    let outcome = state
        .store
        .record_process_event(&job)
        .await
        .map_err(store_failed)?;
    let duplicate = matches!(outcome, StoreOutcome::Duplicate);

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
        &headers,
        &body,
        &state.heartbeat_url,
        state.current_signing_key.as_deref(),
        state.next_signing_key.as_deref(),
    )?;

    Ok(Json(HeartbeatResponse {
        ok: true,
        job: "heartbeat",
    }))
}

fn store_failed(_: StoreError) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "store_write_failed",
        }),
    )
}

fn validate_qstash_delivery(
    headers: &HeaderMap,
    body: &[u8],
    expected_url: &str,
    current_signing_key: Option<&str>,
    next_signing_key: Option<&str>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if !signature_verification_enabled(current_signing_key, next_signing_key) {
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
    current_signing_key: Option<&str>,
    next_signing_key: Option<&str>,
) -> bool {
    [current_signing_key, next_signing_key]
        .into_iter()
        .flatten()
        .any(|value| !value.trim().is_empty())
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

#[cfg(test)]
mod tests {
    use super::{WorkerConfig, build_router};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use db::JsonlEventStore;
    use domain::{ChannelKind, EventPayload, ProcessEventJob};
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    #[test]
    fn config_uses_local_defaults() {
        let config = WorkerConfig::from_env();
        assert_eq!(config.bind_address(), "127.0.0.1:4002");
        assert_eq!(config.event_log_path, "logs/process-events.jsonl");
        assert_eq!(config.worker_base_url, "http://127.0.0.1:4002");
        assert_eq!(config.current_signing_key, None);
        assert_eq!(config.next_signing_key, None);
    }

    #[tokio::test]
    async fn duplicate_events_are_acknowledged() {
        let tempdir = tempdir().expect("tempdir");
        let log_path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&log_path).await.expect("store");
        let router = build_router(
            store,
            WorkerConfig {
                host: "127.0.0.1".to_owned(),
                port: 4002,
                event_log_path: log_path.display().to_string(),
                worker_base_url: "http://127.0.0.1:4002".to_owned(),
                current_signing_key: None,
                next_signing_key: None,
            },
        )
        .expect("router");
        let payload = ProcessEventJob {
            event_id: "evt_1".to_owned(),
            team_id: "team_1".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("hello".to_owned()),
                ts: "1700000000.000001".to_owned(),
                thread_ts: None,
            },
        };
        let body = serde_json::to_vec(&payload).expect("payload");

        let first = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/jobs/process_event")
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .expect("first request"),
            )
            .await
            .expect("first response");
        let second = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/jobs/process_event")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("second request"),
            )
            .await
            .expect("second response");

        let response_body = to_bytes(second.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&response_body).expect("json");
        let health = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("health request"),
            )
            .await
            .expect("health response");
        let health_body = to_bytes(health.into_body(), usize::MAX)
            .await
            .expect("health body");
        let health_payload: serde_json::Value =
            serde_json::from_slice(&health_body).expect("health json");

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(payload["duplicate"], true);
        assert_eq!(health_payload["queue_signature_verification"], false);
        assert_eq!(health_payload["tracked_events"], 1);
        assert_eq!(health_payload["tracked_messages"], 1);
    }

    #[tokio::test]
    async fn reaction_events_are_counted_in_health() {
        let tempdir = tempdir().expect("tempdir");
        let log_path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&log_path).await.expect("store");
        let router = build_router(
            store,
            WorkerConfig {
                host: "127.0.0.1".to_owned(),
                port: 4002,
                event_log_path: log_path.display().to_string(),
                worker_base_url: "http://127.0.0.1:4002".to_owned(),
                current_signing_key: None,
                next_signing_key: None,
            },
        )
        .expect("router");
        let payload = ProcessEventJob {
            event_id: "evt_reaction".to_owned(),
            team_id: "team_1".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ReactionAdded {
                user_id: "U123".to_owned(),
                reaction: "thumbsup".to_owned(),
                item_ts: "1700000000.000001".to_owned(),
            },
        };
        let body = serde_json::to_vec(&payload).expect("payload");

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/jobs/process_event")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        let health = router
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("health request"),
            )
            .await
            .expect("health response");
        let health_body = to_bytes(health.into_body(), usize::MAX)
            .await
            .expect("health body");
        let health_payload: serde_json::Value =
            serde_json::from_slice(&health_body).expect("health json");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(health_payload["tracked_events"], 1);
        assert_eq!(health_payload["tracked_messages"], 0);
        assert_eq!(health_payload["tracked_reactions"], 1);
    }
}
