use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use db::{JsonlEventStore, RepositoryMode, StoreOutcome};
use domain::ProcessEventJob;
use serde::Serialize;
use tower_http::trace::TraceLayer;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4002;
const DEFAULT_EVENT_LOG_PATH: &str = "logs/process-events.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerConfig {
    pub host: String,
    pub port: u16,
    pub event_log_path: String,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("ARCHIVIST_WORKER_HOST")
                .unwrap_or_else(|_| DEFAULT_HOST.to_owned()),
            port: read_port("ARCHIVIST_WORKER_PORT", DEFAULT_PORT),
            event_log_path: std::env::var("ARCHIVIST_EVENT_LOG_PATH")
                .unwrap_or_else(|_| DEFAULT_EVENT_LOG_PATH.to_owned()),
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
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HealthResponse {
    service: &'static str,
    version: &'static str,
    repository_mode: &'static str,
    event_log_path: String,
    tracked_events: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ProcessEventResponse {
    ok: bool,
    duplicate: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HeartbeatResponse {
    ok: bool,
    job: &'static str,
}

pub fn build_router(store: JsonlEventStore, event_log_path: String) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/jobs/process_event", post(process_event))
        .route("/jobs/heartbeat", post(heartbeat))
        .with_state(AppState {
            store,
            event_log_path,
        })
        .layer(TraceLayer::new_for_http())
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let repository_health = state.store.health().await;

    Json(HealthResponse {
        service: "worker",
        version: env!("CARGO_PKG_VERSION"),
        repository_mode: RepositoryMode::LocalJsonlMock.as_str(),
        event_log_path: state.event_log_path,
        tracked_events: repository_health.tracked_events,
    })
}

async fn process_event(
    State(state): State<AppState>,
    Json(job): Json<ProcessEventJob>,
) -> Json<ProcessEventResponse> {
    let duplicate = matches!(
        state.store.record_process_event(&job).await,
        Ok(StoreOutcome::Duplicate)
    );

    Json(ProcessEventResponse {
        ok: true,
        duplicate,
    })
}

async fn heartbeat() -> Json<HeartbeatResponse> {
    Json(HeartbeatResponse {
        ok: true,
        job: "heartbeat",
    })
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
    }

    #[tokio::test]
    async fn duplicate_events_are_acknowledged() {
        let tempdir = tempdir().expect("tempdir");
        let log_path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&log_path).await.expect("store");
        let router = build_router(store, log_path.display().to_string());
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

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(payload["duplicate"], true);
    }
}
