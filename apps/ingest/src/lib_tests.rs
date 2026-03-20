use super::{IngestConfig, build_router};
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    routing::post,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::{net::TcpListener, task::JoinHandle};
use tower::util::ServiceExt;

#[test]
fn config_uses_local_defaults() {
    let config = IngestConfig::from_env();
    assert_eq!(config.bind_address(), "127.0.0.1:4001");
    assert_eq!(config.api_base_url, "http://127.0.0.1:4000");
    assert_eq!(config.worker_base_url, "http://127.0.0.1:4002");
    assert_eq!(config.slack_command_token, None);
    assert_eq!(config.qstash_base_url, None);
    assert_eq!(config.qstash_token, None);
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

#[tokio::test]
async fn health_reports_direct_queue_mode_by_default() {
    let router = build_router(IngestConfig::from_env()).expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(payload["queue_mode"], "direct");
    assert_eq!(
        payload["queue_endpoint"],
        "http://127.0.0.1:4002/jobs/process_event"
    );
}

#[tokio::test]
async fn health_reports_qstash_queue_mode_when_token_is_configured() {
    let router = build_router(IngestConfig {
        host: "127.0.0.1".to_owned(),
        port: 4001,
        api_base_url: "http://127.0.0.1:4000".to_owned(),
        worker_base_url: "https://worker.arkivist.dev".to_owned(),
        qstash_base_url: Some("qstash.upstash.io".to_owned()),
        qstash_token: Some("secret".to_owned()),
        slack_command_token: None,
        signing_secret: None,
    })
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(payload["queue_mode"], "qstash");
    assert_eq!(
        payload["queue_endpoint"],
        "https://worker.arkivist.dev/jobs/process_event"
    );
}

#[tokio::test]
async fn slash_command_stubs_supported_commands() {
    let router = build_router(IngestConfig::from_env()).expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/slack/commands/ask-arkivist")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "command=%2Fask-arkivist&text=release+status&channel_id=C123&user_id=U123",
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "/ask-arkivist");
    assert_eq!(payload["response_type"], "ephemeral");
    assert!(
        payload["text"]
            .as_str()
            .expect("text")
            .contains("release status")
    );
}

#[tokio::test]
async fn slash_command_rejects_unknown_command_paths() {
    let router = build_router(IngestConfig::from_env()).expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/slack/commands/not-a-command")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("command=%2Fnot-a-command"))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(payload["error"], "unknown_slash_command");
}

#[tokio::test]
async fn slash_command_rejects_mismatched_payloads() {
    let router = build_router(IngestConfig::from_env()).expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/slack/commands/recap")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("command=%2Fask-arkivist"))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["error"], "invalid_command_payload");
}

#[tokio::test]
async fn pin_highlight_command_calls_internal_api() {
    let state = MockPinHighlightState::default();
    let api = Router::new()
        .route("/api/internal/slack/highlights", post(mock_pin_highlight))
        .with_state(state.clone());
    let (api_base_url, api_handle) = spawn_app(api).await;
    let router = build_router(IngestConfig {
        host: "127.0.0.1".to_owned(),
        port: 4001,
        api_base_url,
        worker_base_url: "http://127.0.0.1:4002".to_owned(),
        qstash_base_url: None,
        qstash_token: None,
        slack_command_token: Some("internal-secret".to_owned()),
        signing_secret: None,
    })
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/slack/commands/pin-highlight")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "command=%2Fpin-highlight&text=%3Chttps%3A%2F%2Fworkspace.slack.com%2Farchives%2FC123%2Fp1700000000123456%3Fthread_ts%3D1700000000.000001%26cid%3DC123%7Cthread%3E&user_id=U123",
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");

    api_handle.abort();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["ok"], true);
    assert!(
        payload["text"]
            .as_str()
            .expect("text")
            .contains("C123:1700000000.000001")
    );
    assert_eq!(
        state.take_requests(),
        vec![CapturedPinHighlightRequest {
            authorization: Some("Bearer internal-secret".to_owned()),
            slack_user_id: "U123".to_owned(),
            thread_id: "C123:1700000000.000001".to_owned(),
        }]
    );
}

