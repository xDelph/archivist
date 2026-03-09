use super::{ThreadListFilters, ThreadSort, build_thread_summaries};
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
use domain::{ChannelKind, EventPayload, ProcessEventJob};
use tempfile::tempdir;
use tower::util::ServiceExt;

#[test]
fn build_thread_summaries_sorts_by_requested_strategy() {
    let summaries = build_thread_summaries(
        vec![],
        vec![
            domain::Message {
                team_id: "T123".to_owned(),
                channel_id: "C123".to_owned(),
                ts: "1700000000.000001".to_owned(),
                thread_ts: None,
                user_id: Some("U123".to_owned()),
                text: "older".to_owned(),
            },
            domain::Message {
                team_id: "T123".to_owned(),
                channel_id: "C123".to_owned(),
                ts: "1700000005.000001".to_owned(),
                thread_ts: None,
                user_id: Some("U123".to_owned()),
                text: "newer".to_owned(),
            },
        ],
        vec![],
        vec![],
        ThreadListFilters {
            channel_id: None,
            date_from: None,
            date_to: None,
            sort: ThreadSort::Newest,
        },
    );

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].preview, "newer");
}

#[tokio::test]
async fn thread_list_route_returns_paginated_filtered_threads() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let now = current_unix_timestamp();

    for (event_id, channel_id, ts, text) in [
        (
            "evt_1",
            "C123",
            format!("{}.000001", now - 300),
            "first thread",
        ),
        (
            "evt_2",
            "C123",
            format!("{}.000001", now - 200),
            "second thread",
        ),
        (
            "evt_3",
            "C999",
            format!("{}.000001", now - 100),
            "third thread",
        ),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                team_id: "T123".to_owned(),
                event_time: now,
                received_at: now,
                channel_id: channel_id.to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some(text.to_owned()),
                    ts,
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("message insert");
    }

    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            team_id: "T123".to_owned(),
            email: None,
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            exp: now + 60,
        },
    )
    .expect("session token");

    let router = build_router(ApiConfig {
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
    .expect("router");

    let first = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/threads?channel_id=C123&sort=newest&limit=1")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = to_bytes(first.into_body(), usize::MAX).await.expect("body");
    let first_payload: serde_json::Value = serde_json::from_slice(&first_body).expect("json");
    assert_eq!(first_payload["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(first_payload["items"][0]["preview"], "second thread");
    assert_eq!(first_payload["next_cursor"], "1");

    let second = router
        .oneshot(
            Request::builder()
                .uri("/api/threads?channel_id=C123&sort=newest&limit=1&cursor=1")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::OK);
    let second_body = to_bytes(second.into_body(), usize::MAX)
        .await
        .expect("body");
    let second_payload: serde_json::Value = serde_json::from_slice(&second_body).expect("json");
    assert_eq!(second_payload["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(second_payload["items"][0]["preview"], "first thread");
    assert_eq!(second_payload["next_cursor"], serde_json::Value::Null);
}

#[tokio::test]
async fn thread_list_route_rejects_invalid_sort_values() {
    let tempdir = tempdir().expect("tempdir");
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
        event_log_path: tempdir.path().join("events.jsonl").display().to_string(),
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
            .uri("/api/threads?sort=invalid")
            .header("cookie", format!("archivist_session={session_token}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn thread_list_route_only_returns_threads_for_the_session_team() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let now = current_unix_timestamp();

    for (event_id, team_id, text) in [
        ("evt_t123", "T123", "visible thread"),
        ("evt_t999", "T999", "hidden thread"),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                team_id: team_id.to_owned(),
                event_time: now,
                received_at: now,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some(text.to_owned()),
                    ts: "1700000000.000001".to_owned(),
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("message insert");
    }
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            team_id: "T123".to_owned(),
            email: None,
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            exp: now + 60,
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
            .uri("/api/threads")
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

    assert_eq!(payload["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(payload["items"][0]["title"], "visible thread");
}
