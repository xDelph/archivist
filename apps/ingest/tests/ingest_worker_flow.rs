use axum::Router;
use db::JsonlEventStore;
use ingest::{IngestConfig, build_router as build_ingest_router};
use serde_json::json;
use tempfile::tempdir;
use tokio::{net::TcpListener, task::JoinHandle};
use worker::{WorkerConfig, build_router as build_worker_router};

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
