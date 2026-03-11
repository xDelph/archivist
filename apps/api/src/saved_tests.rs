use super::{DeleteSavedItemResponse, SavedItemsResponse, SavedMutationResponse};
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

#[tokio::test]
async fn save_route_requires_authenticated_session() {
    let tempdir = tempdir().expect("tempdir");
    let response = build_router(config_with_defaults(&tempdir))
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/saved")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"thread_id":"C123:1700000000.000001"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn save_and_list_routes_persist_saved_threads() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_root".to_owned(),
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
                text: Some("Ship the first beta this week".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("message");
    store.refresh_thread_summaries().await;

    let session_token = build_session_token(
        "session_secret",
        &SessionClaims {
            slack_user_id: "U123".to_owned(),
            email: Some("thomas@example.com".to_owned()),
            display_name: Some("Thomas".to_owned()),
            avatar_url: Some("https://example.com/avatar.png".to_owned()),
            exp: current_unix_timestamp() + 60,
        },
    )
    .expect("session token");
    let app = build_router(config_with_defaults(&tempdir))
        .await
        .expect("router");

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/saved")
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
    let saved: SavedMutationResponse = serde_json::from_slice(&save_body).expect("json");
    assert!(saved.ok);
    assert_eq!(saved.item.channel_name.as_deref(), Some("general"));
    assert_eq!(saved.item.thread_id, format!("C123:{root_ts}"));

    let list_response = app
        .oneshot(
            Request::builder()
                .uri("/api/saved")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let listed: SavedItemsResponse = serde_json::from_slice(&list_body).expect("json");

    assert_eq!(listed.items.len(), 1);
    assert_eq!(listed.items[0].title, "Ship the first beta this week");
}

#[tokio::test]
async fn save_route_rejects_unknown_threads() {
    let tempdir = tempdir().expect("tempdir");
    let session_token = valid_session_token();
    let response = build_router(config_with_defaults(&tempdir))
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/saved")
                .header("content-type", "application/json")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::from(r#"{"thread_id":"C123:1700000000.000001"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_route_removes_saved_items() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let root_ts = "1700000000.000001";

    store
        .record_process_event(&ProcessEventJob {
            event_id: "evt_message".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U999".to_owned()),
                text: Some("Ship the first beta this week".to_owned()),
                ts: root_ts.to_owned(),
                thread_ts: None,
                files: vec![],
            },
        })
        .await
        .expect("message");
    store.refresh_thread_summaries().await;

    let session_token = valid_session_token();
    let app = build_router(config_with_defaults(&tempdir))
        .await
        .expect("router");
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/saved")
                .header("content-type", "application/json")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::from(format!(r#"{{"thread_id":"C123:{root_ts}"}}"#)))
                .expect("request"),
        )
        .await
        .expect("save response");

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/saved/C123:{root_ts}"))
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(delete_response.status(), StatusCode::OK);
    let delete_body = to_bytes(delete_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let deleted: DeleteSavedItemResponse = serde_json::from_slice(&delete_body).expect("json");
    assert!(deleted.ok);

    let list_response = app
        .oneshot(
            Request::builder()
                .uri("/api/saved")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("body");
    let listed: SavedItemsResponse = serde_json::from_slice(&list_body).expect("json");

    assert!(listed.items.is_empty());
}

fn config_with_defaults(tempdir: &tempfile::TempDir) -> ApiConfig {
    ApiConfig {
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
