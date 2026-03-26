use crate::{WorkerConfig, build_router};
use axum::{
    Json, Router,
    body::{Body, Bytes, to_bytes},
    extract::{Path, Query, State},
    http::{HeaderMap, Request, StatusCode},
    response::IntoResponse,
    routing::{get, head, put},
};
use db::JsonlEventStore;
use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tempfile::tempdir;
use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};
use tower::util::ServiceExt;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct UploadCapture {
    path: String,
    body: Vec<u8>,
    content_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct MockStorageState {
    upload: Arc<Mutex<Option<UploadCapture>>>,
    existing_public_keys: Arc<Mutex<HashSet<String>>>,
}

#[tokio::test]
async fn backfill_channel_rejects_missing_user_token() {
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
            slack_api_base_url: "https://slack.com/api".to_owned(),
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
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
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "channel_id": "C123" }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn backfill_channel_fetches_history_and_upserts_messages() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, handle) = spawn_history_server(Json(json!({
        "ok": true,
        "messages": [
            {
                "ts": "1700000000.000002",
                "user": "U456",
                "text": "reply from backfill",
                "thread_ts": "1700000000.000001",
                "reactions": [
                    {
                        "name": "eyes",
                        "users": ["U999"]
                    }
                ]
            },
            {
                "ts": "1700000000.000001",
                "user": "U123",
                "text": "hello from backfill",
                "files": [
                    {
                        "id": "F123",
                        "name": "brief.pdf",
                        "mimetype": "application/pdf",
                        "permalink": "https://files.example.com/brief.pdf",
                        "size": 42
                    }
                ]
            }
        ],
        "response_metadata": {
            "next_cursor": "cursor_2"
        }
    })))
    .await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");
    let body = json!({ "channel_id": "C123" }).to_string();

    let first = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .expect("request"),
        )
        .await
        .expect("first response");
    let second = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("second response");

    handle.abort();

    assert_eq!(first.status(), StatusCode::OK);
    let first_body = to_bytes(first.into_body(), usize::MAX)
        .await
        .expect("first body");
    let first_payload: serde_json::Value = serde_json::from_slice(&first_body).expect("json");
    assert_eq!(first_payload["inserted"], 2);
    assert_eq!(first_payload["duplicate"], 0);
    assert_eq!(first_payload["next_cursor"], "cursor_2");

    let second_body = to_bytes(second.into_body(), usize::MAX)
        .await
        .expect("second body");
    let second_payload: serde_json::Value = serde_json::from_slice(&second_body).expect("json");
    assert_eq!(second_payload["inserted"], 0);
    assert_eq!(second_payload["duplicate"], 2);

    let messages = store.messages().await;
    let reactions = store.reactions().await;
    let files = store.files().await;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text, "hello from backfill");
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0].name, "eyes");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "brief.pdf");
    assert_eq!(
        files[0].permalink.as_deref(),
        Some("https://files.example.com/brief.pdf")
    );
}

#[tokio::test]
async fn backfill_channel_uploads_files_to_r2_and_sets_archive_url() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, state, handle) = spawn_history_archive_server(Json(json!({
        "ok": true,
        "messages": [
            {
                "ts": "1700000000.000001",
                "user": "U123",
                "text": "hello from backfill",
                "files": [
                    {
                        "id": "F123",
                        "name": "brief.pdf",
                        "mimetype": "application/pdf",
                        "permalink": "https://files.example.com/brief.pdf",
                        "url_private_download": "http://placeholder/download/file",
                        "size": 42
                    }
                ]
            }
        ]
    })))
    .await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url: slack_api_base_url.clone(),
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: Some("acct".to_owned()),
            r2_access_key_id: Some("key".to_owned()),
            r2_secret_access_key: Some("secret".to_owned()),
            r2_bucket: Some("bucket".to_owned()),
            r2_public_url: Some(format!("{slack_api_base_url}/public/")),
            r2_endpoint_url: Some(slack_api_base_url.clone()),
            r2_key_prefix: Some("archive".to_owned()),
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "channel_id": "C123" }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let files = store.files().await;
    assert_eq!(files.len(), 1);
    let expected_public_url = format!("{slack_api_base_url}/public//archive/C123/F123/brief.pdf");
    assert_eq!(
        files[0].permalink.as_deref(),
        Some(expected_public_url.as_str())
    );

    let capture = state.upload.lock().await.clone().expect("upload capture");
    assert_eq!(capture.path, "bucket/archive/C123/F123/brief.pdf");
    assert_eq!(capture.body, b"hello world");
    assert_eq!(capture.content_type.as_deref(), Some("application/pdf"));
}

