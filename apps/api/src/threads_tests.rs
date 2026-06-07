use super::{build_thread_detail, parse_thread_id, same_slack_ts};
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
use db::{GeneratedThreadSummaryRow, JsonlEventStore};
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
fn same_slack_ts_accepts_equivalent_formats() {
    assert!(same_slack_ts("1768488821.313469", "1768488821.313469"));
    assert!(same_slack_ts("1768488821.313469", "1768488821.3134690"));
    assert!(same_slack_ts("1768488821.100000", "1768488821.1"));
    assert!(!same_slack_ts("1768488821", "1768488821.313469"));
}

#[tokio::test]
async fn build_thread_detail_requires_a_root_message() {
    let tempdir = tempdir().expect("tempdir");
    let detail = build_thread_detail(
        "C123:1700000000.000001",
        "C123",
        "1700000000.000001",
        super::ThreadDetailData {
            channels: vec![],
            messages: vec![Message {
                channel_id: "C123".to_owned(),
                ts: "1700000000.000002".to_owned(),
                thread_ts: Some("1700000000.000001".to_owned()),
                user_id: Some("U123".to_owned()),
                text: "reply only".to_owned(),
            }],
            reactions: vec![],
            files: vec![],
            thread_summaries: vec![],
            generated_thread_summaries: vec![],
        },
        &crate::user_store::LocalUserStore::open(tempdir.path().join("synced-users.json"))
            .await
            .expect("user store")
            .into(),
    )
    .await;

    assert_eq!(detail, None);
}

#[tokio::test]
async fn thread_detail_route_returns_messages_reactions_and_files() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    std::fs::write(
        tempdir.path().join("synced-users.json"),
        serde_json::to_vec(&vec![serde_json::json!({
            "slack_user_id": "U789",
            "display_name": "Thomas",
            "avatar_url": null,
            "is_active": true
        })])
        .expect("users json"),
    )
    .expect("write synced users");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_channel".to_owned(),
            event_time: 0,
            received_at: 0,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("general".to_owned()),
                is_archived: Some(false),
            },
        })
        .await
        .expect("channel insert");

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_root".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("root message --&gt; <@U789> in <#C123>".to_owned()),
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
    store.refresh_thread_summaries().await;
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
            .uri("/api/threads/C123:1700000000.000001")
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

    assert_eq!(payload["reply_count"], 1);
    assert_eq!(payload["channel_name"], "general");
    assert_eq!(payload["title"], "root message --> @Thomas in #general");
    assert_eq!(
        payload["summary"]["text"],
        "root message --> @Thomas in #general"
    );
    assert_eq!(payload["summary"]["source"], "fallback");
    assert_eq!(payload["summary"]["is_stale"], false);
    assert_eq!(
        payload["messages"][0]["text"],
        "root message --> @Thomas in #general"
    );
    assert_eq!(payload["messages"][0]["author"], serde_json::Value::Null);
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
            .uri("/api/threads/not-a-thread-id")
            .header("cookie", format!("arkivist_session={session_token}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn thread_detail_route_resolves_single_workspace_threads() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_other_team_root".to_owned(),
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
            .uri("/api/threads/C123:1700000000.000001")
            .header("cookie", format!("arkivist_session={session_token}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn thread_detail_route_returns_fresh_generated_summary_metadata() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_channel".to_owned(),
            event_time: 0,
            received_at: 0,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("general".to_owned()),
                is_archived: Some(false),
            },
        })
        .await
        .expect("channel insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_root".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("Root note".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("root insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_reply".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U456".to_owned()),
                text: Some("Reply with decision".to_owned()),
                ts: "1700000000.000002".to_owned(),
                thread_ts: Some(root_ts.to_owned()),
                files: vec![],
            },
        })
        .await
        .expect("reply insert");
    store.refresh_thread_summaries().await;
    store
        .upsert_generated_thread_summary(&GeneratedThreadSummaryRow {
            channel_id: "C123".to_owned(),
            root_ts: root_ts.to_owned(),
            summary: "Launch plan is settled and assigned.".to_owned(),
            full_summary: Some(
                "## Outcome\nLaunch plan is settled and assigned.\n\n## Next steps\nShip it."
                    .to_owned(),
            ),
            why_it_mattered: Some("The team can ship without another sync.".to_owned()),
            status: "answered".to_owned(),
            topic_tags: vec!["launch".to_owned(), "checklist".to_owned()],
            source_last_activity_ts: "1700000000.000002".to_owned(),
            model: "openrouter/test".to_owned(),
            generated_at: 42,
        })
        .await
        .expect("seed generated summary");

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
            .uri("/api/threads/C123:1700000000.000001")
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

    assert_eq!(payload["summary"]["source"], "ai");
    assert_eq!(
        payload["summary"]["text"],
        "Launch plan is settled and assigned."
    );
    assert_eq!(
        payload["summary"]["why_it_mattered"],
        "The team can ship without another sync."
    );
    assert_eq!(
        payload["summary"]["full_summary"],
        "## Outcome\nLaunch plan is settled and assigned.\n\n## Next steps\nShip it."
    );
    assert_eq!(payload["summary"]["status"], "answered");
    assert_eq!(
        payload["summary"]["topic_tags"],
        serde_json::json!(["launch", "checklist"])
    );
    assert_eq!(payload["summary"]["model"], "openrouter/test");
    assert_eq!(payload["summary"]["generated_at"], 42);
    assert_eq!(payload["summary"]["is_stale"], false);
}

