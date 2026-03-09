use super::{CatchUpWindow, DAY_SECONDS, build_catch_up};
use crate::{
    ApiConfig,
    auth::{SessionClaims, build_session_token, current_unix_timestamp},
    build_router,
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use db::{JsonlEventStore, ThreadSummaryRow};
use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
use tempfile::tempdir;
use tower::util::ServiceExt;

#[test]
fn build_catch_up_filters_to_the_requested_window() {
    let now = 1_700_000_000;
    let channels = vec![];
    let summaries = vec![
        ThreadSummaryRow {
            team_id: "T123".to_owned(),
            channel_id: "C123".to_owned(),
            root_ts: format!("{}.000001", now - 60),
            title: "Recent".to_owned(),
            preview: "recent".to_owned(),
            reply_count: 0,
            participant_count: 1,
            reaction_count: 0,
            file_count: 0,
            last_activity_ts: format!("{}.000001", now - 60),
        },
        ThreadSummaryRow {
            team_id: "T123".to_owned(),
            channel_id: "C123".to_owned(),
            root_ts: format!("{}.000001", now - (8 * DAY_SECONDS)),
            title: "Old".to_owned(),
            preview: "too old".to_owned(),
            reply_count: 0,
            participant_count: 1,
            reaction_count: 0,
            file_count: 0,
            last_activity_ts: format!("{}.000001", now - (8 * DAY_SECONDS)),
        },
    ];

    let catch_up = build_catch_up(channels, summaries, CatchUpWindow::Week, now);

    assert_eq!(catch_up.len(), 1);
    assert_eq!(catch_up[0].threads.len(), 1);
    assert_eq!(catch_up[0].threads[0].preview, "recent");
}

#[tokio::test]
async fn catch_up_route_requires_authenticated_session() {
    let tempdir = tempdir().expect("tempdir");
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
            .uri("/api/catch-up")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn catch_up_route_groups_threads_by_channel_and_sorts_by_activity() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let now = current_unix_timestamp();
    let root_recent_ts = format!("{}.000001", now - 120);
    let reply_recent_ts = format!("{}.000002", now - 60);
    let root_other_ts = format!("{}.000001", now - 600);
    let old_root_ts = format!("{}.000001", now - (8 * DAY_SECONDS));

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_channel".to_owned(),
            team_id: "T123".to_owned(),
            event_time: now - 1_000,
            received_at: now - 1_000,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("general".to_owned()),
                is_archived: Some(false),
            },
        })
        .await
        .expect("channel update");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_root_recent".to_owned(),
            team_id: "T123".to_owned(),
            event_time: now - 120,
            received_at: now - 120,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("Recent root".to_owned()),
                ts: root_recent_ts.clone(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("root recent");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_reply_recent".to_owned(),
            team_id: "T123".to_owned(),
            event_time: now - 60,
            received_at: now - 60,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U456".to_owned()),
                text: Some("Latest reply".to_owned()),
                ts: reply_recent_ts.clone(),
                thread_ts: Some(root_recent_ts.clone()),
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
        .expect("reply recent");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_reaction_recent".to_owned(),
            team_id: "T123".to_owned(),
            event_time: now - 59,
            received_at: now - 59,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ReactionAdded {
                user_id: "U789".to_owned(),
                reaction: "eyes".to_owned(),
                item_ts: reply_recent_ts.clone(),
            },
        })
        .await
        .expect("reaction recent");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_root_other".to_owned(),
            team_id: "T123".to_owned(),
            event_time: now - 600,
            received_at: now - 600,
            channel_id: "C999".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U111".to_owned()),
                text: Some("Other channel root".to_owned()),
                ts: root_other_ts.clone(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("other root");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_old_root".to_owned(),
            team_id: "T123".to_owned(),
            event_time: now - (8 * DAY_SECONDS),
            received_at: now - (8 * DAY_SECONDS),
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U222".to_owned()),
                text: Some("Old root".to_owned()),
                ts: old_root_ts,
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("old root");

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
            .uri("/api/catch-up?window=24h")
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

    assert_eq!(payload["window"], "24h");
    assert_eq!(payload["channels"].as_array().map(Vec::len), Some(2));
    assert_eq!(payload["channels"][0]["id"], "C123");
    assert_eq!(payload["channels"][0]["thread_count"], 1);
    assert_eq!(payload["channels"][0]["threads"][0]["reply_count"], 1);
    assert_eq!(payload["channels"][0]["threads"][0]["participant_count"], 3);
    assert_eq!(payload["channels"][0]["threads"][0]["reaction_count"], 1);
    assert_eq!(payload["channels"][0]["threads"][0]["file_count"], 1);
    assert_eq!(
        payload["channels"][0]["threads"][0]["preview"],
        "Recent root"
    );
    assert_eq!(payload["channels"][1]["id"], "C999");
}

#[tokio::test]
async fn catch_up_route_rejects_invalid_windows() {
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
            .uri("/api/catch-up?window=30d")
            .header("cookie", format!("archivist_session={session_token}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
