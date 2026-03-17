use super::HighlightsResponse;
use crate::{
    ApiConfig, AppState,
    analytics_store::LocalAnalyticsStore,
    auth::{SessionClaims, build_session_token, current_unix_timestamp},
    auth_store::LocalAuthStore,
    build_router,
    highlight_store::{HighlightedThreadRecord, LocalHighlightStore},
    saved_store::LocalSavedItemStore,
    slack_highlights::{
        list_highlights_from_slack_command, pin_highlight_from_slack_command,
        unpin_highlight_from_slack_command,
    },
    user_role_store::ADMIN_ROLE,
    user_store::LocalUserStore,
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::post,
};
use db::JsonlEventStore;
use domain::{ChannelKind, EventPayload, ProcessEventJob};
use tempfile::tempdir;
use tower::util::ServiceExt;

#[tokio::test]
async fn highlights_route_requires_authenticated_session() {
    let tempdir = tempdir().expect("tempdir");
    let response = build_router(config_with_defaults(&tempdir))
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/highlights")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn highlights_route_lists_admin_curated_threads() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let first_root_ts = "1700000000.000001";
    let second_root_ts = "1700000100.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_channel_general".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("general".to_owned()),
                is_archived: Some(false),
            },
        })
        .await
        .expect("channel");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_channel_showcase".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C234".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("showcase".to_owned()),
                is_archived: Some(false),
            },
        })
        .await
        .expect("channel");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_message_one".to_owned(),
            event_time: 5,
            received_at: 6,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U999".to_owned()),
                text: Some("First curated thread".to_owned()),
                ts: first_root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("message");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_message_two".to_owned(),
            event_time: 7,
            received_at: 8,
            channel_id: "C234".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U998".to_owned()),
                text: Some("Second curated thread".to_owned()),
                ts: second_root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("message");
    store.refresh_thread_summaries().await;

    let highlights = LocalHighlightStore::open(tempdir.path().join("highlighted-threads.json"))
        .await
        .expect("highlight store");
    highlights
        .pin_thread(highlighted_thread("C123", first_root_ts, "U-admin", "10"))
        .await
        .expect("pin first");
    highlights
        .pin_thread(highlighted_thread("C234", second_root_ts, "U-admin", "20"))
        .await
        .expect("pin second");

    let app = build_router(config_with_defaults(&tempdir))
        .await
        .expect("router");
    let session_token = valid_session_token();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/highlights")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let listed: HighlightsResponse = serde_json::from_slice(&body).expect("json");

    assert_eq!(listed.items.len(), 2);
    assert_eq!(listed.items[0].thread_id, format!("C234:{second_root_ts}"));
    assert_eq!(listed.items[0].channel_name.as_deref(), Some("showcase"));
    assert_eq!(listed.items[1].thread_id, format!("C123:{first_root_ts}"));

    let filtered_response = app
        .oneshot(
            Request::builder()
                .uri("/api/highlights?channel_id=C123")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(filtered_response.status(), StatusCode::OK);
    let filtered_body = to_bytes(filtered_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let filtered: HighlightsResponse = serde_json::from_slice(&filtered_body).expect("json");
    assert_eq!(filtered.items.len(), 1);
    assert_eq!(filtered.items[0].thread_id, format!("C123:{first_root_ts}"));
}

#[tokio::test]
async fn highlight_mutations_require_admin_role() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_message".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U999".to_owned()),
                text: Some("Thread to highlight".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("message");
    store.refresh_thread_summaries().await;

    let app = build_router(config_with_defaults(&tempdir))
        .await
        .expect("router");
    let session_token = valid_session_token();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/highlights")
                .header("content-type", "application/json")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::from(format!(r#"{{"thread_id":"C123:{root_ts}"}}"#)))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_pin_and_unpin_highlights() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_channel".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("general".to_owned()),
                is_archived: Some(false),
            },
        })
        .await
        .expect("channel");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_message".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U999".to_owned()),
                text: Some("Thread to highlight".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("message");
    store.refresh_thread_summaries().await;

    let user_role_store =
        crate::user_role_store::LocalUserRoleStore::open(tempdir.path().join("user-roles.json"))
            .await
            .expect("user role store");
    user_role_store
        .grant_role("U123", ADMIN_ROLE)
        .await
        .expect("grant admin role");

    let app = build_router(config_with_defaults(&tempdir))
        .await
        .expect("router");
    let session_token = valid_session_token();

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/highlights")
                .header("content-type", "application/json")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::from(format!(r#"{{"thread_id":"C123:{root_ts}"}}"#)))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(save_response.status(), StatusCode::OK);
    let save_body = to_bytes(save_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload = serde_json::from_slice::<serde_json::Value>(&save_body).expect("json");
    assert_eq!(payload["item"]["thread_id"], format!("C123:{root_ts}"));

    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/highlights/C123:{root_ts}"))
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(delete_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn admin_can_pin_highlights_without_thread_summary_refresh() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_message".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U999".to_owned()),
                text: Some("Thread to highlight".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("message");

    let user_role_store =
        crate::user_role_store::LocalUserRoleStore::open(tempdir.path().join("user-roles.json"))
            .await
            .expect("user role store");
    user_role_store
        .grant_role("U123", ADMIN_ROLE)
        .await
        .expect("grant admin role");

    let app = build_router(config_with_defaults(&tempdir))
        .await
        .expect("router");
    let session_token = valid_session_token();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/highlights")
                .header("content-type", "application/json")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::from(format!(r#"{{"thread_id":"C123:{root_ts}"}}"#)))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn internal_slack_highlight_route_requires_valid_token_and_admin_role() {
    let tempdir = tempdir().expect("tempdir");
    let root_ts = "1700000000.000001";
    let store = JsonlEventStore::open(tempdir.path().join("events.jsonl"))
        .await
        .expect("store");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_message".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U999".to_owned()),
                text: Some("Thread to highlight".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("message");
    let user_role_store =
        crate::user_role_store::LocalUserRoleStore::open(tempdir.path().join("user-roles.json"))
            .await
            .expect("user role store");
    let app = Router::new()
        .route(
            "/api/internal/slack/highlights",
            post(pin_highlight_from_slack_command),
        )
        .with_state(
            test_state(
                &tempdir,
                store,
                user_role_store.clone(),
                Some("internal-secret".to_owned()),
            )
            .await,
        );

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/internal/slack/highlights")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"slack_user_id":"U123","thread_id":"C123:{root_ts}"}}"#
                )))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/internal/slack/highlights")
                .header("authorization", "Bearer internal-secret")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"slack_user_id":"U123","thread_id":"C123:{root_ts}"}}"#
                )))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    user_role_store
        .grant_role("U123", ADMIN_ROLE)
        .await
        .expect("grant admin role");
    let success = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/internal/slack/highlights")
                .header("authorization", "Bearer internal-secret")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"slack_user_id":"U123","thread_id":"C123:{root_ts}"}}"#
                )))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(success.status(), StatusCode::OK);
    let payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(success.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");

    assert_eq!(payload["item"]["thread_id"], format!("C123:{root_ts}"));
}

