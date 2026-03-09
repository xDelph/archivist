use super::{build_thread_detail, parse_thread_id};
use crate::{
    ApiConfig,
    auth::{SessionClaims, build_session_token, current_unix_timestamp},
    build_router,
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use db::JsonlEventStore;
use domain::{ChannelKind, EventPayload, Message, ProcessEventJob, SharedFile};
use tempfile::tempdir;
use tower::util::ServiceExt;

#[test]
fn parse_thread_id_requires_channel_and_timestamp() {
    assert_eq!(
        parse_thread_id("C123:1700000000.000001"),
        Some(("C123", "1700000000.000001"))
    );
    assert_eq!(parse_thread_id("C123"), None);
    assert_eq!(parse_thread_id(":1700000000.000001"), None);
}

#[test]
fn build_thread_detail_requires_a_root_message() {
    let detail = build_thread_detail(
        "C123:1700000000.000001",
        "C123",
        "1700000000.000001",
        vec![Message {
            team_id: "T123".to_owned(),
            channel_id: "C123".to_owned(),
            ts: "1700000000.000002".to_owned(),
            thread_ts: Some("1700000000.000001".to_owned()),
            user_id: Some("U123".to_owned()),
            text: "reply only".to_owned(),
        }],
        vec![],
        vec![],
    );

    assert_eq!(detail, None);
}

#[tokio::test]
async fn thread_detail_route_returns_messages_reactions_and_files() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_root".to_owned(),
            team_id: "T123".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("root message".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![SharedFile {
                    id: "F123".to_owned(),
                    name: "brief.pdf".to_owned(),
                    mimetype: Some("application/pdf".to_owned()),
                    permalink: None,
                    size: Some(42),
                }],
            },
        })
        .await
        .expect("root insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_reply".to_owned(),
            team_id: "T123".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U456".to_owned()),
                text: Some("reply message".to_owned()),
                ts: "1700000000.000002".to_owned(),
                thread_ts: Some(root_ts.to_owned()),
                files: vec![],
            },
        })
        .await
        .expect("reply insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_reaction".to_owned(),
            team_id: "T123".to_owned(),
            event_time: 5,
            received_at: 6,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ReactionAdded {
                user_id: "U789".to_owned(),
                reaction: "eyes".to_owned(),
                item_ts: root_ts.to_owned(),
            },
        })
        .await
        .expect("reaction insert");
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            team_id: "T123".to_owned(),
            email: None,
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            exp: current_unix_timestamp() + 60,
        },
    )
    .expect("session token");

    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
        slack_workspace_id: None,
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
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri("/api/threads/C123:1700000000.000001")
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
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(payload["reply_count"], 1);
    assert_eq!(payload["messages"][0]["text"], "root message");
    assert_eq!(payload["messages"][0]["reactions"][0]["name"], "eyes");
    assert_eq!(payload["messages"][0]["files"][0]["name"], "brief.pdf");
    assert_eq!(payload["messages"][1]["text"], "reply message");
}

#[tokio::test]
async fn thread_detail_route_requires_authenticated_session() {
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
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri("/api/threads/not-a-thread-id")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn thread_detail_route_rejects_invalid_ids() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            team_id: "T123".to_owned(),
            email: None,
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            exp: current_unix_timestamp() + 60,
        },
    )
    .expect("session token");

    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
        slack_workspace_id: None,
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
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri("/api/threads/not-a-thread-id")
            .header("cookie", format!("archivist_session={session_token}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn thread_detail_route_hides_threads_from_other_teams() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_other_team_root".to_owned(),
            team_id: "T999".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U999".to_owned()),
                text: Some("hidden root".to_owned()),
                ts: "1700000000.000001".to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("message insert");
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            team_id: "T123".to_owned(),
            email: None,
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            exp: current_unix_timestamp() + 60,
        },
    )
    .expect("session token");

    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
        slack_workspace_id: None,
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
    })
    .await
    .expect("router")
    .oneshot(
        Request::builder()
            .uri("/api/threads/C123:1700000000.000001")
            .header("cookie", format!("archivist_session={session_token}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
