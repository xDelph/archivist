use crate::{WorkerConfig, build_router};
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
    assert_eq!(config.slack_api_base_url, "https://slack.com/api");
    assert_eq!(config.openrouter_base_url, "https://openrouter.ai/api/v1");
    assert_eq!(config.openrouter_api_key, None);
    assert_eq!(config.openrouter_model, None);
    assert_eq!(config.slack_user_token, None);
    assert_eq!(config.r2_account_id, None);
    assert_eq!(config.r2_access_key_id, None);
    assert_eq!(config.r2_secret_access_key, None);
    assert_eq!(config.r2_bucket, None);
    assert_eq!(config.r2_public_url, None);
    assert_eq!(config.r2_endpoint_url, None);
    assert_eq!(config.r2_key_prefix, None);
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
    let payload = ProcessEventJob {
        event_id: "evt_1".to_owned(),
        event_time: 1,
        received_at: 2,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U123".to_owned()),
            text: Some("hello".to_owned()),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            files: vec![],
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
    assert_eq!(health_payload["tracked_files"], 0);
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
    let payload = ProcessEventJob {
        event_id: "evt_reaction".to_owned(),
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
    assert_eq!(health_payload["tracked_files"], 0);
}

#[tokio::test]
async fn file_share_messages_are_counted_in_health() {
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
    let payload = ProcessEventJob {
        event_id: "evt_file".to_owned(),
        event_time: 1,
        received_at: 2,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U123".to_owned()),
            text: Some("uploaded brief".to_owned()),
            ts: "1700000000.000002".to_owned(),
            thread_ts: None,
            files: vec![domain::SharedFile {
                id: "F123".to_owned(),
                name: "brief.pdf".to_owned(),
                mimetype: Some("application/pdf".to_owned()),
                permalink: Some("https://files.example.com/brief.pdf".to_owned()),
                size: Some(42),
            }],
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
    assert_eq!(health_payload["tracked_messages"], 1);
    assert_eq!(health_payload["tracked_files"], 1);
}

#[tokio::test]
async fn refresh_thread_summaries_job_reports_refreshed_threads() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_root".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("root message".to_owned()),
                ts: "1700000000.000001".to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("insert root");
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
                .uri("/jobs/refresh_thread_summaries")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(payload["ok"], true);
    assert_eq!(payload["refreshed"], 1);
}

#[tokio::test]
async fn local_worker_with_signing_keys_still_accepts_unsigned_refresh_jobs() {
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
            current_signing_key: Some("current".to_owned()),
            next_signing_key: Some("next".to_owned()),
        },
    )
    .expect("router");

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/refresh_thread_summaries")
                .body(Body::from("{}"))
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
    assert_eq!(health_payload["queue_signature_verification"], false);
}