#[tokio::test]
async fn thread_detail_route_formats_mentions_in_generated_summary_metadata() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_channel".to_owned(),
            event_time: 0,
            received_at: 0,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("general".to_owned()),
                is_archived: Some(false),
            },
        })
        .await
        .expect("channel insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_root".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("Root note".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("root insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_reply".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U456".to_owned()),
                text: Some("Reply".to_owned()),
                ts: "1700000000.000002".to_owned(),
                thread_ts: Some(root_ts.to_owned()),
                files: vec![],
            },
        })
        .await
        .expect("reply insert");
    let user_store = LocalUserStore::open(tempdir.path().join("synced-users.json"))
        .await
        .expect("user store");
    user_store
        .upsert_user(SyncedUserRecord {
            slack_user_id: "U123".to_owned(),
            display_name: Some("thomas".to_owned()),
            avatar_url: None,
            is_active: true,
            is_anonymized: false,
        })
        .await
        .expect("seed user");
    user_store
        .upsert_user(SyncedUserRecord {
            slack_user_id: "U456".to_owned(),
            display_name: Some("patrick".to_owned()),
            avatar_url: None,
            is_active: true,
            is_anonymized: false,
        })
        .await
        .expect("seed user");
    store.refresh_thread_summaries().await;
    store
        .upsert_generated_thread_summary(&GeneratedThreadSummaryRow {
            channel_id: "C123".to_owned(),
            root_ts: root_ts.to_owned(),
            summary: "<@U456> confirmed next steps in <#C123|general>.".to_owned(),
            full_summary: Some(
                "## Outcome\n<@U456> confirmed next steps in <#C123|general>.".to_owned(),
            ),
            why_it_mattered: Some("Keeps <@U123> aligned.".to_owned()),
            status: "answered".to_owned(),
            topic_tags: vec![],
            source_last_activity_ts: "1700000000.000002".to_owned(),
            model: "openrouter/test".to_owned(),
            generated_at: 42,
        })
        .await
        .expect("seed generated summary");

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
            .uri("/api/threads/C123:1700000000.000001")
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

    assert_eq!(
        payload["summary"]["full_summary"],
        "## Outcome\n@patrick confirmed next steps in #general."
    );
    assert_eq!(
        payload["summary"]["why_it_mattered"],
        "Keeps @thomas aligned."
    );
}

#[tokio::test]
async fn thread_detail_route_ignores_stale_generated_summary_metadata() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_channel".to_owned(),
            event_time: 0,
            received_at: 0,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("general".to_owned()),
                is_archived: Some(false),
            },
        })
        .await
        .expect("channel insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_root".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("Root note".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("root insert");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_reply".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U456".to_owned()),
                text: Some("First reply".to_owned()),
                ts: "1700000000.000002".to_owned(),
                thread_ts: Some(root_ts.to_owned()),
                files: vec![],
            },
        })
        .await
        .expect("reply insert");
    store.refresh_thread_summaries().await;
    store
        .upsert_generated_thread_summary(&GeneratedThreadSummaryRow {
            channel_id: "C123".to_owned(),
            root_ts: root_ts.to_owned(),
            summary: "Old answer".to_owned(),
            full_summary: Some("## Outcome\nOld answer".to_owned()),
            why_it_mattered: Some("Stale".to_owned()),
            status: "answered".to_owned(),
            topic_tags: vec!["old".to_owned()],
            source_last_activity_ts: "1700000000.000002".to_owned(),
            model: "openrouter/test".to_owned(),
            generated_at: 7,
        })
        .await
        .expect("seed generated summary");
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_reply_new".to_owned(),
            event_time: 5,
            received_at: 6,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U789".to_owned()),
                text: Some("Latest update".to_owned()),
                ts: "1700000000.000003".to_owned(),
                thread_ts: Some(root_ts.to_owned()),
                files: vec![],
            },
        })
        .await
        .expect("new reply insert");
    store.refresh_thread_summaries().await;

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
            .uri("/api/threads/C123:1700000000.000001")
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

    assert_eq!(payload["summary"]["source"], "ai");
    assert_eq!(payload["summary"]["text"], "Old answer");
    assert_eq!(payload["summary"]["full_summary"], "## Outcome\nOld answer");
    assert_eq!(payload["summary"]["why_it_mattered"], "Stale");
    assert_eq!(payload["summary"]["status"], "answered");
    assert_eq!(payload["summary"]["topic_tags"], serde_json::json!(["old"]));
    assert_eq!(payload["summary"]["generated_at"], 7);
    assert_eq!(payload["summary"]["is_stale"], true);
}