#[tokio::test]
async fn internal_slack_highlight_routes_can_list_and_unpin() {
    let tempdir = tempdir().expect("tempdir");
    let first_root_ts = "1700000000.000001";
    let second_root_ts = "1700000000.000002";
    let store = JsonlEventStore::open(tempdir.path().join("events.jsonl"))
        .await
        .expect("store");
    for (event_id, channel_id, channel_name, root_ts, text) in [
        (
            "evt_message_one",
            "C123",
            "general",
            first_root_ts,
            "First thread to highlight",
        ),
        (
            "evt_message_two",
            "C234",
            "showcase",
            second_root_ts,
            "Second thread to highlight",
        ),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: format!("{event_id}_channel"),
                event_time: 1,
                received_at: 2,
                channel_id: channel_id.to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ChannelUpdated {
                    name: Some(channel_name.to_owned()),
                    is_archived: Some(false),
                },
            })
            .await
            .expect("channel");
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                event_time: 3,
                received_at: 4,
                channel_id: channel_id.to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U999".to_owned()),
                    text: Some(text.to_owned()),
                    ts: root_ts.to_owned(),
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("message");
    }
    let user_role_store =
        crate::user_role_store::LocalUserRoleStore::open(tempdir.path().join("user-roles.json"))
            .await
            .expect("user role store");
    user_role_store
        .grant_role("U123", ADMIN_ROLE)
        .await
        .expect("grant admin role");
    let highlights = LocalHighlightStore::open(tempdir.path().join("highlighted-threads.json"))
        .await
        .expect("highlight store");
    highlights
        .pin_thread(highlighted_thread("C123", first_root_ts, "U123", "10"))
        .await
        .expect("pin first");
    highlights
        .pin_thread(highlighted_thread("C234", second_root_ts, "U123", "20"))
        .await
        .expect("pin second");
    let app = Router::new()
        .route(
            "/api/internal/slack/highlights/list",
            post(list_highlights_from_slack_command),
        )
        .route(
            "/api/internal/slack/highlights/unpin",
            post(unpin_highlight_from_slack_command),
        )
        .with_state(
            test_state(
                &tempdir,
                store,
                user_role_store.clone(),
                Some("internal-secret".to_owned()),
            )
            .await,
        );

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/internal/slack/highlights/list")
                .header("authorization", "Bearer internal-secret")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"slack_user_id":"U123"}"#))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(list_response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");
    assert_eq!(list_payload["items"].as_array().expect("items").len(), 2);

    let unpin_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/internal/slack/highlights/unpin")
                .header("authorization", "Bearer internal-secret")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"slack_user_id":"U123","thread_id":"C123:{first_root_ts}"}}"#
                )))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unpin_response.status(), StatusCode::OK);
    let remaining = LocalHighlightStore::open(tempdir.path().join("highlighted-threads.json"))
        .await
        .expect("highlight store")
        .list_threads()
        .await;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].thread_id, format!("C234:{second_root_ts}"));
}

