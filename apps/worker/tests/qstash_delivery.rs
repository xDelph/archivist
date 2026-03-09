use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::Engine;
use db::JsonlEventStore;
use domain::{ChannelKind, EventPayload, ProcessEventJob};
use hmac::{Hmac, Mac};
use queue::UPSTASH_SIGNATURE_HEADER;
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tower::util::ServiceExt;
use worker::{WorkerConfig, build_router};

const CURRENT_SIGNING_KEY: &str = "current_signing_key";
const NEXT_SIGNING_KEY: &str = "next_signing_key";
const WORKER_BASE_URL: &str = "https://archivist.example.com";

#[tokio::test]
async fn process_event_rejects_missing_signature_when_verification_is_enabled() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let router = build_router(store, signed_worker_config(&log_path)).expect("router");
    let payload = serde_json::to_vec(&sample_job()).expect("payload");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/process_event")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(payload["error"], "missing_qstash_signature");
}

#[tokio::test]
async fn process_event_accepts_valid_qstash_signature() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let router = build_router(store, signed_worker_config(&log_path)).expect("router");
    let job = sample_job();
    let body = serde_json::to_vec(&job).expect("payload");
    let signature = sign_qstash_request(
        CURRENT_SIGNING_KEY,
        &body,
        &format!("{WORKER_BASE_URL}/jobs/process_event"),
    );

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/process_event")
                .header("content-type", "application/json")
                .header(UPSTASH_SIGNATURE_HEADER, signature)
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
    assert_eq!(health_payload["queue_signature_verification"], true);
    assert_eq!(health_payload["tracked_events"], 1);
}

#[tokio::test]
async fn heartbeat_accepts_next_signing_key() {
    let tempdir = tempdir().expect("tempdir");
    let log_path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&log_path).await.expect("store");
    let router = build_router(store, signed_worker_config(&log_path)).expect("router");
    let body = br#"{}"#;
    let signature = sign_qstash_request(
        NEXT_SIGNING_KEY,
        body,
        &format!("{WORKER_BASE_URL}/jobs/heartbeat"),
    );

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/heartbeat")
                .header("content-type", "application/json")
                .header(UPSTASH_SIGNATURE_HEADER, signature)
                .body(Body::from(body.to_vec()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

fn sample_job() -> ProcessEventJob {
    ProcessEventJob {
        event_id: "evt_signed".to_owned(),
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
            files: vec![],
        },
    }
}

fn signed_worker_config(log_path: &std::path::Path) -> WorkerConfig {
    WorkerConfig {
        host: "127.0.0.1".to_owned(),
        port: 4002,
        event_log_path: log_path.display().to_string(),
        worker_base_url: WORKER_BASE_URL.to_owned(),
        slack_api_base_url: "https://slack.com/api".to_owned(),
        slack_bot_token: None,
        current_signing_key: Some(CURRENT_SIGNING_KEY.to_owned()),
        next_signing_key: Some(NEXT_SIGNING_KEY.to_owned()),
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
