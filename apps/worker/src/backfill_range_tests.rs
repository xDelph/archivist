use super::{
    BackfillChannelRequest, parse_backfill_request, resolve_user_sync_oldest_ts, should_sync_user,
};
use db::{EventStore, JsonlEventStore};
use domain::{ChannelKind, EventPayload, ProcessEventJob};
use tempfile::tempdir;

#[test]
fn parse_backfill_request_rejects_conflicting_range_options() {
    let body = axum::body::Bytes::from_static(
        br#"{"resume_from_last_message_ts":true,"oldest_ts":"1700000000.000001"}"#,
    );

    let error = parse_backfill_request(&body).expect_err("conflict should fail");

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
}

#[test]
fn should_sync_user_honors_oldest_ts_cutoff() {
    assert!(should_sync_user(
        Some(1_700_000_001),
        Some("1700000000.000001")
    ));
    assert!(!should_sync_user(
        Some(1_700_000_000),
        Some("1700000000.000001")
    ));
    assert!(should_sync_user(None, Some("1700000000.000001")));
    assert!(should_sync_user(Some(1_700_000_000), None));
}

#[tokio::test]
async fn resolve_user_sync_oldest_ts_uses_the_earliest_channel_resume_point() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");

    for (event_id, channel_id, ts) in [
        ("evt-1", "C123", "1700000005.000001"),
        ("evt-2", "C123", "1700000008.000001"),
        ("evt-3", "C456", "1700000003.000001"),
    ] {
        store
            .record_process_event(&ProcessEventJob {
                event_id: event_id.to_owned(),
                event_time: 1_700_000_000,
                received_at: 1_700_000_000,
                channel_id: channel_id.to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some("message".to_owned()),
                    ts: ts.to_owned(),
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("insert message");
    }

    let event_store = EventStore::from(store.clone());
    let oldest_ts = resolve_user_sync_oldest_ts(
        &event_store,
        &BackfillChannelRequest {
            resume_from_last_message_ts: true,
            ..BackfillChannelRequest::default()
        },
        &["C123".to_owned(), "C456".to_owned()],
    )
    .await
    .expect("resolve oldest ts");

    assert_eq!(oldest_ts.as_deref(), Some("1700000003.000001"));
}
