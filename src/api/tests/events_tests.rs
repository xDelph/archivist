use std::time::{SystemTime, UNIX_EPOCH};

use vercel_runtime::{Body, Request, StatusCode};

use crate::api::events::process;
use crate::db::{InMemoryRepository, Repository};
use crate::slack::signature::compute_signature;

const SECRET: &str = "test_signing_secret";

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Build a correctly signed POST request.
fn signed_request(body: &str) -> Request {
    let now = now_secs();
    let sig = compute_signature(SECRET, now, body.as_bytes());
    http::Request::builder()
        .method("POST")
        .uri("/api/slack/events")
        .header("x-slack-request-timestamp", now.to_string())
        .header("x-slack-signature", sig)
        .header("content-type", "application/json")
        .body(Body::Text(body.to_owned()))
        .unwrap()
}

/// Build a request with a deliberately bad signature.
fn unsigned_request(body: &str) -> Request {
    http::Request::builder()
        .method("POST")
        .uri("/api/slack/events")
        .header("x-slack-request-timestamp", "1700000000")
        .header("x-slack-signature", "v0=badbadbadbad")
        .body(Body::Text(body.to_owned()))
        .unwrap()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_missing_signature_returns_401() {
    let repo = InMemoryRepository::default();
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/slack/events")
        .body(Body::Text("{}".into()))
        .unwrap();
    let resp = process(&repo, SECRET, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_signature_returns_401() {
    let repo = InMemoryRepository::default();
    let resp = process(&repo, SECRET, unsigned_request("{}"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_url_verification_returns_challenge() {
    let repo = InMemoryRepository::default();
    let body = r#"{"type":"url_verification","challenge":"test_challenge_xyz"}"#;
    let resp = process(&repo, SECRET, signed_request(body)).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let text = match resp.into_body() {
        Body::Text(s) => s,
        other => panic!("expected Text body, got {other:?}"),
    };
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["challenge"], "test_challenge_xyz");
}

#[tokio::test]
async fn test_valid_message_event_returns_200() {
    let repo = InMemoryRepository::default();
    let body = r#"{
        "type": "event_callback",
        "team_id": "T001",
        "api_app_id": "A001",
        "event_id": "Ev999",
        "event_time": 1700000000,
        "event": {
            "type": "message",
            "channel": "C001",
            "user": "U001",
            "text": "hi",
            "ts": "1700000000.000100"
        }
    }"#;
    let resp = process(&repo, SECRET, signed_request(body)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(repo.event_exists("Ev999").await.unwrap());
}

#[tokio::test]
async fn test_duplicate_event_returns_200_and_is_ignored() {
    let repo = InMemoryRepository::default();
    let body = r#"{
        "type": "event_callback",
        "team_id": "T001",
        "api_app_id": "A001",
        "event_id": "Ev998",
        "event_time": 1700000000,
        "event": {
            "type": "message",
            "channel": "C001",
            "user": "U001",
            "text": "hi",
            "ts": "1700000000.000200"
        }
    }"#;
    process(&repo, SECRET, signed_request(body)).await.unwrap();
    let resp = process(&repo, SECRET, signed_request(body)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(repo.messages.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_unknown_envelope_type_returns_200() {
    let repo = InMemoryRepository::default();
    let body = r#"{"type":"app_rate_limited","team_id":"T001"}"#;
    let resp = process(&repo, SECRET, signed_request(body)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
