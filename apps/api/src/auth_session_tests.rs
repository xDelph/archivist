use super::*;
use crate::{
    AppState,
    auth::{SessionClaims, SlackAuthConfig, current_unix_timestamp, me},
};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
};
use tower::util::ServiceExt;

#[tokio::test]
async fn me_returns_the_current_user_from_a_valid_session_cookie() {
    let tempdir = tempdir().expect("tempdir");
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            email: Some("thomas@example.com".to_owned()),
            display_name: Some("Thomas".to_owned()),
            avatar_url: Some("https://images.example.com/avatar.png".to_owned()),
            exp: current_unix_timestamp() + 60,
        },
    );
    let mut config = config_with_defaults(&tempdir);
    config.session_secret = Some("session_secret".to_owned());

    let response = crate::build_router(config)
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn me_returns_roles_for_the_current_user() {
    let tempdir = tempdir().expect("tempdir");
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            email: Some("thomas@example.com".to_owned()),
            display_name: Some("Thomas".to_owned()),
            avatar_url: Some("https://images.example.com/avatar.png".to_owned()),
            exp: current_unix_timestamp() + 60,
        },
    );
    let mut config = config_with_defaults(&tempdir);
    config.session_secret = Some("session_secret".to_owned());
    crate::user_role_store::LocalUserRoleStore::open(tempdir.path().join("user-roles.json"))
        .await
        .expect("user role store")
        .grant_role("U123", crate::user_role_store::ADMIN_ROLE)
        .await
        .expect("grant role");

    let response = crate::build_router(config)
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let payload = serde_json::from_slice::<serde_json::Value>(&body).expect("json payload");

    assert_eq!(payload["user"]["roles"], serde_json::json!(["admin"]));
}

#[tokio::test]
async fn me_rejects_missing_sessions() {
    let tempdir = tempdir().expect("tempdir");
    let state = AppState {
        store: db::JsonlEventStore::open(tempdir.path().join("events.jsonl"))
            .await
            .expect("store")
            .into(),
        slack_auth: SlackAuthConfig {
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            team_id: None,
            token_url: None,
        },
        web_origin: crate::LOCAL_DEV_WEB_ORIGIN.to_owned(),
        session_secret: Some("session_secret".to_owned()),
        auth_store: crate::auth_store::LocalAuthStore::open(
            tempdir.path().join("auth-identities.json"),
        )
        .await
        .expect("auth store")
        .into(),
        user_store: crate::user_store::LocalUserStore::open(
            tempdir.path().join("synced-users.json"),
        )
        .await
        .expect("user store")
        .into(),
        user_role_store: crate::user_role_store::LocalUserRoleStore::open(
            tempdir.path().join("user-roles.json"),
        )
        .await
        .expect("user role store")
        .into(),
        highlight_store: crate::highlight_store::LocalHighlightStore::open(
            tempdir.path().join("highlighted-threads.json"),
        )
        .await
        .expect("highlight store")
        .into(),
        saved_store: crate::saved_store::LocalSavedItemStore::open(
            tempdir.path().join("saved-items.json"),
        )
        .await
        .expect("saved store")
        .into(),
        analytics_store: crate::analytics_store::LocalAnalyticsStore::open(
            tempdir.path().join("analytics-events.json"),
        )
        .await
        .expect("analytics store")
        .into(),
    };

    let result = me(State(state), axum::http::HeaderMap::new()).await;

    assert!(matches!(result, Err((StatusCode::UNAUTHORIZED, _))));
}

#[tokio::test]
async fn me_rejects_expired_sessions() {
    let tempdir = tempdir().expect("tempdir");
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            email: None,
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            exp: current_unix_timestamp() - 1,
        },
    );
    let mut config = config_with_defaults(&tempdir);
    config.session_secret = Some("session_secret".to_owned());

    let response = crate::build_router(config)
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_clears_the_session_cookie() {
    let tempdir = tempdir().expect("tempdir");
    let mut config = config_with_defaults(&tempdir);
    config.session_secret = Some("session_secret".to_owned());

    let response = crate::build_router(config)
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let set_cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .expect("set-cookie");
    assert!(set_cookie.contains("archivist_session="));
    assert!(set_cookie.contains("Max-Age=0"));
    assert!(set_cookie.contains("HttpOnly"));
}
