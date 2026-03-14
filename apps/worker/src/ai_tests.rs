use crate::{WorkerConfig, ai_openrouter::chat_completions_url, build_router};
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    routing::post,
};
use db::{GeneratedThreadSummaryRow, JsonlEventStore};
use domain::{ChannelKind, EventPayload, ProcessEventJob};
use serde_json::json;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};
use tower::util::ServiceExt;

#[derive(Clone, Default)]
struct MockAiState {
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
}

#[tokio::test]
async fn generate_thread_summaries_requires_openrouter_config() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    seed_root_message(&store, "C123", "1700000000.000001", "root message").await;
    let router = build_router(
        store,
        worker_config(&log_path, "https://openrouter.ai/api/v1", None, None),
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/generate_thread_summaries")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"channel_id":"C123"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["error"], "missing_openrouter_config");
}

#[tokio::test]
async fn generate_thread_summaries_persists_model_output() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    seed_root_message(
        &store,
        "C123",
        "1700000000.000001",
        "How should we run the launch?",
    )
    .await;
    seed_reply_message(
        &store,
        "C123",
        "1700000000.000002",
        "1700000000.000001",
        "We should publish a short checklist and owners.",
    )
    .await;
    let (openrouter_base_url, state, handle) = spawn_openrouter_server().await;
    let router = build_router(
        store.clone(),
        worker_config(
            &log_path,
            &openrouter_base_url,
            Some("test-openrouter-key"),
            Some("openai/gpt-oss-120b:free"),
        ),
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/generate_thread_summaries")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"channel_id":"C123"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["generated"], 1);
    assert_eq!(payload["skipped"], 0);
    assert_eq!(payload["failed"], 0);

    let summaries = store.generated_thread_summaries().await;
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].channel_id, "C123");
    assert_eq!(summaries[0].root_ts, "1700000000.000001");
    assert_eq!(summaries[0].status, "answered");
    assert_eq!(summaries[0].topic_tags, vec!["launch", "checklist"]);
    assert_eq!(summaries[0].model, "openai/gpt-oss-120b:free");
    assert!(summaries[0].summary.contains("launch plan"));

    let requests = state.requests.lock().await;
    assert_eq!(requests.len(), 1);
}

#[test]
fn chat_completions_url_accepts_base_or_full_endpoint() {
    assert_eq!(
        chat_completions_url("https://openrouter.ai/api/v1"),
        "https://openrouter.ai/api/v1/chat/completions"
    );
    assert_eq!(
        chat_completions_url("https://openrouter.ai/api/v1/chat/completions"),
        "https://openrouter.ai/api/v1/chat/completions"
    );
}

#[tokio::test]
async fn generate_thread_summaries_skips_unchanged_threads_for_same_model() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    seed_root_message(&store, "C123", "1700000000.000001", "Root message").await;
    let last_activity_ts = store.thread_summaries().await[0].last_activity_ts.clone();
    store
        .upsert_generated_thread_summary(&GeneratedThreadSummaryRow {
            channel_id: "C123".to_owned(),
            root_ts: "1700000000.000001".to_owned(),
            summary: "Existing summary".to_owned(),
            why_it_mattered: Some("Existing why".to_owned()),
            status: "discussion".to_owned(),
            topic_tags: vec!["existing".to_owned()],
            source_last_activity_ts: last_activity_ts,
            model: "openai/gpt-oss-120b:free".to_owned(),
            generated_at: 1,
        })
        .await
        .expect("seed generated summary");
    let (openrouter_base_url, state, handle) = spawn_openrouter_server().await;
    let router = build_router(
        store,
        worker_config(
            &log_path,
            &openrouter_base_url,
            Some("test-openrouter-key"),
            Some("openai/gpt-oss-120b:free"),
        ),
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/generate_thread_summaries")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"channel_id":"C123"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["generated"], 0);
    assert_eq!(payload["skipped"], 1);
    assert_eq!(payload["failed"], 0);
    assert_eq!(state.requests.lock().await.len(), 0);
}

#[tokio::test]
async fn generate_thread_summaries_skips_threads_without_replies() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    seed_root_message(
        &store,
        "C123",
        "1700000000.000001",
        "Root message without replies",
    )
    .await;
    let (openrouter_base_url, state, handle) = spawn_openrouter_server().await;
    let router = build_router(
        store,
        worker_config(
            &log_path,
            &openrouter_base_url,
            Some("test-openrouter-key"),
            Some("openai/gpt-oss-120b:free"),
        ),
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/generate_thread_summaries")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"channel_id":"C123"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["generated"], 0);
    assert_eq!(payload["skipped"], 1);
    assert_eq!(payload["failed"], 0);
    assert_eq!(state.requests.lock().await.len(), 0);
}