#[tokio::test]
async fn list_highlights_command_calls_internal_api() {
    let state = MockHighlightUserState::default();
    let api = Router::new()
        .route(
            "/api/internal/slack/highlights/list",
            post(mock_list_highlights),
        )
        .with_state(state.clone());
    let (api_base_url, api_handle) = spawn_app(api).await;
    let router = build_router(IngestConfig {
        host: "127.0.0.1".to_owned(),
        port: 4001,
        api_base_url,
        worker_base_url: "http://127.0.0.1:4002".to_owned(),
        qstash_base_url: None,
        qstash_token: None,
        slack_command_token: Some("internal-secret".to_owned()),
        signing_secret: None,
    })
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/slack/commands/list-highlights")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("command=%2Flist-highlights&user_id=U123"))
                .expect("request"),
        )
        .await
        .expect("response");
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");

    api_handle.abort();

    assert_eq!(payload["ok"], true);
    assert!(
        payload["text"]
            .as_str()
            .expect("text")
            .contains("C123:1700000000.000001")
    );
    assert_eq!(
        state.take_requests(),
        vec![CapturedHighlightUserRequest {
            authorization: Some("Bearer internal-secret".to_owned()),
            slack_user_id: "U123".to_owned(),
        }]
    );
}

#[tokio::test]
async fn pin_highlight_command_rejects_invalid_targets() {
    let router = build_router(IngestConfig {
        host: "127.0.0.1".to_owned(),
        port: 4001,
        api_base_url: "http://127.0.0.1:4000".to_owned(),
        worker_base_url: "http://127.0.0.1:4002".to_owned(),
        qstash_base_url: None,
        qstash_token: None,
        slack_command_token: Some("internal-secret".to_owned()),
        signing_secret: None,
    })
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/slack/commands/pin-highlight")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "command=%2Fpin-highlight&text=not-a-thread&user_id=U123",
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");

    assert_eq!(payload["ok"], false);
    assert!(
        payload["text"]
            .as_str()
            .expect("text")
            .contains("Usage: /pin-highlight")
    );
}

#[tokio::test]
async fn pin_highlight_command_reports_admin_requirement() {
    let api = Router::new().route(
        "/api/internal/slack/highlights",
        post(|| async {
            (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error":"admin_required"})),
            )
        }),
    );
    let (api_base_url, api_handle) = spawn_app(api).await;
    let router = build_router(IngestConfig {
        host: "127.0.0.1".to_owned(),
        port: 4001,
        api_base_url,
        worker_base_url: "http://127.0.0.1:4002".to_owned(),
        qstash_base_url: None,
        qstash_token: None,
        slack_command_token: Some("internal-secret".to_owned()),
        signing_secret: None,
    })
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/slack/commands/pin-highlight")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "command=%2Fpin-highlight&text=C123%3A1700000000.000001&user_id=U999",
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");

    api_handle.abort();

    assert_eq!(payload["ok"], false);
    assert_eq!(
        payload["text"],
        "Only admin users can manage highlighted threads."
    );
}

#[tokio::test]
async fn unpin_highlight_command_accepts_highlight_id() {
    let state = MockPinHighlightState::default();
    let api = Router::new()
        .route(
            "/api/internal/slack/highlights/unpin",
            post(mock_unpin_highlight),
        )
        .with_state(state.clone());
    let (api_base_url, api_handle) = spawn_app(api).await;
    let router = build_router(IngestConfig {
        host: "127.0.0.1".to_owned(),
        port: 4001,
        api_base_url,
        worker_base_url: "http://127.0.0.1:4002".to_owned(),
        qstash_base_url: None,
        qstash_token: None,
        slack_command_token: Some("internal-secret".to_owned()),
        signing_secret: None,
    })
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/slack/commands/unpin-highlight")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "command=%2Funpin-highlight&text=C123%3A1700000000.000001&user_id=U123",
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");

    api_handle.abort();

    assert_eq!(payload["ok"], true);
    assert!(
        payload["text"]
            .as_str()
            .expect("text")
            .contains("Unpinned C123:1700000000.000001")
    );
    assert_eq!(
        state.take_requests(),
        vec![CapturedPinHighlightRequest {
            authorization: Some("Bearer internal-secret".to_owned()),
            slack_user_id: "U123".to_owned(),
            thread_id: "C123:1700000000.000001".to_owned(),
        }]
    );
}

