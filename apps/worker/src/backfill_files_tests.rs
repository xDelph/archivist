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

#[derive(Debug, Clone, PartialEq, Eq)]
struct UploadCapture {
    path: String,
    body: Vec<u8>,
    content_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct MockStorageState {
    uploads: Arc<Mutex<Vec<UploadCapture>>>,
    existing_public_keys: Arc<Mutex<HashSet<String>>>,
}

#[tokio::test]
async fn backfill_files_without_channel_id_archives_missing_files_across_workspace() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    seed_file_message(&store, "evt-1", "C123", "1700000000.000001", "F123").await;
    seed_file_message(&store, "evt-2", "C456", "1700000001.000001", "F456").await;
    let (slack_api_base_url, state, handle) = spawn_file_backfill_server(HashMap::from([
        (
            ("C123".to_owned(), "1700000000.000001".to_owned()),
            json!({
                "ok": true,
                "messages": [
                    {
                        "ts": "1700000000.000001",
                        "files": [
                            {
                                "id": "F123",
                                "name": "brief.pdf",
                                "mimetype": "application/pdf",
                                "url_private_download": "http://placeholder/download/file",
                                "size": 42
                            }
                        ]
                    }
                ]
            }),
        ),
        (
            ("C456".to_owned(), "1700000001.000001".to_owned()),
            json!({
                "ok": true,
                "messages": [
                    {
                        "ts": "1700000001.000001",
                        "files": [
                            {
                                "id": "F456",
                                "name": "brief.pdf",
                                "mimetype": "application/pdf",
                                "url_private_download": "http://placeholder/download/file",
                                "size": 42
                            }
                        ]
                    }
                ]
            }),
        ),
    ]))
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
                .uri("/jobs/backfill_files")
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
    assert_eq!(payload["archived"], 2);
    assert_eq!(payload["skipped"], 0);
    assert_eq!(payload["failed"], 0);

    let files = store.files().await;
    assert_eq!(files.len(), 2);
    let expected_c123 = format!("{slack_api_base_url}/public//archive/C123/F123/brief.pdf");
    let expected_c456 = format!("{slack_api_base_url}/public//archive/C456/F456/brief.pdf");
    assert_eq!(files[0].permalink.as_deref(), Some(expected_c123.as_str()));
    assert_eq!(files[1].permalink.as_deref(), Some(expected_c456.as_str()));

    let uploads = state.uploads.lock().await.clone();
    assert_eq!(uploads.len(), 2);
}

#[tokio::test]
async fn backfill_files_with_channel_id_only_archives_requested_channel() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    seed_file_message(&store, "evt-1", "C123", "1700000000.000001", "F123").await;
    seed_file_message(&store, "evt-2", "C456", "1700000001.000001", "F456").await;
    let (slack_api_base_url, state, handle) = spawn_file_backfill_server(HashMap::from([(
        ("C123".to_owned(), "1700000000.000001".to_owned()),
        json!({
            "ok": true,
            "messages": [
                {
                    "ts": "1700000000.000001",
                    "files": [
                        {
                            "id": "F123",
                            "name": "brief.pdf",
                            "mimetype": "application/pdf",
                            "url_private_download": "http://placeholder/download/file",
                            "size": 42
                        }
                    ]
                }
            ]
        }),
    )]))
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
                .uri("/jobs/backfill_files")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "channel_id": "C123" }).to_string()))
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
    assert_eq!(payload["archived"], 1);
    assert_eq!(payload["skipped"], 0);
    assert_eq!(payload["failed"], 0);

    let files = store.files().await;
    let expected_c123 = format!("{slack_api_base_url}/public//archive/C123/F123/brief.pdf");
    assert_eq!(files[0].permalink.as_deref(), Some(expected_c123.as_str()));
    assert_eq!(
        files[1].permalink.as_deref(),
        Some("https://files.example.com/brief.pdf")
    );

    let uploads = state.uploads.lock().await.clone();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].path, "bucket/archive/C123/F123/brief.pdf");
}

async fn seed_file_message(
    store: &JsonlEventStore,
    event_id: &str,
    channel_id: &str,
    ts: &str,
    file_id: &str,
) {
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
                files: vec![SharedFile {
                    id: file_id.to_owned(),
                    name: "brief.pdf".to_owned(),
                    mimetype: Some("application/pdf".to_owned()),
                    permalink: Some("https://files.example.com/brief.pdf".to_owned()),
                    size: Some(42),
                }],
            },
        })
        .await
        .expect("insert file message");
}

async fn spawn_file_backfill_server(
    histories: HashMap<(String, String), serde_json::Value>,
) -> (String, MockStorageState, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let base_url = format!("http://{address}");
    let state = MockStorageState::default();
    let histories = histories
        .into_iter()
        .map(|(key, value)| (key, rewrite_download_urls(value, &base_url)))
        .collect::<HashMap<_, _>>();
    let app = Router::new()
        .route(
            "/conversations.history",
            get(move |Query(query): Query<HashMap<String, String>>| {
                let histories = histories.clone();
                async move {
                    assert_eq!(query.get("inclusive").map(String::as_str), Some("true"));
                    assert_eq!(query.get("limit").map(String::as_str), Some("1"));
                    let channel_id = query.get("channel").cloned().expect("channel query param");
                    let message_ts = query.get("oldest").cloned().expect("oldest query param");
                    assert_eq!(query.get("latest").cloned(), Some(message_ts.clone()));
                    Json(
                        histories
                            .get(&(channel_id, message_ts))
                            .cloned()
                            .expect("history payload"),
                    )
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
                    state.uploads.lock().await.push(UploadCapture {
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
