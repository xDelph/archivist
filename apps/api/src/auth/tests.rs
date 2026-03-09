use super::{
    CallbackError, SLACK_ISSUER, SlackAuthConfig, build_authorize_url, current_unix_timestamp,
    exchange_code_for_identity, parse_identity_claims,
};
use crate::{ApiConfig, build_router};
use axum::{
    Json,
    body::{Body, Bytes},
    http::{Request, StatusCode},
    routing::post,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;
use tempfile::tempdir;
use tokio::{net::TcpListener, task::JoinHandle};
use tower::util::ServiceExt;

#[test]
fn authorize_url_omits_team_when_workspace_is_not_configured() {
    let url = build_authorize_url(&SlackAuthConfig {
        client_id: Some("client_123".to_owned()),
        client_secret: None,
        redirect_uri: Some("https://archivist.dev/api/auth/slack/callback".to_owned()),
        workspace_id: None,
        token_url: None,
    })
    .expect("authorize url");

    assert!(url.starts_with("https://slack.com/openid/connect/authorize?"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("client_id=client_123"));
    assert!(url.contains("scope=openid%20profile%20email"));
    assert!(
        url.contains("redirect_uri=https%3A%2F%2Farchivist.dev%2Fapi%2Fauth%2Fslack%2Fcallback")
    );
    assert!(!url.contains("&team="));
}

#[test]
fn authorize_url_requires_client_id_and_redirect_uri() {
    assert_eq!(
        build_authorize_url(&SlackAuthConfig {
            client_id: None,
            client_secret: None,
            redirect_uri: Some("https://archivist.dev/callback".to_owned()),
            workspace_id: None,
            token_url: None,
        }),
        None
    );
    assert_eq!(
        build_authorize_url(&SlackAuthConfig {
            client_id: Some("client_123".to_owned()),
            client_secret: None,
            redirect_uri: Some("   ".to_owned()),
            workspace_id: None,
            token_url: None,
        }),
        None
    );
}

#[test]
fn parse_identity_claims_reads_slack_claims() {
    let token = sample_id_token("client_123", "T123", "U123", current_unix_timestamp() + 60);
    let claims = parse_identity_claims(&token).expect("claims");

    assert_eq!(claims.iss, SLACK_ISSUER);
    assert_eq!(claims.aud, "client_123");
    assert_eq!(claims.team_id, "T123");
    assert_eq!(claims.slack_user_id, "U123");
}

#[tokio::test]
async fn slack_start_redirects_to_slack_oidc() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: Some("client_123".to_owned()),
        slack_client_secret: Some("secret".to_owned()),
        slack_redirect_uri: Some("https://archivist.dev/api/auth/slack/callback".to_owned()),
        slack_workspace_id: Some("T123".to_owned()),
        slack_token_url: None,
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri("/api/auth/slack/start")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .expect("location header");

    assert!(location.contains("response_type=code"));
    assert!(location.contains("client_id=client_123"));
    assert!(location.contains("scope=openid%20profile%20email"));
    assert!(location.contains("team=T123"));
}

#[tokio::test]
async fn slack_start_rejects_missing_config() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
        slack_workspace_id: None,
        slack_token_url: None,
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri("/api/auth/slack/start")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn slack_callback_rejects_missing_code() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: Some("client_123".to_owned()),
        slack_client_secret: Some("secret".to_owned()),
        slack_redirect_uri: Some("https://archivist.dev/api/auth/slack/callback".to_owned()),
        slack_workspace_id: Some("T123".to_owned()),
        slack_token_url: None,
    })
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

fn sample_id_token(client_id: &str, team_id: &str, user_id: &str, exp: i64) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
    let claims = URL_SAFE_NO_PAD.encode(
        json!({
            "iss": SLACK_ISSUER,
            "aud": client_id,
            "exp": exp,
            "https://slack.com/user_id": user_id,
            "https://slack.com/team_id": team_id,
            "email": "thomas@example.com",
            "name": "Thomas",
            "picture": "https://images.example.com/avatar.png"
        })
        .to_string(),
    );

    format!("{header}.{claims}.signature")
}

async fn spawn_token_server(id_token: String) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("local addr");
    let app = axum::Router::new().route(
        "/token",
        post({
            let id_token = id_token.clone();
            move |_body: Bytes| async move { Json(json!({ "ok": true, "id_token": id_token })) }
        }),
    );
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    (format!("http://{address}"), handle)
}
