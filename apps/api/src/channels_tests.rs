use crate::{ApiConfig, auth::build_session_token, auth::current_unix_timestamp, build_router};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use db::JsonlEventStore;
use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
use tempfile::tempdir;
use tower::util::ServiceExt;

#[tokio::test]
async fn channels_route_requires_authenticated_session() {
    let tempdir = tempdir().expect("tempdir");
    let response = build_router(ApiConfig {
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
    .oneshot(
        Request::builder()
            .uri("/api/channels")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn channels_route_reports_activity_and_channel_metadata() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_message".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("hello".to_owned()),
                ts: "1700000000.000010".to_owned(),
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
        .expect("message insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_reaction".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ReactionAdded {
                user_id: "U456".to_owned(),
                reaction: "thumbsup".to_owned(),
                item_ts: "1700000000.000010".to_owned(),
            },
        })
        .await
        .expect("reaction insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_channel".to_owned(),
            event_time: 5,
            received_at: 6,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("announcements".to_owned()),
                is_archived: Some(false),
            },
        })
        .await
        .expect("channel insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_message_2".to_owned(),
            event_time: 7,
            received_at: 8,
            channel_id: "C999".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U789".to_owned()),
                text: Some("second channel".to_owned()),
                ts: "1700000000.000009".to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("second message insert");
    let session_token = build_session_token(
        "session_secret",
        &crate::auth::SessionClaims {
            slack_user_id: "U123".to_owned(),
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
    .oneshot(
        Request::builder()
            .uri("/api/channels")
            .header("cookie", format!("arkivist_session={session_token}"))
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

    assert_eq!(payload.as_array().map(Vec::len), Some(2));
    assert_eq!(payload[0]["id"], "C123");
    assert_eq!(payload[0]["name"], "announcements");
    assert_eq!(payload[0]["kind"], "public");
    assert_eq!(payload[0]["message_count"], 1);
    assert_eq!(payload[0]["reaction_count"], 1);
    assert_eq!(payload[0]["file_count"], 1);
    assert_eq!(payload[1]["id"], "C999");
    assert_eq!(payload[1]["name"], serde_json::Value::Null);
    assert_eq!(payload[1]["kind"], "public");
    assert_eq!(payload[1]["message_count"], 1);
}

#[tokio::test]
async fn channels_route_merges_single_workspace_channel_activity() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");

    for (event_id, channel_id, ts, text) in [
        ("evt_1", "C123", "1700000000.000001", "first thread"),
        ("evt_2", "C123", "1700000000.000002", "second thread"),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                event_time: 1,
                received_at: 2,
                channel_id: channel_id.to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some(text.to_owned()),
                    ts: ts.to_owned(),
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("message insert");
    }
    let session_token = build_session_token(
        "session_secret",
        &crate::auth::SessionClaims {
            slack_user_id: "U123".to_owned(),
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
    .oneshot(
        Request::builder()
            .uri("/api/channels")
            .header("cookie", format!("arkivist_session={session_token}"))
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

    assert_eq!(payload.as_array().map(Vec::len), Some(1));
    assert_eq!(payload[0]["id"], "C123");
    assert_eq!(payload[0]["message_count"], 2);
}
