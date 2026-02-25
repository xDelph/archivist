use crate::db::{InMemoryRepository, MessageRecord, ReactionRecord, Repository, SlackEventRecord};

fn raw_payload() -> serde_json::Value {
    serde_json::json!({"type": "event_callback"})
}

fn make_message(channel_id: &str, ts: &str) -> MessageRecord {
    MessageRecord {
        team_id: "T001".into(),
        channel_id: channel_id.into(),
        ts: ts.into(),
        thread_ts: None,
        user_id: None,
        text: String::new(),
        subtype: None,
        edited_ts: None,
        deleted: false,
        raw_json: serde_json::json!({}),
    }
}

#[tokio::test]
async fn test_event_exists_false_then_true() {
    let repo = InMemoryRepository::default();
    assert!(!repo.event_exists("Ev001").await.unwrap());

    repo.insert_slack_event(SlackEventRecord {
        event_id: "Ev001",
        team_id: "T001",
        event_time: 1_700_000_000,
        payload_json: &raw_payload(),
    })
    .await
    .unwrap();

    assert!(repo.event_exists("Ev001").await.unwrap());
}

#[tokio::test]
async fn test_insert_slack_event_idempotent() {
    let repo = InMemoryRepository::default();
    for _ in 0..2 {
        repo.insert_slack_event(SlackEventRecord {
            event_id: "Ev002",
            team_id: "T001",
            event_time: 1_700_000_001,
            payload_json: &raw_payload(),
        })
        .await
        .unwrap();
    }
    // Must not error and event must exist exactly once (set semantics)
    assert!(repo.event_exists("Ev002").await.unwrap());
}

#[tokio::test]
async fn test_upsert_message_insert_and_update() {
    let repo = InMemoryRepository::default();

    let msg = MessageRecord {
        text: "hello".into(),
        ..make_message("C001", "1700000000.000100")
    };
    let id = repo.upsert_message(&msg).await.unwrap();

    // Edit: same (channel_id, ts) → must return same id, updated text
    let msg_edited = MessageRecord {
        text: "hello edited".into(),
        edited_ts: Some("1700000001.000000".into()),
        ..make_message("C001", "1700000000.000100")
    };
    let id2 = repo.upsert_message(&msg_edited).await.unwrap();
    assert_eq!(id, id2);

    // Verify text was updated
    let text = repo
        .messages
        .lock()
        .unwrap()
        .get(&("C001".into(), "1700000000.000100".into()))
        .map(|(_, m)| m.text.clone())
        .unwrap();
    assert_eq!(text, "hello edited");
}

#[tokio::test]
async fn test_insert_reaction_idempotent() {
    let repo = InMemoryRepository::default();
    let make_reaction = || ReactionRecord {
        team_id: "T001".into(),
        channel_id: "C001".into(),
        message_ts: "1700000000.000100".into(),
        user_id: "U001".into(),
        reaction_name: "thumbsup".into(),
        event_ts: "1700000001.000000".into(),
    };

    repo.insert_reaction(&make_reaction()).await.unwrap();
    repo.insert_reaction(&make_reaction()).await.unwrap(); // duplicate — must not error

    let count = repo.reactions.lock().unwrap().len();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_get_last_archived_ts_empty() {
    let repo = InMemoryRepository::default();
    assert!(repo.get_last_archived_ts("C001").await.unwrap().is_none());
}

#[tokio::test]
async fn test_get_last_archived_ts_returns_max() {
    let repo = InMemoryRepository::default();
    for ts in [
        "1700000000.000100",
        "1700000000.000200",
        "1700000000.000050",
    ] {
        repo.upsert_message(&make_message("C001", ts))
            .await
            .unwrap();
    }
    let last = repo.get_last_archived_ts("C001").await.unwrap();
    assert_eq!(last.as_deref(), Some("1700000000.000200"));
}

#[tokio::test]
async fn test_upsert_thread_weekly_score_is_recorded() {
    let repo = InMemoryRepository::default();
    repo.upsert_thread_weekly_score("C001", "1700000000.000100")
        .await
        .unwrap();
    assert_eq!(
        repo.weekly_upserts.lock().unwrap().as_slice(),
        [("C001".to_owned(), "1700000000.000100".to_owned())]
    );
}
