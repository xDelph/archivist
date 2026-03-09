use axum::Router;
use base64::Engine;
use db::JsonlEventStore;
use hmac::{Hmac, Mac};
use ingest::{IngestConfig, build_router as build_ingest_router};
use queue::UPSTASH_SIGNATURE_HEADER;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use tokio::{net::TcpListener, task::JoinHandle};
use worker::{WorkerConfig, build_router as build_worker_router};

const QSTASH_TOKEN: &str = "qstash_token";
const CURRENT_SIGNING_KEY: &str = "current_signing_key";

#[tokio::test]
async fn slack_event_flows_from_ingest_to_worker() {
    let tempdir = tempdir().expect("tempdir");
    let event_log_path = tempdir.path().join("process-events.jsonl");
    let worker_store = JsonlEventStore::open(&event_log_path)
        .await
        .expect("worker store");
    let worker_router = build_worker_router(
        worker_store,
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: event_log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url: "https://slack.com/api".to_owned(),
            slack_bot_token: None,
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("worker router");
    let (worker_base_url, worker_handle) = spawn_app(worker_router).await;

    let ingest_router = build_ingest_router(IngestConfig {
        host: "127.0.0.1".to_owned(),
        port: 4001,
        worker_base_url: worker_base_url.clone(),
        qstash_base_url: None,
        qstash_token: None,
        signing_secret: None,
    })
    .expect("ingest router");
    let (ingest_base_url, ingest_handle) = spawn_app(ingest_router).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{ingest_base_url}/api/slack/events"))
        .json(&json!({
            "type": "event_callback",
            "team_id": "T123",
            "event_id": "Ev123",
            "event_time": 1700000000,
            "event": {
                "type": "message",
                "channel": "C123",
                "channel_type": "channel",
                "user": "U123",
                "text": "hello from ingest",
                "ts": "1700000000.000001",
                "thread_ts": null,
                "subtype": null
            }
        }))
        .send()
        .await
        .expect("ingest response");
    let response_payload: serde_json::Value = response.json().await.expect("response payload");

    let health_payload: serde_json::Value = client
        .get(format!("{worker_base_url}/health"))
        .send()
        .await
        .expect("worker health response")
        .json()
        .await
        .expect("worker health payload");
    let reopened_store = JsonlEventStore::open(&event_log_path)
        .await
        .expect("reopened store");
    let messages = reopened_store.messages().await;

    ingest_handle.abort();
    worker_handle.abort();

    assert_eq!(response_payload["ok"], true);
    assert_eq!(response_payload["enqueued"], true);
    assert_eq!(health_payload["tracked_events"], 1);
    assert_eq!(health_payload["tracked_messages"], 1);
    assert_eq!(health_payload["tracked_reactions"], 0);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "hello from ingest");
}

#[tokio::test]
async fn channel_rename_flows_from_ingest_to_worker() {
    let tempdir = tempdir().expect("tempdir");
    let event_log_path = tempdir.path().join("process-events.jsonl");
    let worker_store = JsonlEventStore::open(&event_log_path)
        .await
        .expect("worker store");
    let worker_router = build_worker_router(
        worker_store,
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: event_log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url: "https://slack.com/api".to_owned(),
            slack_bot_token: None,
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("worker router");
    let (worker_base_url, worker_handle) = spawn_app(worker_router).await;

    let ingest_router = build_ingest_router(IngestConfig {
        host: "127.0.0.1".to_owned(),
        port: 4001,
        worker_base_url: worker_base_url.clone(),
        qstash_base_url: None,
        qstash_token: None,
        signing_secret: None,
    })
    .expect("ingest router");
    let (ingest_base_url, ingest_handle) = spawn_app(ingest_router).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{ingest_base_url}/api/slack/events"))
        .json(&json!({
            "type": "event_callback",
            "team_id": "T123",
            "event_id": "EvChannel123",
            "event_time": 1700000600,
            "event": {
                "type": "channel_rename",
                "channel": {
                    "id": "C123",
                    "name": "announcements"
                }
            }
        }))
        .send()
        .await
        .expect("ingest response");
    let response_payload: serde_json::Value = response.json().await.expect("response payload");
    let reopened_store = JsonlEventStore::open(&event_log_path)
        .await
        .expect("reopened store");
    let channels = reopened_store.channels().await;

    ingest_handle.abort();
    worker_handle.abort();

    assert_eq!(response_payload["ok"], true);
    assert_eq!(response_payload["enqueued"], true);
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].id, "C123");
    assert_eq!(channels[0].name.as_deref(), Some("announcements"));
    assert!(!channels[0].is_archived);
}

