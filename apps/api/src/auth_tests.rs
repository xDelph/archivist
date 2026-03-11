use super::{
    SLACK_ISSUER, SessionClaims, SlackAuthConfig, build_authorize_url, current_unix_timestamp,
    parse_identity_claims,
};
use crate::ApiConfig;
use axum::{
    Json,
    body::{Body, Bytes},
    http::{Request, StatusCode},
    routing::post,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use tempfile::{TempDir, tempdir};
use tokio::{net::TcpListener, task::JoinHandle};
use tower::util::ServiceExt;

#[path = "auth_callback_tests.rs"]
mod callback;
#[path = "auth_session_tests.rs"]
mod session;

#[test]
fn authorize_url_omits_team_when_workspace_is_not_configured() {
    let url = build_authorize_url(&SlackAuthConfig {
        client_id: Some("client_123".to_owned()),
        client_secret: None,
        redirect_uri: Some("https://archivist.dev/api/auth/slack/callback".to_owned()),
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
            token_url: None,
        }),
        None
    );
    assert_eq!(
        build_authorize_url(&SlackAuthConfig {
            client_id: Some("client_123".to_owned()),
            client_secret: None,
            redirect_uri: Some("   ".to_owned()),
            token_url: None,
        }),
        None
    );
}

#[test]
fn parse_identity_claims_reads_slack_claims() {
    let token = sample_id_token("client_123", "U123", current_unix_timestamp() + 60);
    let claims = parse_identity_claims(&token).expect("claims");

    assert_eq!(claims.iss, SLACK_ISSUER);
    assert_eq!(claims.aud, "client_123");
    assert_eq!(claims.slack_user_id, "U123");
}

#[tokio::test]
async fn slack_start_redirects_to_slack_oidc() {
    let tempdir = tempdir().expect("tempdir");
    let response = crate::build_router(config_with_defaults(&tempdir))
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
    assert!(!location.contains("&team="));
}

#[tokio::test]
async fn slack_start_rejects_missing_config() {
    let tempdir = tempdir().expect("tempdir");
    let mut config = config_with_defaults(&tempdir);
    config.slack_client_id = None;
    config.slack_client_secret = None;
    config.slack_redirect_uri = None;
    let response = crate::build_router(config)
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

fn config_with_defaults(tempdir: &TempDir) -> ApiConfig {
    ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: tempdir.path().join("events.jsonl").display().to_string(),
        slack_client_id: Some("client_123".to_owned()),
        slack_client_secret: Some("secret".to_owned()),
        slack_redirect_uri: Some("https://archivist.dev/api/auth/slack/callback".to_owned()),
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
        web_origin: crate::LOCAL_DEV_WEB_ORIGIN.to_owned(),
    }
}

async fn seed_synced_user(tempdir: &TempDir, user_id: &str, is_active: bool) {
    crate::user_store::LocalUserStore::open(tempdir.path().join("synced-users.json"))
        .await
        .expect("user store")
        .upsert_user(crate::user_store::SyncedUserRecord {
            slack_user_id: user_id.to_owned(),
            display_name: Some("Thomas".to_owned()),
            avatar_url: Some("https://images.example.com/avatar.png".to_owned()),
            is_active,
        })
        .await
        .expect("seed synced user");
}

fn sample_id_token(client_id: &str, user_id: &str, exp: i64) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
    let claims = URL_SAFE_NO_PAD.encode(
        json!({
            "iss": SLACK_ISSUER,
            "aud": client_id,
            "exp": exp,
            "https://slack.com/user_id": user_id,
            "email": "thomas@example.com",
            "name": "Thomas",
            "picture": "https://images.example.com/avatar.png"
        })
        .to_string(),
    );

    format!("{header}.{claims}.signature")
}

fn build_session_token(session_secret: &str, claims: &SessionClaims) -> String {
    type HmacSha256 = Hmac<Sha256>;

    let payload = serde_json::to_vec(claims).expect("session json");
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
    let mut mac = HmacSha256::new_from_slice(session_secret.as_bytes()).expect("session secret");
    mac.update(payload_b64.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    format!("{payload_b64}.{signature}")
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
