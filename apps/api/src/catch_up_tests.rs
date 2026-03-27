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

#[tokio::test]
async fn build_catch_up_filters_to_the_requested_window() {
    let now = 1_700_000_000;
    let tempdir = tempdir().expect("tempdir");
    let channels = vec![];
    let summaries = vec![
        ThreadSummaryRow {
            channel_id: "C123".to_owned(),
            root_ts: format!("{}.000001", now - 60),
            reply_count: 0,
            participant_count: 1,
            reaction_count: 0,
            file_count: 0,
            root_message_at: format!("{}.000001", now - 60),
            last_activity_ts: format!("{}.000001", now - 60),
        },
        ThreadSummaryRow {
            channel_id: "C123".to_owned(),
            root_ts: format!("{}.000001", now - (8 * DAY_SECONDS)),
            reply_count: 0,
            participant_count: 1,
            reaction_count: 0,
            file_count: 0,
            root_message_at: format!("{}.000001", now - (8 * DAY_SECONDS)),
            last_activity_ts: format!("{}.000001", now - (8 * DAY_SECONDS)),
        },
    ];

    let catch_up = build_catch_up(
        channels,
        summaries,
        vec![],
        vec![],
        &crate::user_store::LocalUserStore::open(tempdir.path().join("synced-users.json"))
            .await
            .expect("user store")
            .into(),
        super::CatchUpFilters {
            window: CatchUpWindow::Week,
            channel_id: None,
            sort: super::CatchUpSort::Date,
        },
        now,
    )
    .await;

    assert_eq!(catch_up.channels.len(), 1);
    assert_eq!(catch_up.items.len(), 1);
    assert_eq!(catch_up.items[0].title, "(no text)");
    assert_eq!(catch_up.items[0].preview, "(no text)");
    assert_eq!(catch_up.items[0].summary_preview, None);
    assert_eq!(catch_up.items[0].preview_source, "fallback");
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
            .uri("/api/catch-up")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn catch_up_route_groups_threads_by_channel_and_sorts_by_date() {
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
            .uri("/api/catch-up?window=24h")
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

    assert_eq!(payload["window"], "24h");
    assert_eq!(payload["channels"].as_array().map(Vec::len), Some(2));
    assert_eq!(payload["channels"][0]["id"], "C123");
    assert_eq!(payload["channels"][0]["thread_count"], 1);
    assert_eq!(payload["items"].as_array().map(Vec::len), Some(2));
    assert_eq!(payload["items"][0]["author"], serde_json::Value::Null);
    assert_eq!(payload["items"][0]["channel_id"], "C123");
    assert_eq!(payload["items"][0]["reply_count"], 1);
    assert_eq!(payload["items"][0]["participant_count"], 3);
    assert_eq!(payload["items"][0]["reaction_count"], 1);
    assert_eq!(payload["items"][0]["file_count"], 1);
    assert_eq!(payload["items"][0]["preview"], "Recent root");
    assert_eq!(payload["channels"][1]["id"], "C999");
    assert_eq!(payload["next_cursor"], serde_json::Value::Null);
}

#[tokio::test]
async fn catch_up_route_rejects_invalid_windows() {
    let tempdir = tempdir().expect("tempdir");
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
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
            .uri("/api/catch-up?window=30d")
            .header("cookie", format!("arkivist_session={session_token}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn catch_up_route_returns_all_single_workspace_threads() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let now = current_unix_timestamp();

    for (event_id, channel_id, ts_suffix, text) in [
        ("evt_1", "C123", "000001", "visible root"),
        ("evt_2", "C123", "000002", "second root"),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                event_time: now,
                received_at: now,
                channel_id: channel_id.to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some(text.to_owned()),
                    ts: format!("{}.{ts_suffix}", now - 60),
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
            .uri("/api/catch-up")
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

    assert_eq!(payload["channels"].as_array().map(Vec::len), Some(1));
    assert_eq!(payload["channels"][0]["thread_count"], 2);
    assert_eq!(payload["items"].as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn catch_up_route_paginates_and_filters_threads() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let now = current_unix_timestamp();

    for (event_id, channel_id, offset, text) in [
        ("evt_1", "C123", 20, "first"),
        ("evt_2", "C123", 10, "second"),
        ("evt_3", "C999", 5, "third"),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                event_time: now - offset,
                received_at: now - offset,
                channel_id: channel_id.to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some(text.to_owned()),
                    ts: format!("{}.000001", now - offset),
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
            .uri("/api/catch-up?window=24h&channel_id=C123&limit=1")
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

    assert_eq!(payload["channels"].as_array().map(Vec::len), Some(2));
    assert_eq!(payload["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(payload["items"][0]["channel_id"], "C123");
    assert_eq!(payload["items"][0]["preview"], "second");
    assert_eq!(payload["next_cursor"], "1");
}

#[tokio::test]
async fn catch_up_route_supports_reaction_sort() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let now = current_unix_timestamp();
    let low_thread_ts = format!("{}.000001", now - 50);
    let high_thread_ts = format!("{}.000001", now - 100);
    let high_reply_ts = format!("{}.000002", now - 40);

    for (event_id, channel_id, ts, thread_ts, text) in [
        (
            "evt_root_low",
            "C123",
            low_thread_ts.as_str(),
            None,
            "recent but quiet",
        ),
        (
            "evt_root_high",
            "C999",
            high_thread_ts.as_str(),
            None,
            "older but hot",
        ),
        (
            "evt_reply_high",
            "C999",
            high_reply_ts.as_str(),
            Some(high_thread_ts.as_str()),
            "reply",
        ),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                event_time: now - 10,
                received_at: now - 10,
                channel_id: channel_id.to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some(text.to_owned()),
                    ts: ts.to_owned(),
                    thread_ts: thread_ts.map(str::to_owned),
                    files: vec![],
                },
            })
            .await
            .expect("message insert");
    }
    for (event_id, reaction, item_ts) in [
        ("evt_reaction_1", "eyes", high_reply_ts.as_str()),
        ("evt_reaction_2", "rocket", high_reply_ts.as_str()),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                event_time: now - 5,
                received_at: now - 5,
                channel_id: "C999".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ReactionAdded {
                    user_id: "U456".to_owned(),
                    reaction: reaction.to_owned(),
                    item_ts: item_ts.to_owned(),
                },
            })
            .await
            .expect("reaction insert");
    }

    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
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
            .uri("/api/catch-up?window=7d&sort=reactions")
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

    assert_eq!(payload["items"][0]["channel_id"], "C999");
    assert_eq!(payload["items"][0]["preview"], "older but hot");
}

#[tokio::test]
async fn catch_up_route_rejects_invalid_sort_and_cursor() {
    let tempdir = tempdir().expect("tempdir");
    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            email: None,
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            exp: current_unix_timestamp() + 60,
        },
    )
    .expect("session token");
    let app = build_router(ApiConfig {
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
    .expect("router");

    let invalid_sort = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/catch-up?sort=unknown")
                .header("cookie", format!("arkivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(invalid_sort.status(), StatusCode::BAD_REQUEST);

    let invalid_cursor = app
        .oneshot(
            Request::builder()
                .uri("/api/catch-up?cursor=bad")
                .header("cookie", format!("arkivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(invalid_cursor.status(), StatusCode::BAD_REQUEST);
}