#[tokio::test]
async fn backfill_channel_reuses_existing_r2_object_without_slack_download_url() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, state, handle) = spawn_history_archive_server(Json(json!({
        "ok": true,
        "messages": [
            {
                "ts": "1700000000.000001",
                "user": "U123",
                "text": "hello from backfill",
                "files": [
                    {
                        "id": "F123",
                        "name": "brief.pdf",
                        "mimetype": "application/pdf",
                        "permalink": "https://files.example.com/brief.pdf",
                        "size": 42
                    }
                ]
            }
        ]
    })))
    .await;
    state
        .existing_public_keys
        .lock()
        .await
        .insert("/archive/C123/F123/brief.pdf".to_owned());
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url: slack_api_base_url.clone(),
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: Some("acct".to_owned()),
            r2_access_key_id: Some("key".to_owned()),
            r2_secret_access_key: Some("secret".to_owned()),
            r2_bucket: Some("bucket".to_owned()),
            r2_public_url: Some(format!("{slack_api_base_url}/public/")),
            r2_endpoint_url: Some(slack_api_base_url.clone()),
            r2_key_prefix: Some("archive".to_owned()),
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "channel_id": "C123" }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let files = store.files().await;
    assert_eq!(files.len(), 1);
    let expected_public_url = format!("{slack_api_base_url}/public//archive/C123/F123/brief.pdf");
    assert_eq!(
        files[0].permalink.as_deref(),
        Some(expected_public_url.as_str())
    );
    assert_eq!(state.upload.lock().await.clone(), None);
}

#[tokio::test]
async fn backfill_channel_fails_when_file_has_no_archive_and_no_download_url() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, _state, handle) = spawn_history_archive_server(Json(json!({
        "ok": true,
        "messages": [
            {
                "ts": "1700000000.000001",
                "user": "U123",
                "text": "hello from backfill",
                "files": [
                    {
                        "id": "F123",
                        "name": "brief.pdf",
                        "mimetype": "application/pdf",
                        "permalink": "https://files.example.com/brief.pdf",
                        "size": 42
                    }
                ]
            }
        ]
    })))
    .await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url: slack_api_base_url.clone(),
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: Some("acct".to_owned()),
            r2_access_key_id: Some("key".to_owned()),
            r2_secret_access_key: Some("secret".to_owned()),
            r2_bucket: Some("bucket".to_owned()),
            r2_public_url: Some(format!("{slack_api_base_url}/public/")),
            r2_endpoint_url: Some(slack_api_base_url.clone()),
            r2_key_prefix: Some("archive".to_owned()),
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "channel_id": "C123" }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["error"], "missing_file_download_url");
    let files = store.files().await;
    assert_eq!(files.len(), 1);
    assert_eq!(
        files[0].permalink.as_deref(),
        Some("https://files.example.com/brief.pdf")
    );
}

#[tokio::test]
async fn backfill_channel_fetches_thread_replies_for_root_messages() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, handle) = spawn_thread_history_server().await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"channel_id":"C123"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let messages = store.messages().await;
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages
            .iter()
            .filter(|message| message.thread_ts.is_some())
            .count(),
        2
    );
}

#[tokio::test]
async fn backfill_channel_retries_thread_replies_after_rate_limit() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, handle) = spawn_thread_history_server_with_reply_rate_limit().await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"channel_id":"C123"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let messages = store.messages().await;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].thread_ts.as_deref(), Some("1700000000.000001"));
}

