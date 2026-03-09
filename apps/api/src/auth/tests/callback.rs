use super::*;
use crate::auth::{
    CallbackError, SlackAuthConfig, current_unix_timestamp, exchange_code_for_identity,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};

#[tokio::test]
async fn slack_callback_rejects_missing_code() {
    let tempdir = tempdir().expect("tempdir");
    let response = crate::build_router(config_with_defaults(&tempdir))
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/slack/callback")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn slack_callback_exchanges_code_and_validates_identity() {
    let token_server = spawn_token_server(sample_id_token(
        "client_123",
        "T123",
        "U123",
        current_unix_timestamp() + 60,
    ))
    .await;
    let identity = exchange_code_for_identity(
        &SlackAuthConfig {
            client_id: Some("client_123".to_owned()),
            client_secret: Some("secret".to_owned()),
            redirect_uri: Some("https://archivist.dev/api/auth/slack/callback".to_owned()),
            workspace_id: Some("T123".to_owned()),
            token_url: Some(format!("{}/token", token_server.0)),
        },
        "code_123",
    )
    .await
    .expect("identity");

    token_server.1.abort();

    assert_eq!(identity.slack_user_id, "U123");
    assert_eq!(identity.team_id, "T123");
    assert_eq!(identity.display_name.as_deref(), Some("Thomas"));
}

#[tokio::test]
async fn slack_callback_persists_identity_to_the_local_store() {
    let tempdir = tempdir().expect("tempdir");
    seed_synced_user(&tempdir, "U123", true).await;
    let auth_store_path = tempdir.path().join("auth-identities.json");
    let token_server = spawn_token_server(sample_id_token(
        "client_123",
        "T123",
        "U123",
        current_unix_timestamp() + 60,
    ))
    .await;
    let mut config = config_with_defaults(&tempdir);
    config.slack_token_url = Some(format!("{}/token", token_server.0));
    config.session_secret = Some("session_secret".to_owned());

    let response = crate::build_router(config)
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/slack/callback?code=code_123")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    token_server.1.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let set_cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .expect("set-cookie");
    assert!(set_cookie.contains("archivist_session="));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));

    let store = crate::auth_store::LocalAuthStore::open(&auth_store_path)
        .await
        .expect("reopened auth store");
    let identities = store.identities().await;

    assert_eq!(identities.len(), 1);
    assert_eq!(identities[0].slack_user_id, "U123");
    assert_eq!(identities[0].team_id, "T123");
    assert_eq!(identities[0].display_name.as_deref(), Some("Thomas"));
}

#[tokio::test]
async fn slack_callback_rejects_users_missing_from_the_synced_store() {
    let tempdir = tempdir().expect("tempdir");
    let token_server = spawn_token_server(sample_id_token(
        "client_123",
        "T123",
        "U123",
        current_unix_timestamp() + 60,
    ))
    .await;
    let mut config = config_with_defaults(&tempdir);
    config.slack_token_url = Some(format!("{}/token", token_server.0));

    let response = crate::build_router(config)
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/slack/callback?code=code_123")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    token_server.1.abort();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn slack_callback_rejects_inactive_synced_users() {
    let tempdir = tempdir().expect("tempdir");
    seed_synced_user(&tempdir, "U123", false).await;
    let token_server = spawn_token_server(sample_id_token(
        "client_123",
        "T123",
        "U123",
        current_unix_timestamp() + 60,
    ))
    .await;
    let mut config = config_with_defaults(&tempdir);
    config.slack_token_url = Some(format!("{}/token", token_server.0));

    let response = crate::build_router(config)
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/slack/callback?code=code_123")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    token_server.1.abort();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn slack_callback_requires_session_config_to_complete_sign_in() {
    let tempdir = tempdir().expect("tempdir");
    seed_synced_user(&tempdir, "U123", true).await;
    let token_server = spawn_token_server(sample_id_token(
        "client_123",
        "T123",
        "U123",
        current_unix_timestamp() + 60,
    ))
    .await;
    let mut config = config_with_defaults(&tempdir);
    config.slack_token_url = Some(format!("{}/token", token_server.0));

    let response = crate::build_router(config)
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/slack/callback?code=code_123")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    token_server.1.abort();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn slack_callback_rejects_workspace_mismatch() {
    let token_server = spawn_token_server(sample_id_token(
        "client_123",
        "T999",
        "U123",
        current_unix_timestamp() + 60,
    ))
    .await;
    let result = exchange_code_for_identity(
        &SlackAuthConfig {
            client_id: Some("client_123".to_owned()),
            client_secret: Some("secret".to_owned()),
            redirect_uri: Some("https://archivist.dev/api/auth/slack/callback".to_owned()),
            workspace_id: Some("T123".to_owned()),
            token_url: Some(format!("{}/token", token_server.0)),
        },
        "code_123",
    )
    .await;

    token_server.1.abort();

    assert!(matches!(result, Err(CallbackError::WorkspaceMismatch)));
}
