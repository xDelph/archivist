use super::{ApiConfig, build_router};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::ACCESS_CONTROL_ALLOW_ORIGIN},
};
use tempfile::tempdir;
use tower::util::ServiceExt;

#[test]
fn config_uses_defaults() {
    let config = ApiConfig::from_env();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 4000);
    assert_eq!(config.event_log_path, "logs/process-events.jsonl");
    assert_eq!(config.slack_client_id, None);
    assert_eq!(config.slack_client_secret, None);
    assert_eq!(config.slack_redirect_uri, None);
    assert_eq!(config.slack_token_url, None);
    assert_eq!(config.session_secret, None);
    assert_eq!(config.auth_store_path, "logs/auth-identities.json");
    assert_eq!(config.synced_users_path, "logs/synced-users.json");
    assert_eq!(config.web_origin, super::LOCAL_DEV_WEB_ORIGIN);
    assert_eq!(config.bind_address(), "127.0.0.1:4000");
}

#[tokio::test]
async fn health_route_reports_workspace_capabilities() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
        slack_token_url: None,
        session_secret: None,
        auth_store_path: tempdir
            .path()
            .join("auth-identities.json")
            .display()
            .to_string(),
        synced_users_path: tempdir
            .path()
            .join("synced-users.json")
            .display()
            .to_string(),
        web_origin: super::LOCAL_DEV_WEB_ORIGIN.to_owned(),
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(payload["service"], "api");
    assert_eq!(payload["workspace_mode"], "single_workspace");
    assert_eq!(payload["repository_mode"], "test_jsonl");
}

#[tokio::test]
async fn auth_me_includes_cors_headers_for_the_frontend_origin() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
        slack_token_url: None,
        session_secret: Some("test-secret".to_owned()),
        auth_store_path: tempdir
            .path()
            .join("auth-identities.json")
            .display()
            .to_string(),
        synced_users_path: tempdir
            .path()
            .join("synced-users.json")
            .display()
            .to_string(),
        web_origin: super::LOCAL_DEV_WEB_ORIGIN.to_owned(),
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri("/api/auth/me")
            .header("origin", super::LOCAL_DEV_WEB_ORIGIN)
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&super::LOCAL_DEV_WEB_ORIGIN.parse().expect("origin"))
    );
}

#[tokio::test]
async fn auth_me_includes_cors_headers_for_localhost_frontend_origin() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
        slack_token_url: None,
        session_secret: Some("test-secret".to_owned()),
        auth_store_path: tempdir
            .path()
            .join("auth-identities.json")
            .display()
            .to_string(),
        synced_users_path: tempdir
            .path()
            .join("synced-users.json")
            .display()
            .to_string(),
        web_origin: super::LOCAL_DEV_WEB_ORIGIN.to_owned(),
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri("/api/auth/me")
            .header("origin", super::LOCALHOST_WEB_ORIGIN)
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&super::LOCALHOST_WEB_ORIGIN.parse().expect("origin"))
    );
}
