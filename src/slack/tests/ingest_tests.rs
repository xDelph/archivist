use crate::db::{InMemoryRepository, Repository};
use crate::slack::ingest::handle_event;
use crate::slack::types::{
    EventCallback, MessageEvent, MessageUpdate, ReactionEvent, ReactionItem, SlackEvent,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_message_cb(event_id: &str, subtype: Option<&str>) -> (EventCallback, serde_json::Value) {
    let cb = EventCallback {
        team_id: "T001".into(),
        api_app_id: "A001".into(),
        event_id: event_id.into(),
        event_time: 1_700_000_000,
        event: SlackEvent::Message(MessageEvent {
            channel: "C001".into(),
            user: Some("U001".into()),
            text: Some("hello".into()),
            ts: format!("{event_id}.000100"),
            thread_ts: None,
            subtype: subtype.map(String::from),
            message: None,
        }),
    };
    let raw = serde_json::json!({"event_id": event_id});
    (cb, raw)
}

fn make_reaction_cb(event_id: &str) -> (EventCallback, serde_json::Value) {
    let cb = EventCallback {
        team_id: "T001".into(),
        api_app_id: "A001".into(),
        event_id: event_id.into(),
        event_time: 1_700_000_001,
        event: SlackEvent::ReactionAdded(ReactionEvent {
            reaction: "thumbsup".into(),
            user: "U001".into(),
            item: ReactionItem {
                item_type: "message".into(),
                channel: "C001".into(),
                ts: "1700000000.000100".into(),
            },
            event_ts: "1700000001.000100".into(),
        }),
    };
    let raw = serde_json::json!({"event_id": event_id});
    (cb, raw)
}

fn make_unknown_cb(event_id: &str) -> (EventCallback, serde_json::Value) {
    let cb = EventCallback {
        team_id: "T001".into(),
        api_app_id: "A001".into(),
        event_id: event_id.into(),
        event_time: 1_700_000_002,
        event: SlackEvent::Unknown,
    };
    let raw = serde_json::json!({"event_id": event_id});
    (cb, raw)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_message_is_stored() {
    let repo = InMemoryRepository::default();
    let (cb, raw) = make_message_cb("Ev001", None);

    handle_event(&repo, None, "", cb, raw).await.unwrap();

    assert!(repo.event_exists("Ev001").await.unwrap());
    assert_eq!(repo.messages.lock().unwrap().len(), 1);
    assert_eq!(repo.weekly_upserts.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_duplicate_event_id_is_ignored() {
    let repo = InMemoryRepository::default();
    let (cb1, raw1) = make_message_cb("Ev001", None);
    let (cb2, raw2) = make_message_cb("Ev001", None);

    handle_event(&repo, None, "", cb1, raw1).await.unwrap();
    handle_event(&repo, None, "", cb2, raw2).await.unwrap();

    // Only one message stored despite two calls
    assert_eq!(repo.messages.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_bot_message_subtype_is_ignored() {
    let repo = InMemoryRepository::default();
    let (cb, raw) = make_message_cb("Ev002", Some("bot_message"));

    handle_event(&repo, None, "", cb, raw).await.unwrap();

    // Event recorded for dedup, but no message stored
    assert!(repo.event_exists("Ev002").await.unwrap());
    assert_eq!(repo.messages.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn test_all_ignored_subtypes_are_skipped() {
    let ignored = [
        "bot_message",
        "channel_join",
        "channel_leave",
        "channel_topic",
        "channel_purpose",
        "channel_name",
        "channel_archive",
        "channel_unarchive",
        "message_replied",
    ];
    for (i, subtype) in ignored.iter().enumerate() {
        let repo = InMemoryRepository::default();
        let event_id = format!("Ev{i:03}");
        let (cb, raw) = make_message_cb(&event_id, Some(subtype));
        handle_event(&repo, None, "", cb, raw).await.unwrap();
        assert_eq!(
            repo.messages.lock().unwrap().len(),
            0,
            "subtype '{subtype}' should be ignored"
        );
    }
}

#[tokio::test]
async fn test_message_changed_updates_correct_ts() {
    let repo = InMemoryRepository::default();

    // Store the original message first
    let (cb_orig, raw_orig) = make_message_cb("Ev030", None);
    handle_event(&repo, None, "", cb_orig, raw_orig)
        .await
        .unwrap();

    // Now send a message_changed event — nested message has the original ts
    let original_ts = "Ev030.000100".to_string();
    let cb_edit = EventCallback {
        team_id: "T001".into(),
        api_app_id: "A001".into(),
        event_id: "Ev031".into(),
        event_time: 1_700_000_010,
        event: SlackEvent::Message(MessageEvent {
            channel: "C001".into(),
            user: None,                // absent at top level for message_changed
            text: None,                // absent at top level for message_changed
            ts: "Ev031.000200".into(), // event notification ts — NOT the message ts
            thread_ts: None,
            subtype: Some("message_changed".into()),
            message: Some(Box::new(MessageUpdate {
                user: Some("U001".into()),
                text: Some("edited text".into()),
                ts: original_ts.clone(),
                thread_ts: None,
                edited: None,
            })),
        }),
    };
    handle_event(&repo, None, "", cb_edit, serde_json::json!({}))
        .await
        .unwrap();

    let store = repo.messages.lock().unwrap();
    // Still only one message record — upserted by the original ts
    assert_eq!(store.len(), 1);
    let (_, msg) = store.get(&("C001".into(), original_ts)).unwrap();
    assert_eq!(msg.text, "edited text");
}

#[tokio::test]
async fn test_reaction_is_stored() {
    let repo = InMemoryRepository::default();
    let (cb, raw) = make_reaction_cb("Ev010");

    handle_event(&repo, None, "", cb, raw).await.unwrap();

    assert!(repo.event_exists("Ev010").await.unwrap());
    assert_eq!(repo.reactions.lock().unwrap().len(), 1);
    assert_eq!(repo.weekly_upserts.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_duplicate_reaction_is_idempotent() {
    let repo = InMemoryRepository::default();
    let (cb1, raw1) = make_reaction_cb("Ev011");
    let (cb2, raw2) = make_reaction_cb("Ev012"); // different event_id, same reaction

    handle_event(&repo, None, "", cb1, raw1).await.unwrap();
    handle_event(&repo, None, "", cb2, raw2).await.unwrap();

    // Both events recorded, but reaction deduped (same team/channel/ts/user/name)
    assert_eq!(repo.reactions.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_unknown_event_type_is_acked_not_stored() {
    let repo = InMemoryRepository::default();
    let (cb, raw) = make_unknown_cb("Ev020");

    handle_event(&repo, None, "", cb, raw).await.unwrap();

    // Event recorded for dedup, but nothing stored in messages or reactions
    assert!(repo.event_exists("Ev020").await.unwrap());
    assert_eq!(repo.messages.lock().unwrap().len(), 0);
    assert_eq!(repo.reactions.lock().unwrap().len(), 0);
}