#[tokio::test]
async fn backfill_channel_tolerates_files_missing_names() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, handle) = spawn_history_server(Json(json!({
        "ok": true,
        "messages": [
            {
                "ts": "1700000000.000001",
                "user": "U123",
                "text": "hello from backfill",
                "files": [
                    {
                        "id": "F123",
                        "mimetype": "application/pdf",
                        "permalink": "https://files.example.com/brief.pdf",
                        "size": 42
                    }
                ]
            }
        ]
    })))
    .await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"channel_id":"C123"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let files = store.files().await;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].id, "F123");
    assert_eq!(files[0].name, "F123");
}

#[tokio::test]
async fn backfill_channel_returns_bad_gateway_when_slack_history_fails() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, handle) = spawn_history_server(Json(json!({ "ok": false }))).await;
    let router = build_router(
        store,
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "channel_id": "C123" }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn backfill_channel_uses_explicit_oldest_ts() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, handle) = spawn_history_server_with_query_assertions(
        HashMap::from([
            ("channel".to_owned(), "C123".to_owned()),
            ("oldest".to_owned(), "1700000000.125".to_owned()),
            ("inclusive".to_owned(), "false".to_owned()),
        ]),
        Json(json!({
            "ok": true,
            "messages": [{ "ts": "1700000001.000001", "user": "U123", "text": "newer message" }]
        })),
    )
    .await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "channel_id": "C123", "oldest_ts": 1700000000.125 }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let messages = store.messages().await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].ts, "1700000001.000001");
}

#[tokio::test]
async fn backfill_channel_resume_from_last_message_ts_uses_latest_stored_message() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_existing".to_owned(),
            event_time: 1_700_000_000,
            received_at: 1_700_000_000,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U111".to_owned()),
                text: Some("existing message".to_owned()),
                ts: "1700000000.000001".to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("insert existing message");
    let (slack_api_base_url, handle) = spawn_history_server_with_query_assertions(
        HashMap::from([
            ("channel".to_owned(), "C123".to_owned()),
            ("oldest".to_owned(), "1700000000.000001".to_owned()),
            ("inclusive".to_owned(), "true".to_owned()),
        ]),
        Json(json!({
            "ok": true,
            "messages": [{ "ts": "1700000000.000002", "user": "U222", "text": "new message" }]
        })),
    )
    .await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "channel_id": "C123",
                        "resume_from_last_message_ts": true
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let messages = store.messages().await;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].ts, "1700000000.000001");
    assert_eq!(messages[1].ts, "1700000000.000002");
}

#[tokio::test]
async fn backfill_channel_resume_from_last_message_ts_reprocesses_boundary_files() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "backfill:C123:1700000000.000001".to_owned(),
            event_time: 1_700_000_000,
            received_at: 1_700_000_000,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U111".to_owned()),
                text: Some("existing message".to_owned()),
                ts: "1700000000.000001".to_owned(),
                thread_ts: None,
                files: vec![SharedFile {
                    id: "F123".to_owned(),
                    name: "brief.pdf".to_owned(),
                    mimetype: Some("application/pdf".to_owned()),
                    permalink: Some("https://files.example.com/brief.pdf".to_owned()),
                    size: Some(42),
                }],
            },
        })
        .await
        .expect("insert existing message");
    let (slack_api_base_url, state, handle) = spawn_history_archive_server_with_query_assertions(
        HashMap::from([
            ("channel".to_owned(), "C123".to_owned()),
            ("oldest".to_owned(), "1700000000.000001".to_owned()),
            ("inclusive".to_owned(), "true".to_owned()),
        ]),
        Json(json!({
            "ok": true,
            "messages": [
                {
                    "ts": "1700000000.000001",
                    "user": "U111",
                    "text": "existing message",
                    "files": [
                        {
                            "id": "F123",
                            "name": "brief.pdf",
                            "mimetype": "application/pdf",
                            "permalink": "https://files.example.com/brief.pdf",
                            "url_private_download": "http://placeholder/download/file",
                            "size": 42
                        }
                    ]
                }
            ]
        })),
    )
    .await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url: slack_api_base_url.clone(),
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: Some("acct".to_owned()),
            r2_access_key_id: Some("key".to_owned()),
            r2_secret_access_key: Some("secret".to_owned()),
            r2_bucket: Some("bucket".to_owned()),
            r2_public_url: Some(format!("{slack_api_base_url}/public/")),
            r2_endpoint_url: Some(slack_api_base_url.clone()),
            r2_key_prefix: Some("archive".to_owned()),
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "channel_id": "C123",
                        "resume_from_last_message_ts": true
                    })
                    .to_string(),
                ))
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
    assert_eq!(payload["inserted"], 0);
    assert_eq!(payload["duplicate"], 1);

    let files = store.files().await;
    assert_eq!(files.len(), 1);
    let expected_public_url = format!("{slack_api_base_url}/public//archive/C123/F123/brief.pdf");
    assert_eq!(
        files[0].permalink.as_deref(),
        Some(expected_public_url.as_str())
    );

    let capture = state.upload.lock().await.clone().expect("upload capture");
    assert_eq!(capture.path, "bucket/archive/C123/F123/brief.pdf");
    assert_eq!(capture.body, b"hello world");
    assert_eq!(capture.content_type.as_deref(), Some("application/pdf"));
}