fn highlighted_thread(
    channel_id: &str,
    root_ts: &str,
    pinned_by_user_id: &str,
    pinned_at: &str,
) -> HighlightedThreadRecord {
    HighlightedThreadRecord {
        thread_id: format!("{channel_id}:{root_ts}"),
        channel_id: channel_id.to_owned(),
        root_ts: root_ts.to_owned(),
        pinned_by_user_id: pinned_by_user_id.to_owned(),
        pinned_at: pinned_at.to_owned(),
    }
}

fn config_with_defaults(tempdir: &tempfile::TempDir) -> ApiConfig {
    ApiConfig {
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
    }
}

async fn test_state(
    tempdir: &tempfile::TempDir,
    store: JsonlEventStore,
    user_role_store: crate::user_role_store::LocalUserRoleStore,
    slack_command_token: Option<String>,
) -> AppState {
    AppState {
        store: store.into(),
        slack_auth: crate::auth::SlackAuthConfig {
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            team_id: None,
            token_url: None,
        },
        web_origin: crate::LOCAL_DEV_WEB_ORIGIN.to_owned(),
        session_secret: Some("session_secret".to_owned()),
        slack_command_token,
        auth_store: LocalAuthStore::open(tempdir.path().join("auth-identities.json"))
            .await
            .expect("auth store")
            .into(),
        user_store: LocalUserStore::open(tempdir.path().join("synced-users.json"))
            .await
            .expect("user store")
            .into(),
        user_role_store: user_role_store.into(),
        highlight_store: LocalHighlightStore::open(tempdir.path().join("highlighted-threads.json"))
            .await
            .expect("highlight store")
            .into(),
        saved_store: LocalSavedItemStore::open(tempdir.path().join("saved-items.json"))
            .await
            .expect("saved store")
            .into(),
        analytics_store: LocalAnalyticsStore::open(tempdir.path().join("analytics-events.json"))
            .await
            .expect("analytics store")
            .into(),
    }
}

fn valid_session_token() -> String {
    build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            email: Some("thomas@example.com".to_owned()),
            display_name: Some("Thomas".to_owned()),
            avatar_url: Some("https://example.com/avatar.png".to_owned()),
            exp: current_unix_timestamp() + 60,
        },
    )
    .expect("session token")
}