#[tokio::test]
async fn slack_event_flows_from_ingest_to_worker_through_qstash_mock() {
    let tempdir = tempdir().expect("tempdir");
    let event_log_path = tempdir.path().join("process-events.jsonl");
    let worker_store = JsonlEventStore::open(&event_log_path)
        .await
        .expect("worker store");
    let worker_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind worker listener");
    let worker_address = worker_listener.local_addr().expect("worker address");
    let worker_base_url = format!("http://{worker_address}");
    let worker_router = build_worker_router(
        worker_store,
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: worker_address.port(),
            event_log_path: event_log_path.display().to_string(),
            worker_base_url: worker_base_url.clone(),
            slack_api_base_url: "https://slack.com/api".to_owned(),
            slack_bot_token: None,
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            current_signing_key: Some(CURRENT_SIGNING_KEY.to_owned()),
            next_signing_key: None,
        },
    )
    .expect("worker router");
    let worker_handle = spawn_listener(worker_listener, worker_router);

    let qstash_state = QStashMockState::new(QSTASH_TOKEN, CURRENT_SIGNING_KEY);
    let qstash_router = axum::Router::new()
        .route(
            "/v2/publish/{*destination}",
            axum::routing::post(qstash_publish),
        )
        .with_state(qstash_state.clone());
    let (qstash_base_url, qstash_handle) = spawn_app(qstash_router).await;

    let ingest_router = build_ingest_router(IngestConfig {
        host: "127.0.0.1".to_owned(),
        port: 4001,
        worker_base_url: worker_base_url.clone(),
        qstash_base_url: Some(qstash_base_url),
        qstash_token: Some(QSTASH_TOKEN.to_owned()),
        signing_secret: None,
    })
    .expect("ingest router");
    let (ingest_base_url, ingest_handle) = spawn_app(ingest_router).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{ingest_base_url}/api/slack/events"))
        .json(&json!({
            "type": "event_callback",
            "team_id": "T123",
            "event_id": "EvQstash123",
            "event_time": 1700000000,
            "event": {
                "type": "message",
                "channel": "C123",
                "channel_type": "channel",
                "user": "U123",
                "text": "hello through qstash",
                "ts": "1700000000.000009",
                "thread_ts": null,
                "subtype": null
            }
        }))
        .send()
        .await
        .expect("ingest response");
    let response_payload: serde_json::Value = response.json().await.expect("response payload");

    let health_payload: serde_json::Value = client
        .get(format!("{worker_base_url}/health"))
        .send()
        .await
        .expect("worker health response")
        .json()
        .await
        .expect("worker health payload");
    let reopened_store = JsonlEventStore::open(&event_log_path)
        .await
        .expect("reopened store");
    let messages = reopened_store.messages().await;

    ingest_handle.abort();
    qstash_handle.abort();
    worker_handle.abort();

    assert_eq!(response_payload["ok"], true);
    assert_eq!(response_payload["enqueued"], true);
    assert_eq!(qstash_state.publish_count(), 1);
    assert_eq!(health_payload["queue_signature_verification"], true);
    assert_eq!(health_payload["tracked_events"], 1);
    assert_eq!(health_payload["tracked_messages"], 1);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "hello through qstash");
}

async fn spawn_app(app: Router) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("listener address");
    let handle = spawn_listener(listener, app);

    (format!("http://{address}"), handle)
}

fn spawn_listener(listener: TcpListener, app: Router) -> JoinHandle<()> {
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    })
}

#[derive(Debug, Clone)]
struct QStashMockState {
    token: String,
    signing_key: String,
    publish_count: Arc<Mutex<usize>>,
}

impl QStashMockState {
    fn new(token: &str, signing_key: &str) -> Self {
        Self {
            token: token.to_owned(),
            signing_key: signing_key.to_owned(),
            publish_count: Arc::new(Mutex::new(0)),
        }
    }

    fn publish_count(&self) -> usize {
        *self.publish_count.lock().expect("publish count lock")
    }

    fn record_publish(&self) {
        let mut publish_count = self.publish_count.lock().expect("publish count lock");
        *publish_count += 1;
    }
}

async fn qstash_publish(
    axum::extract::State(state): axum::extract::State<QStashMockState>,
    axum::extract::Path(destination): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::http::StatusCode {
    let authorization = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());
    let expected_authorization = format!("Bearer {}", state.token);
    if authorization != Some(expected_authorization.as_str()) {
        return axum::http::StatusCode::UNAUTHORIZED;
    }

    let signature = sign_qstash_request(&state.signing_key, &body, &destination);
    let response = reqwest::Client::new()
        .post(&destination)
        .header(UPSTASH_SIGNATURE_HEADER, signature)
        .body(body.to_vec())
        .send()
        .await
        .expect("deliver to worker");

    if response.status().is_success() {
        state.record_publish();
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::BAD_GATEWAY
    }
}

fn sign_qstash_request(signing_key: &str, body: &[u8], url: &str) -> String {
    let now = current_unix_timestamp();
    let header = json!({
        "alg": "HS256",
        "typ": "JWT",
    });
    let claims = json!({
        "iss": "Upstash",
        "sub": url,
        "nbf": now - 5,
        "exp": now + 300,
        "body": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(body)),
    });
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&header).expect("header json"));
    let claims_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&claims).expect("claims json"));
    let signing_input = format!("{header_b64}.{claims_b64}");

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(signing_key.as_bytes()).expect("signing key");
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);

    format!("{signing_input}.{signature_b64}")
}

fn current_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}