#[tokio::test]
async fn backfill_without_channel_id_uses_explicit_oldest_ts_for_each_public_channel() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (slack_api_base_url, handle) = spawn_workspace_backfill_server_with_history_assertions(
        HashMap::from([
            ("C123".to_owned(), "1700000000.125".to_owned()),
            ("C456".to_owned(), "1700000000.125".to_owned()),
        ]),
        false,
        HashMap::from([
            (
                "C123".to_owned(),
                json!({
                    "ok": true,
                    "messages": [{ "ts": "1700000000.000001", "user": "U123", "text": "first channel" }]
                }),
            ),
            (
                "C456".to_owned(),
                json!({
                    "ok": true,
                    "messages": [{ "ts": "1700000001.000001", "user": "U456", "text": "second channel" }]
                }),
            ),
        ]),
    )
    .await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "oldest_ts": 1700000000.125 }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let messages = store.messages().await;
    assert_eq!(messages.len(), 2);
}

#[tokio::test]
async fn backfill_without_channel_id_resume_from_last_message_ts_uses_each_channel_latest_ts() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    for (event_id, channel_id, ts) in [
        ("evt-1", "C123", "1700000005.000001"),
        ("evt-2", "C456", "1700000003.000001"),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                event_time: 1_700_000_000,
                received_at: 1_700_000_000,
                channel_id: channel_id.to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some("existing message".to_owned()),
                    ts: ts.to_owned(),
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("insert existing message");
    }
    let (slack_api_base_url, handle) = spawn_workspace_backfill_server_with_history_assertions(
        HashMap::from([
            ("C123".to_owned(), "1700000005.000001".to_owned()),
            ("C456".to_owned(), "1700000003.000001".to_owned()),
        ]),
        true,
        HashMap::from([
            (
                "C123".to_owned(),
                json!({
                    "ok": true,
                    "messages": [{ "ts": "1700000006.000001", "user": "U123", "text": "first channel new" }]
                }),
            ),
            (
                "C456".to_owned(),
                json!({
                    "ok": true,
                    "messages": [{ "ts": "1700000004.000001", "user": "U456", "text": "second channel new" }]
                }),
            ),
        ]),
    )
    .await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "resume_from_last_message_ts": true }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    handle.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let messages = store.messages().await;
    assert_eq!(messages.len(), 4);
}

