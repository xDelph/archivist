use super::{IngestConfig, build_router};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt;

#[test]
fn config_uses_local_defaults() {
    let config = IngestConfig::from_env();
    assert_eq!(config.bind_address(), "127.0.0.1:4001");
    assert_eq!(config.worker_base_url, "http://127.0.0.1:4002");
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
        worker_base_url: "https://worker.archivist.dev".to_owned(),
        qstash_base_url: Some("qstash.upstash.io".to_owned()),
        qstash_token: Some("secret".to_owned()),
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
        "https://worker.archivist.dev/jobs/process_event"
    );
}

#[tokio::test]
async fn slash_command_stubs_supported_commands() {
    let router = build_router(IngestConfig::from_env()).expect("router");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/slack/commands/ask-archivist")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "command=%2Fask-archivist&text=release+status&channel_id=C123&user_id=U123",
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
    assert_eq!(payload["command"], "/ask-archivist");
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
                .body(Body::from("command=%2Fask-archivist"))
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
