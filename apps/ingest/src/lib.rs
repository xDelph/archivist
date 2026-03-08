use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use queue::{DirectQueue, QueueError};
use serde::Serialize;
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
            signing_secret: std::env::var("SLACK_SIGNING_SECRET").ok(),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Clone)]
struct AppState {
    queue: DirectQueue,
    signing_secret: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HealthResponse {
    service: &'static str,
    version: &'static str,
    queue_endpoint: String,
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

pub fn build_router(config: IngestConfig) -> Result<Router, QueueError> {
    let queue = DirectQueue::new(&config.worker_base_url)?;

    Ok(Router::new()
        .route("/health", get(health))
        .route("/api/slack/events", post(slack_events))
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
            Ok((StatusCode::OK, Json(challenge)).into_response())
        }
        SlackEnvelope::EventCallback(callback) => {
            if let Some(job) = callback.into_job(current_unix_timestamp()) {
                state
                    .queue
                    .publish_process_event(&job)
                    .await
                    .map_err(queue_failed)?;

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
        SlackEnvelope::Unknown => Ok((
            StatusCode::OK,
            Json(EventAck {
                ok: true,
                enqueued: false,
                reason: "unknown_event",
            }),
        )
            .into_response()),
    }
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

fn queue_failed(_: QueueError) -> (StatusCode, Json<ErrorResponse>) {
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

#[cfg(test)]
mod tests {
    use super::{IngestConfig, build_router};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::util::ServiceExt;

    #[test]
    fn config_uses_local_defaults() {
        let config = IngestConfig::from_env();
        assert_eq!(config.bind_address(), "127.0.0.1:4001");
        assert_eq!(config.worker_base_url, "http://127.0.0.1:4002");
    }

    #[tokio::test]
    async fn url_verification_is_echoed_back() {
        let router = build_router(IngestConfig::from_env()).expect("router");
        let body = r#"{"type":"url_verification","challenge":"abc123"}"#;

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/slack/events")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