#[tokio::test]
async fn backfill_without_channel_id_fetches_all_public_channels() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let histories = HashMap::from([
        (
            "C123".to_owned(),
            json!({
                "ok": true,
                "messages": [{ "ts": "1700000000.000001", "user": "U123", "text": "first channel" }]
            }),
        ),
        (
            "C456".to_owned(),
            json!({
                "ok": true,
                "messages": [{ "ts": "1700000001.000001", "user": "U456", "text": "second channel" }]
            }),
        ),
    ]);
    let (slack_api_base_url, handle) = spawn_workspace_backfill_server(histories).await;
    let router = build_router(
        store.clone(),
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url,
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .body(Body::empty())
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
    assert_eq!(payload["inserted"], 2);
    assert_eq!(payload["duplicate"], 0);
    assert_eq!(payload["next_cursor"], serde_json::Value::Null);

    let messages = store.messages().await;
    let channels = store.channels().await;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].channel_id, "C123");
    assert_eq!(messages[1].channel_id, "C456");
    assert_eq!(channels.len(), 2);
    assert_eq!(channels[0].name.as_deref(), Some("general"));
    assert_eq!(channels[1].name.as_deref(), Some("random"));
}

#[tokio::test]
async fn backfill_channel_rejects_invalid_json_body() {
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
            slack_api_base_url: "https://slack.com/api".to_owned(),
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from("{"))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["error"], "invalid_backfill_request");
}

#[tokio::test]
async fn backfill_channel_rejects_conflicting_range_options() {
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
            slack_api_base_url: "https://slack.com/api".to_owned(),
            openrouter_base_url: "https://openrouter.ai/api/v1".to_owned(),
            openrouter_api_key: None,
            openrouter_model: None,
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: None,
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/backfill_channel")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "channel_id": "C123",
                        "oldest_ts": "1700000000.000001",
                        "resume_from_last_message_ts": true
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["error"], "invalid_backfill_request");
}

async fn spawn_history_server(response: Json<serde_json::Value>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let app = Router::new().route(
        "/conversations.history",
        get(move |_body: Bytes| {
            let response = response.clone();
            async move { response }
        }),
    );
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (format!("http://{address}"), handle)
}

async fn spawn_history_archive_server(
    response: Json<serde_json::Value>,
) -> (String, MockStorageState, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let base_url = format!("http://{address}");
    let state = MockStorageState::default();
    let history = rewrite_download_urls(response.0, &base_url);
    let app = Router::new()
        .route(
            "/conversations.history",
            get({
                let history = history.clone();
                move || {
                    let history = history.clone();
                    async move { Json(history) }
                }
            }),
        )
        .route(
            "/download/file",
            get(|| async {
                (
                    [(reqwest::header::CONTENT_TYPE.as_str(), "application/pdf")],
                    "hello world",
                )
                    .into_response()
            }),
        )
        .route(
            "/public/{*key}",
            head(
                |State(state): State<MockStorageState>, Path(key): Path<String>| async move {
                    if state.existing_public_keys.lock().await.contains(&key) {
                        StatusCode::OK
                    } else {
                        StatusCode::NOT_FOUND
                    }
                },
            ),
        )
        .route(
            "/{bucket}/{*key}",
            put(
                |State(state): State<MockStorageState>,
                 Path((bucket, key)): Path<(String, String)>,
                 headers: HeaderMap,
                 body: Bytes| async move {
                    *state.upload.lock().await = Some(UploadCapture {
                        path: format!("{bucket}/{key}"),
                        body: body.to_vec(),
                        content_type: headers
                            .get(reqwest::header::CONTENT_TYPE)
                            .and_then(|value| value.to_str().ok())
                            .map(str::to_owned),
                    });
                    StatusCode::OK
                },
            ),
        )
        .with_state(state.clone());
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (base_url, state, handle)
}

