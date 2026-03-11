use crate::{WorkerConfig, build_router};
use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::{Path, State},
    http::{HeaderMap, Request, StatusCode},
    response::IntoResponse,
    routing::{get, head, put},
};
use db::JsonlEventStore;
use serde_json::json;
use std::{collections::HashSet, sync::Arc};
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
async fn archive_file_rejects_missing_user_token() {
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
            slack_user_token: None,
            r2_account_id: Some("acct".to_owned()),
            r2_access_key_id: Some("key".to_owned()),
            r2_secret_access_key: Some("secret".to_owned()),
            r2_bucket: Some("bucket".to_owned()),
            r2_public_url: Some("https://files.example.com".to_owned()),
            r2_endpoint_url: Some("https://r2.example.com".to_owned()),
            r2_key_prefix: Some("T123".to_owned()),
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/archive_file")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "channel_id": "C123",
                        "message_ts": "1700000000.000001",
                        "file_id": "F123",
                        "filename": "brief.pdf",
                        "download_url": "https://files.slack.com/brief.pdf"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn archive_file_rejects_missing_r2_config() {
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
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: None,
            r2_access_key_id: None,
            r2_secret_access_key: None,
            r2_bucket: None,
            r2_public_url: None,
            r2_endpoint_url: None,
            r2_key_prefix: Some("T123".to_owned()),
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/archive_file")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "channel_id": "C123",
                        "message_ts": "1700000000.000001",
                        "file_id": "F123",
                        "filename": "brief.pdf",
                        "download_url": "https://files.slack.com/brief.pdf"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn archive_file_downloads_from_slack_and_uploads_to_r2() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (base_url, state, handle) = spawn_archive_servers().await;
    let router = build_router(
        store,
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url: "https://slack.com/api".to_owned(),
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: Some("acct".to_owned()),
            r2_access_key_id: Some("key".to_owned()),
            r2_secret_access_key: Some("secret".to_owned()),
            r2_bucket: Some("bucket".to_owned()),
            r2_public_url: Some(format!("{base_url}/public")),
            r2_endpoint_url: Some(base_url.clone()),
            r2_key_prefix: Some("T123".to_owned()),
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/archive_file")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "channel_id": "C123",
                        "message_ts": "1700000000.000001",
                        "file_id": "F123",
                        "filename": "brief v1.pdf",
                        "download_url": format!("{base_url}/download/file"),
                        "mimetype": "application/pdf"
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
    assert_eq!(payload["bytes_uploaded"], 11);
    assert_eq!(payload["storage_key"], "T123/C123/F123/brief v1.pdf");
    assert_eq!(
        payload["public_url"],
        format!("{base_url}/public/T123/C123/F123/brief v1.pdf")
    );

    let capture = state.upload.lock().await.clone().expect("upload capture");
    assert_eq!(capture.path, "bucket/T123/C123/F123/brief v1.pdf");
    assert_eq!(capture.body, b"hello world");
    assert_eq!(capture.content_type.as_deref(), Some("application/pdf"));
}

#[tokio::test]
async fn archive_file_reuses_existing_r2_object() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let (base_url, state, handle) = spawn_archive_servers().await;
    state
        .existing_public_keys
        .lock()
        .await
        .insert("T123/C123/F123/brief.pdf".to_owned());
    let router = build_router(
        store,
        WorkerConfig {
            host: "127.0.0.1".to_owned(),
            port: 4002,
            event_log_path: log_path.display().to_string(),
            worker_base_url: "http://127.0.0.1:4002".to_owned(),
            slack_api_base_url: "https://slack.com/api".to_owned(),
            slack_user_token: Some("xoxp-test".to_owned()),
            r2_account_id: Some("acct".to_owned()),
            r2_access_key_id: Some("key".to_owned()),
            r2_secret_access_key: Some("secret".to_owned()),
            r2_bucket: Some("bucket".to_owned()),
            r2_public_url: Some(format!("{base_url}/public")),
            r2_endpoint_url: Some(base_url.clone()),
            r2_key_prefix: Some("T123".to_owned()),
            current_signing_key: None,
            next_signing_key: None,
        },
    )
    .expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/archive_file")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "channel_id": "C123",
                        "message_ts": "1700000000.000001",
                        "file_id": "F123",
                        "filename": "brief.pdf",
                        "download_url": format!("{base_url}/download/file"),
                        "mimetype": "application/pdf"
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
    assert_eq!(payload["bytes_uploaded"], 0);
    assert_eq!(payload["storage_key"], "T123/C123/F123/brief.pdf");
    assert_eq!(
        payload["public_url"],
        format!("{base_url}/public/T123/C123/F123/brief.pdf")
    );
    assert_eq!(state.upload.lock().await.clone(), None);
}

async fn spawn_archive_servers() -> (String, MockStorageState, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let state = MockStorageState::default();
    let app = Router::new()
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

    (format!("http://{address}"), state, handle)
}