#[test]
fn pin_highlight_parses_supported_targets() {
    assert_eq!(
        super::slack_commands::parse_highlight_thread_target("C123:1700000000.000001"),
        Some("C123:1700000000.000001".to_owned())
    );
    assert_eq!(
        super::slack_commands::parse_highlight_thread_target(
            "<https://workspace.slack.com/archives/C123/p1700000000123456?thread_ts=1700000000.000001&cid=C123|thread>"
        ),
        Some("C123:1700000000.000001".to_owned())
    );
    assert_eq!(
        super::slack_commands::parse_highlight_thread_target(
            "https://workspace.slack.com/archives/C123/p1700000000123456"
        ),
        Some("C123:1700000000.123456".to_owned())
    );
    assert_eq!(
        super::slack_commands::parse_highlight_thread_target(""),
        None
    );
    assert_eq!(
        super::slack_commands::parse_highlight_thread_target("not-a-thread"),
        None
    );
}

#[derive(Debug, Clone, Default)]
struct MockPinHighlightState {
    requests: Arc<Mutex<Vec<CapturedPinHighlightRequest>>>,
}

impl MockPinHighlightState {
    fn take_requests(&self) -> Vec<CapturedPinHighlightRequest> {
        self.requests.lock().expect("requests lock").clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedPinHighlightRequest {
    authorization: Option<String>,
    slack_user_id: String,
    thread_id: String,
}

#[derive(Debug, Deserialize)]
struct MockPinHighlightPayload {
    slack_user_id: String,
    thread_id: String,
}

#[derive(Debug, Clone, Default)]
struct MockHighlightUserState {
    requests: Arc<Mutex<Vec<CapturedHighlightUserRequest>>>,
}

impl MockHighlightUserState {
    fn take_requests(&self) -> Vec<CapturedHighlightUserRequest> {
        self.requests.lock().expect("requests lock").clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedHighlightUserRequest {
    authorization: Option<String>,
    slack_user_id: String,
}

#[derive(Debug, Deserialize)]
struct MockHighlightUserPayload {
    slack_user_id: String,
}

async fn mock_pin_highlight(
    State(state): State<MockPinHighlightState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<MockPinHighlightPayload>,
) -> Json<Value> {
    state
        .requests
        .lock()
        .expect("requests lock")
        .push(CapturedPinHighlightRequest {
            authorization: headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            slack_user_id: payload.slack_user_id,
            thread_id: payload.thread_id,
        });

    Json(serde_json::json!({
        "item": {
            "thread_id": "C123:1700000000.000001"
        }
    }))
}

async fn mock_unpin_highlight(
    State(state): State<MockPinHighlightState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<MockPinHighlightPayload>,
) -> Json<Value> {
    state
        .requests
        .lock()
        .expect("requests lock")
        .push(CapturedPinHighlightRequest {
            authorization: headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            slack_user_id: payload.slack_user_id,
            thread_id: payload.thread_id,
        });

    Json(serde_json::json!({
        "ok": true
    }))
}

async fn mock_list_highlights(
    State(state): State<MockHighlightUserState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<MockHighlightUserPayload>,
) -> Json<Value> {
    state
        .requests
        .lock()
        .expect("requests lock")
        .push(CapturedHighlightUserRequest {
            authorization: headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            slack_user_id: payload.slack_user_id,
        });

    Json(serde_json::json!({
        "items": [
            {
                "thread_id": "C123:1700000000.000001",
                "channel_name": "general",
                "title": "First highlighted thread"
            },
            {
                "thread_id": "C234:1700000000.000002",
                "channel_name": "showcase",
                "title": "Second highlighted thread"
            }
        ]
    }))
}

async fn spawn_app(app: Router) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("listener address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (format!("http://{address}"), handle)
}