async fn spawn_history_archive_server_with_query_assertions(
    expected_query: HashMap<String, String>,
    response: Json<serde_json::Value>,
) -> (String, MockStorageState, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let base_url = format!("http://{address}");
    let state = MockStorageState::default();
    let history = rewrite_download_urls(response.0, &base_url);
    let app = Router::new()
        .route(
            "/conversations.history",
            get({
                let history = history.clone();
                move |Query(query): Query<HashMap<String, String>>| {
                    let history = history.clone();
                    let expected_query = expected_query.clone();
                    async move {
                        for (key, expected) in expected_query {
                            assert_eq!(
                                query.get(&key),
                                Some(&expected),
                                "query mismatch for {key}"
                            );
                        }
                        Json(history)
                    }
                }
            }),
        )
        .route(
            "/download/file",
            get(|| async {
                (
                    [(reqwest::header::CONTENT_TYPE.as_str(), "application/pdf")],
                    "hello world",
                )
                    .into_response()
            }),
        )
        .route(
            "/public/{*key}",
            head(
                |State(state): State<MockStorageState>, Path(key): Path<String>| async move {
                    if state.existing_public_keys.lock().await.contains(&key) {
                        StatusCode::OK
                    } else {
                        StatusCode::NOT_FOUND
                    }
                },
            ),
        )
        .route(
            "/{bucket}/{*key}",
            put(
                |State(state): State<MockStorageState>,
                 Path((bucket, key)): Path<(String, String)>,
                 headers: HeaderMap,
                 body: Bytes| async move {
                    *state.upload.lock().await = Some(UploadCapture {
                        path: format!("{bucket}/{key}"),
                        body: body.to_vec(),
                        content_type: headers
                            .get(reqwest::header::CONTENT_TYPE)
                            .and_then(|value| value.to_str().ok())
                            .map(str::to_owned),
                    });
                    StatusCode::OK
                },
            ),
        )
        .with_state(state.clone());
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (base_url, state, handle)
}

async fn spawn_history_server_with_query_assertions(
    expected_query: HashMap<String, String>,
    response: Json<serde_json::Value>,
) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let app = Router::new().route(
        "/conversations.history",
        get(move |Query(query): Query<HashMap<String, String>>| {
            let expected_query = expected_query.clone();
            let response = response.clone();
            async move {
                for (key, expected) in expected_query {
                    assert_eq!(query.get(&key), Some(&expected), "query mismatch for {key}");
                }
                response
            }
        }),
    );
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (format!("http://{address}"), handle)
}

fn rewrite_download_urls(payload: serde_json::Value, base_url: &str) -> serde_json::Value {
    match payload {
        serde_json::Value::Object(mut object) => {
            for (key, value) in object.iter_mut() {
                if matches!(key.as_str(), "url_private" | "url_private_download") {
                    if let serde_json::Value::String(url) = value
                        && url.starts_with("http://placeholder/")
                    {
                        *url = format!(
                            "{base_url}/{}",
                            url.trim_start_matches("http://placeholder/")
                        );
                    }
                } else {
                    let updated = rewrite_download_urls(value.take(), base_url);
                    *value = updated;
                }
            }
            serde_json::Value::Object(object)
        }
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(|value| rewrite_download_urls(value, base_url))
                .collect(),
        ),
        value => value,
    }
}

async fn spawn_thread_history_server() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let app = Router::new()
        .route(
            "/conversations.history",
            get(|| async {
                Json(json!({
                    "ok": true,
                    "messages": [
                        {
                            "ts": "1700000000.000001",
                            "user": "U123",
                            "text": "hello from backfill",
                            "reply_count": 2
                        }
                    ]
                }))
            }),
        )
        .route(
            "/conversations.replies",
            get(|Query(query): Query<HashMap<String, String>>| async move {
                assert_eq!(query.get("channel").map(String::as_str), Some("C123"));
                assert_eq!(
                    query.get("ts").map(String::as_str),
                    Some("1700000000.000001")
                );
                Json(json!({
                    "ok": true,
                    "messages": [
                        {
                            "ts": "1700000000.000001",
                            "user": "U123",
                            "text": "hello from backfill"
                        },
                        {
                            "ts": "1700000000.000002",
                            "user": "U456",
                            "text": "first reply",
                            "thread_ts": "1700000000.000001"
                        },
                        {
                            "ts": "1700000000.000003",
                            "user": "U789",
                            "text": "second reply",
                            "thread_ts": "1700000000.000001"
                        }
                    ]
                }))
            }),
        );
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (format!("http://{address}"), handle)
}

