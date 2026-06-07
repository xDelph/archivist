use super::PrivacyActionResponse;
use crate::{
    ApiConfig,
    auth::{SessionClaims, build_session_token, current_unix_timestamp},
    build_router,
    user_store::{LocalUserStore, SyncedUserRecord},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::json;
use tempfile::tempdir;
use tower::util::ServiceExt;

#[tokio::test]
async fn anonymize_self_marks_current_user_anonymous() {
    let tempdir = tempdir().expect("tempdir");
    seed_user(&tempdir, "U123", false).await;
    let session_token = session_token("U123");

    let response = router(&tempdir)
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/anonymize")
                .header("cookie", format!("arkivist_session={session_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "confirm": true }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: PrivacyActionResponse = serde_json::from_slice(&body).expect("json");
    assert!(payload.ok);

    let user = LocalUserStore::open(tempdir.path().join("synced-users.json"))
        .await
        .expect("user store")
        .find_user("U123")
        .await
        .expect("user");
    assert!(user.is_anonymized);
    assert_eq!(user.display_name.as_deref(), Some("anonymous"));
}

#[tokio::test]
async fn de_anonymize_self_restores_visibility_for_active_users() {
    let tempdir = tempdir().expect("tempdir");
    seed_user(&tempdir, "U123", true).await;
    let session_token = session_token("U123");

    let response = router(&tempdir)
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/de-anonymize")
                .header("cookie", format!("arkivist_session={session_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "confirm": true }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);

    let user = crate::user_store::UserStore::from(
        LocalUserStore::open(tempdir.path().join("synced-users.json"))
            .await
            .expect("user store"),
    )
    .find_user_raw("U123")
    .await
    .expect("user");
    assert!(!user.is_anonymized);
}

#[tokio::test]
async fn de_anonymize_self_rejects_deactivated_users() {
    let tempdir = tempdir().expect("tempdir");
    let store = LocalUserStore::open(tempdir.path().join("synced-users.json"))
        .await
        .expect("user store");
    store
        .upsert_user(SyncedUserRecord {
            slack_user_id: "U123".to_owned(),
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            is_active: false,
            is_anonymized: true,
        })
        .await
        .expect("seed");
    let session_token = session_token("U123");

    let response = router(&tempdir)
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/de-anonymize")
                .header("cookie", format!("arkivist_session={session_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "confirm": true }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

async fn seed_user(tempdir: &tempfile::TempDir, user_id: &str, anonymized: bool) {
    LocalUserStore::open(tempdir.path().join("synced-users.json"))
        .await
        .expect("user store")
        .upsert_user(SyncedUserRecord {
            slack_user_id: user_id.to_owned(),
            display_name: Some("Thomas".to_owned()),
            avatar_url: Some("https://images.example.com/avatar.png".to_owned()),
            is_active: true,
            is_anonymized: anonymized,
        })
        .await
        .expect("seed");
}

fn session_token(user_id: &str) -> String {
    build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: user_id.to_owned(),
            email: Some("thomas@example.com".to_owned()),
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            exp: current_unix_timestamp() + 60,
        },
    )
    .expect("token")
}

async fn router(tempdir: &tempfile::TempDir) -> axum::Router {
    build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: tempdir.path().join("events.jsonl").display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
        slack_team_id: None,
        slack_token_url: None,
        session_secret: Some("session_secret".to_owned()),
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
    })
    .await
    .expect("router")
}
