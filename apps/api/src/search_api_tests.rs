use super::{SearchApiQuery, SearchResponse, build_search_results, parse_search_query};
use crate::{
    ApiConfig,
    auth::{SessionClaims, build_session_token, current_unix_timestamp},
    build_router,
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use db::{JsonlEventStore, SearchDocumentRow};
use domain::{ChannelKind, EventPayload, ProcessEventJob};
use search::SearchSort;
use tempfile::tempdir;
use tower::util::ServiceExt;

#[test]
fn parse_search_query_requires_query_text() {
    let query = SearchApiQuery {
        q: Some("   ".to_owned()),
        channel_id: None,
        date_from: None,
        date_to: None,
        sort: None,
        cursor: None,
        limit: None,
    };
    let error = parse_search_query(&query).expect_err("missing query");

    assert_eq!(error.0, StatusCode::BAD_REQUEST);
}

#[test]
fn build_search_results_respects_filters_and_sorting() {
    let query = search::SearchQuery {
        text: "release notes".to_owned(),
        filters: search::SearchFilters {
            channel_ids: vec!["C123".to_owned()],
            date_from: Some("1700000100".to_owned()),
            date_to: Some("1700000999".to_owned()),
        },
        sort: SearchSort::Relevance,
    };

    let items = build_search_results(
        vec![
            domain::Channel {
                id: "C123".to_owned(),
                name: Some("product".to_owned()),
                kind: domain::ChannelKind::Public,
                is_archived: false,
            },
            domain::Channel {
                id: "C999".to_owned(),
                name: Some("random".to_owned()),
                kind: domain::ChannelKind::Public,
                is_archived: false,
            },
        ],
        vec![
            domain::Message {
                channel_id: "C123".to_owned(),
                ts: "1700000200.000001".to_owned(),
                thread_ts: None,
                user_id: Some("U123".to_owned()),
                text: "release notes are ready".to_owned(),
            },
            domain::Message {
                channel_id: "C123".to_owned(),
                ts: "1700000300.000001".to_owned(),
                thread_ts: Some("1700000200.000001".to_owned()),
                user_id: Some("U123".to_owned()),
                text: "release notes include search improvements".to_owned(),
            },
            domain::Message {
                channel_id: "C999".to_owned(),
                ts: "1700000400.000001".to_owned(),
                thread_ts: None,
                user_id: Some("U999".to_owned()),
                text: "release notes in another channel".to_owned(),
            },
        ],
        vec![
            SearchDocumentRow {
                channel_id: "C123".to_owned(),
                root_ts: "1700000200.000001".to_owned(),
                message_ts: "1700000200.000001".to_owned(),
                title: Some("release notes are ready".to_owned()),
                body: "release notes are ready".to_owned(),
                message_occurred_at: "1700000200".to_owned(),
            },
            SearchDocumentRow {
                channel_id: "C123".to_owned(),
                root_ts: "1700000200.000001".to_owned(),
                message_ts: "1700000300.000001".to_owned(),
                title: Some("release notes are ready".to_owned()),
                body: "release notes include search improvements".to_owned(),
                message_occurred_at: "1700000300".to_owned(),
            },
            SearchDocumentRow {
                channel_id: "C999".to_owned(),
                root_ts: "1700000400.000001".to_owned(),
                message_ts: "1700000400.000001".to_owned(),
                title: Some("release notes in another channel".to_owned()),
                body: "release notes in another channel".to_owned(),
                message_occurred_at: "1700000400".to_owned(),
            },
        ],
        &query,
    );

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].channel_id, "C123");
    assert_eq!(items[0].channel_name.as_deref(), Some("product"));
    assert_eq!(items[0].thread_id, "C123:1700000200.000001");
    assert!(items[0].score >= items[1].score);
}

#[tokio::test]
async fn search_route_returns_filtered_results_for_authenticated_users() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let now = current_unix_timestamp();

    for (event_id, channel_id, ts, text, thread_ts) in [
        (
            "evt_1",
            "C123",
            format!("{}.000001", now - 120),
            "release notes",
            None,
        ),
        (
            "evt_2",
            "C123",
            format!("{}.000001", now - 60),
            "search release notes follow-up",
            Some(format!("{}.000001", now - 120)),
        ),
        (
            "evt_3",
            "C999",
            format!("{}.000001", now - 30),
            "release notes elsewhere",
            None,
        ),
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
                    ts,
                    thread_ts,
                    files: vec![],
                },
            })
            .await
            .expect("message insert");
    }
    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_4".to_owned(),
            event_time: now,
            received_at: now,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U999".to_owned()),
                text: Some("release notes from another author".to_owned()),
                ts: format!("{}.000001", now - 10),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("other team message insert");

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

    let router = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: path.display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
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

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/search?q=release%20notes&channel_id=C123&sort=relevance&limit=2")
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
    let payload: SearchResponse = serde_json::from_slice(&body).expect("json");

    assert_eq!(payload.query, "release notes");
    assert_eq!(payload.items.len(), 2);
    assert!(payload.items.iter().all(|item| item.channel_id == "C123"));
    assert!(
        payload
            .items
            .iter()
            .any(|item| item.snippet.contains("another author"))
    );
}

#[tokio::test]
async fn search_route_requires_authenticated_session() {
    let tempdir = tempdir().expect("tempdir");

    let response = build_router(ApiConfig {
        host: "127.0.0.1".to_owned(),
        port: 4000,
        event_log_path: tempdir.path().join("events.jsonl").display().to_string(),
        slack_client_id: None,
        slack_client_secret: None,
        slack_redirect_uri: None,
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
            .uri("/api/search?q=release")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn search_route_rejects_invalid_sort_values() {
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
            .uri("/api/search?q=release&sort=invalid")
            .header("cookie", format!("archivist_session={session_token}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