async fn spawn_thread_history_server_with_reply_rate_limit() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let reply_attempts = Arc::new(Mutex::new(0usize));
    let app = Router::new()
        .route(
            "/conversations.history",
            get(|| async {
                Json(json!({
                    "ok": true,
                    "messages": [
                        {
                            "ts": "1700000000.000001",
                            "user": "U123",
                            "text": "hello from backfill",
                            "reply_count": 1
                        }
                    ]
                }))
            }),
        )
        .route(
            "/conversations.replies",
            get({
                let reply_attempts = reply_attempts.clone();
                move || {
                    let reply_attempts = reply_attempts.clone();
                    async move {
                        let mut attempts = reply_attempts.lock().await;
                        *attempts += 1;
                        if *attempts == 1 {
                            return (
                                StatusCode::TOO_MANY_REQUESTS,
                                [("retry-after", "0")],
                                Body::empty(),
                            );
                        }

                        (
                            StatusCode::OK,
                            [("content-type", "application/json")],
                            Body::from(
                                json!({
                                    "ok": true,
                                    "messages": [
                                        {
                                            "ts": "1700000000.000001",
                                            "user": "U123",
                                            "text": "hello from backfill"
                                        },
                                        {
                                            "ts": "1700000000.000002",
                                            "user": "U456",
                                            "text": "reply after retry",
                                            "thread_ts": "1700000000.000001"
                                        }
                                    ]
                                })
                                .to_string(),
                            ),
                        )
                    }
                }
            }),
        );
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (format!("http://{address}"), handle)
}

#[derive(Clone, Default)]
struct WorkspaceBackfillState {
    histories: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

async fn spawn_workspace_backfill_server(
    histories: HashMap<String, serde_json::Value>,
) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let state = WorkspaceBackfillState {
        histories: Arc::new(Mutex::new(histories)),
    };
    let app = Router::new()
        .route(
            "/conversations.list",
            get(|| async {
                Json(json!({
                    "ok": true,
                    "channels": [
                        { "id": "C123", "name": "general", "is_archived": false },
                        { "id": "C456", "name": "random", "is_archived": false }
                    ]
                }))
            }),
        )
        .route(
            "/conversations.history",
            get(
                |State(state): State<WorkspaceBackfillState>,
                 Query(query): Query<HashMap<String, String>>| async move {
                    let channel_id = query.get("channel").cloned().expect("channel query param");
                    let payload = state
                        .histories
                        .lock()
                        .await
                        .get(&channel_id)
                        .cloned()
                        .expect("channel history payload");
                    Json(payload)
                },
            ),
        )
        .with_state(state);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (format!("http://{address}"), handle)
}

async fn spawn_workspace_backfill_server_with_history_assertions(
    expected_oldest_by_channel: HashMap<String, String>,
    expected_inclusive: bool,
    histories: HashMap<String, serde_json::Value>,
) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let state = WorkspaceBackfillState {
        histories: Arc::new(Mutex::new(histories)),
    };
    let app = Router::new()
        .route(
            "/conversations.list",
            get(|| async {
                Json(json!({
                    "ok": true,
                    "channels": [
                        { "id": "C123", "name": "general", "is_archived": false },
                        { "id": "C456", "name": "random", "is_archived": false }
                    ]
                }))
            }),
        )
        .route(
            "/conversations.history",
            get(
                move |State(state): State<WorkspaceBackfillState>,
                      Query(query): Query<HashMap<String, String>>| {
                    let expected_oldest_by_channel = expected_oldest_by_channel.clone();
                    async move {
                        let channel_id =
                            query.get("channel").cloned().expect("channel query param");
                        assert_eq!(
                            query.get("oldest").cloned(),
                            expected_oldest_by_channel.get(&channel_id).cloned()
                        );
                        assert_eq!(
                            query.get("inclusive").map(String::as_str),
                            Some(if expected_inclusive { "true" } else { "false" })
                        );
                        let payload = state
                            .histories
                            .lock()
                            .await
                            .get(&channel_id)
                            .cloned()
                            .expect("channel history payload");
                        Json(payload)
                    }
                },
            ),
        )
        .with_state(state);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (format!("http://{address}"), handle)
}