#[tokio::test]
async fn generate_thread_summaries_resume_mode_filters_per_channel() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    seed_root_message(&store, "C123", "1700000000.000001", "Old general thread").await;
    seed_root_message(&store, "C123", "1700000100.000001", "New general thread").await;
    seed_reply_message(
        &store,
        "C123",
        "1700000101.000001",
        "1700000100.000001",
        "Follow-up reply for the new general thread",
    )
    .await;
    seed_root_message(&store, "C456", "1700000200.000001", "Old random thread").await;
    seed_root_message(&store, "C456", "1700000300.000001", "New random thread").await;
    seed_reply_message(
        &store,
        "C456",
        "1700000301.000001",
        "1700000300.000001",
        "Follow-up reply for the new random thread",
    )
    .await;
    let (openrouter_base_url, state, handle) = spawn_openrouter_server().await;
    let router = build_router(
        store.clone(),
        worker_config(
            &log_path,
            &openrouter_base_url,
            Some("test-openrouter-key"),
            Some("openai/gpt-oss-120b:free"),
        ),
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/generate_thread_summaries")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"resume_from_last_message_ts":true}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["generated"], 2);
    assert_eq!(payload["skipped"], 0);
    assert_eq!(payload["failed"], 0);

    let summaries = store.generated_thread_summaries().await;
    let roots = summaries
        .into_iter()
        .map(|row| (row.channel_id, row.root_ts))
        .collect::<Vec<_>>();
    assert_eq!(
        roots,
        vec![
            ("C123".to_owned(), "1700000100.000001".to_owned()),
            ("C456".to_owned(), "1700000300.000001".to_owned()),
        ]
    );
    assert_eq!(state.requests.lock().await.len(), 2);
}

fn worker_config(
    log_path: &std::path::Path,
    openrouter_base_url: &str,
    openrouter_api_key: Option<&str>,
    openrouter_model: Option<&str>,
) -> WorkerConfig {
    WorkerConfig {
        host: "127.0.0.1".to_owned(),
        port: 4002,
        event_log_path: log_path.display().to_string(),
        worker_base_url: "http://127.0.0.1:4002".to_owned(),
        slack_api_base_url: "https://slack.com/api".to_owned(),
        openrouter_base_url: openrouter_base_url.to_owned(),
        openrouter_api_key: openrouter_api_key.map(str::to_owned),
        openrouter_model: openrouter_model.map(str::to_owned),
        slack_user_token: None,
        r2_account_id: None,
        r2_access_key_id: None,
        r2_secret_access_key: None,
        r2_bucket: None,
        r2_public_url: None,
        r2_endpoint_url: None,
        r2_key_prefix: None,
        current_signing_key: None,
        next_signing_key: None,
    }
}

async fn seed_root_message(store: &JsonlEventStore, channel_id: &str, ts: &str, text: &str) {
    store
        .record_process_event(&ProcessEventJob {
            event_id: format!("evt:{channel_id}:{ts}"),
            event_time: 1,
            received_at: 2,
            channel_id: channel_id.to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some(text.to_owned()),
                ts: ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("insert root");
}

async fn seed_reply_message(
    store: &JsonlEventStore,
    channel_id: &str,
    ts: &str,
    thread_ts: &str,
    text: &str,
) {
    store
        .record_process_event(&ProcessEventJob {
            event_id: format!("evt:{channel_id}:{ts}"),
            event_time: 1,
            received_at: 2,
            channel_id: channel_id.to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U456".to_owned()),
                text: Some(text.to_owned()),
                ts: ts.to_owned(),
                thread_ts: Some(thread_ts.to_owned()),
                files: vec![],
            },
        })
        .await
        .expect("insert reply");
}

async fn spawn_openrouter_server() -> (String, MockAiState, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let state = MockAiState::default();
    let app = Router::new()
        .route(
            "/chat/completions",
            post(
                |State(state): State<MockAiState>, Json(payload): Json<serde_json::Value>| async move {
                    state.requests.lock().await.push(payload);
                    Json(json!({
                        "choices": [
                            {
                                "message": {
                                    "content": "{\"summary\":\"This thread converged on a launch plan with a checklist and owners.\",\"why_it_mattered\":\"It defined the concrete next steps for shipping.\",\"status\":\"answered\",\"topic_tags\":[\"launch\",\"checklist\"]}"
                                }
                            }
                        ]
                    }))
                },
            ),
        )
        .with_state(state.clone());
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (format!("http://{address}"), state, handle)
}
