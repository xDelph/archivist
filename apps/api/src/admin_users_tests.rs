use super::AdminUsersResponse;
use crate::{
    ApiConfig,
    auth::{SessionClaims, build_session_token, current_unix_timestamp},
    build_router,
    user_role_store::{ADMIN_ROLE, LocalUserRoleStore},
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
async fn admin_users_route_requires_admin_role() {
    let tempdir = tempdir().expect("tempdir");
    seed_user(&tempdir, "U123").await;
    let session_token = session_token("U123");

    let response = router(&tempdir)
        .await
        .oneshot(
            Request::builder()
                .uri("/api/admin/users")
                .header("cookie", format!("arkivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_anonymize_and_reactivate_users() {
    let tempdir = tempdir().expect("tempdir");
    seed_user(&tempdir, "UADMIN").await;
    seed_user(&tempdir, "UTARGET").await;
    grant_admin(&tempdir, "UADMIN").await;
    let session_token = session_token("UADMIN");

    let anonymize = router(&tempdir)
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/users/UTARGET/anonymize")
                .header("cookie", format!("arkivist_session={session_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "confirm": true }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(anonymize.status(), StatusCode::OK);

    let user_store = crate::user_store::UserStore::from(
        LocalUserStore::open(tempdir.path().join("synced-users.json"))
            .await
            .expect("user store"),
    );
    let anonymized = user_store.find_user_raw("UTARGET").await.expect("user");
    assert!(anonymized.is_anonymized);

    let deactivate = router(&tempdir)
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/users/UTARGET/deactivate")
                .header("cookie", format!("arkivist_session={session_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "confirm": true }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(deactivate.status(), StatusCode::OK);

    let deactivated = reloaded_user_store(&tempdir)
        .await
        .find_user_raw("UTARGET")
        .await
        .expect("user");
    assert!(!deactivated.is_active);
    assert!(deactivated.is_anonymized);

    let reactivate = router(&tempdir)
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/users/UTARGET/reactivate")
                .header("cookie", format!("arkivist_session={session_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "confirm": true }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(reactivate.status(), StatusCode::OK);

    let restored = reloaded_user_store(&tempdir)
        .await
        .find_user_raw("UTARGET")
        .await
        .expect("user");
    assert!(restored.is_active);
    assert!(!restored.is_anonymized);

    let list = router(&tempdir)
        .await
        .oneshot(
            Request::builder()
                .uri("/api/admin/users")
                .header("cookie", format!("arkivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(list.status(), StatusCode::OK);
    let body = to_bytes(list.into_body(), usize::MAX).await.expect("body");
    let payload: AdminUsersResponse = serde_json::from_slice(&body).expect("json");
    assert!(
        payload
            .users
            .iter()
            .any(|user| user.slack_user_id == "UTARGET")
    );
}

async fn reloaded_user_store(tempdir: &tempfile::TempDir) -> crate::user_store::UserStore {
    crate::user_store::UserStore::from(
        LocalUserStore::open(tempdir.path().join("synced-users.json"))
            .await
            .expect("user store"),
    )
}

async fn seed_user(tempdir: &tempfile::TempDir, user_id: &str) {
    LocalUserStore::open(tempdir.path().join("synced-users.json"))
        .await
        .expect("user store")
        .upsert_user(SyncedUserRecord {
            slack_user_id: user_id.to_owned(),
            display_name: Some(format!("User {user_id}")),
            avatar_url: None,
            is_active: true,
            is_anonymized: false,
        })
        .await
        .expect("seed");
}

async fn grant_admin(tempdir: &tempfile::TempDir, user_id: &str) {
    let role_store = LocalUserRoleStore::open(user_roles_path(tempdir))
        .await
        .expect("role store");
    crate::user_role_store::UserRoleStore::from(role_store)
        .grant_role(user_id, ADMIN_ROLE)
        .await
        .expect("grant admin");
}

fn user_roles_path(tempdir: &tempfile::TempDir) -> String {
    tempdir.path().join("user-roles.json").display().to_string()
}

fn session_token(user_id: &str) -> String {
    build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: user_id.to_owned(),
            email: Some(format!("{user_id}@example.com")),
            display_name: Some("Admin".to_owned()),
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
