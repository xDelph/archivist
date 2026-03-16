use super::{GeneratedThreadSummaryRow, JsonlEventStore, StoreOutcome};
use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
use tempfile::tempdir;

fn sample_job(event_id: &str) -> ProcessEventJob {
    ProcessEventJob {
        event_id: event_id.to_owned(),
        event_time: 1,
        received_at: 2,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U123".to_owned()),
            text: Some("hello".to_owned()),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            files: vec![],
        },
    }
}

#[tokio::test]
async fn store_deduplicates_and_persists_ids() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");

    assert_eq!(
        store
            .record_process_event(&sample_job("evt_1"))
            .await
            .expect("insert"),
        StoreOutcome::Inserted
    );
    assert_eq!(
        store
            .record_process_event(&sample_job("evt_1"))
            .await
            .expect("duplicate"),
        StoreOutcome::Duplicate
    );

    let reopened = JsonlEventStore::open(&path).await.expect("reopened");
    let health = reopened.health().await;

    assert_eq!(health.tracked_events, 1);
    assert_eq!(health.tracked_messages, 1);
    assert_eq!(health.tracked_reactions, 0);
    assert_eq!(health.tracked_files, 0);
    assert_eq!(health.tracked_channels, 0);
}

#[tokio::test]
async fn message_jobs_upsert_by_channel_and_timestamp() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let original = sample_job("evt_1");
    let EventPayload::Message {
        user_id,
        ts,
        thread_ts,
        ..
    } = original.payload.clone()
    else {
        unreachable!("sample job is a message");
    };
    let updated = ProcessEventJob {
        event_id: "evt_2".to_owned(),
        payload: EventPayload::Message {
            user_id,
            text: Some("updated".to_owned()),
            ts,
            thread_ts,
            files: vec![],
        },
        ..original
    };

    store
        .record_process_event(&sample_job("evt_1"))
        .await
        .expect("insert original");
    store
        .record_process_event(&updated)
        .await
        .expect("insert updated");

    let messages = store.messages().await;

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "updated");
}

#[tokio::test]
async fn reaction_jobs_are_tracked_separately() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let reaction_job = ProcessEventJob {
        event_id: "evt_reaction".to_owned(),
        event_time: 3,
        received_at: 4,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::ReactionAdded {
            user_id: "U123".to_owned(),
            reaction: "thumbsup".to_owned(),
            item_ts: "1700000000.000001".to_owned(),
        },
    };

    store
        .record_process_event(&reaction_job)
        .await
        .expect("insert reaction");

    let reactions = store.reactions().await;
    let health = store.health().await;

    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0].name, "thumbsup");
    assert_eq!(health.tracked_reactions, 1);
}

#[tokio::test]
async fn file_share_messages_track_attached_files() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let file_job = ProcessEventJob {
        event_id: "evt_file".to_owned(),
        event_time: 5,
        received_at: 6,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U123".to_owned()),
            text: Some("uploaded brief".to_owned()),
            ts: "1700000000.000002".to_owned(),
            thread_ts: None,
            files: vec![SharedFile {
                id: "F123".to_owned(),
                name: "brief.pdf".to_owned(),
                mimetype: Some("application/pdf".to_owned()),
                permalink: Some("https://files.example.com/brief.pdf".to_owned()),
                size: Some(42),
            }],
        },
    };

    store
        .record_process_event(&file_job)
        .await
        .expect("insert file event");

    let files = store.files().await;
    let health = store.health().await;

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "brief.pdf");
    assert_eq!(health.tracked_files, 1);
}

#[tokio::test]
async fn channel_update_jobs_preserve_latest_name_and_archive_state() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let rename_job = ProcessEventJob {
        event_id: "evt_channel_rename".to_owned(),
        event_time: 7,
        received_at: 8,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::ChannelUpdated {
            name: Some("announcements".to_owned()),
            is_archived: None,
        },
    };
    let archive_job = ProcessEventJob {
        event_id: "evt_channel_archive".to_owned(),
        event_time: 9,
        received_at: 10,
        payload: EventPayload::ChannelUpdated {
            name: None,
            is_archived: Some(true),
        },
        ..rename_job.clone()
    };

    store
        .record_process_event(&rename_job)
        .await
        .expect("insert rename");
    store
        .record_process_event(&archive_job)
        .await
        .expect("insert archive");

    let channels = store.channels().await;
    let health = store.health().await;

    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].name.as_deref(), Some("announcements"));
    assert!(channels[0].is_archived);
    assert_eq!(health.tracked_channels, 1);
}

#[tokio::test]
async fn message_jobs_refresh_search_documents_for_full_threads() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let reply_job = ProcessEventJob {
        event_id: "evt_reply".to_owned(),
        event_time: 11,
        received_at: 12,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U456".to_owned()),
            text: Some("reply details".to_owned()),
            ts: "1700000000.000002".to_owned(),
            thread_ts: Some("1700000000.000001".to_owned()),
            files: vec![],
        },
    };
    let root_job = ProcessEventJob {
        event_id: "evt_root".to_owned(),
        event_time: 13,
        received_at: 14,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U123".to_owned()),
            text: Some("root summary".to_owned()),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            files: vec![],
        },
    };

    store
        .record_process_event(&reply_job)
        .await
        .expect("insert reply");
    store
        .record_process_event(&root_job)
        .await
        .expect("insert root");

    let search_documents = store.search_documents().await;
    assert_eq!(search_documents.len(), 2);
    assert!(
        search_documents
            .iter()
            .all(|document| document.title.as_deref() == Some("root summary"))
    );

    let reopened = JsonlEventStore::open(&path).await.expect("reopened");
    let reopened_documents = reopened.search_documents().await;
    assert_eq!(reopened_documents.len(), 2);
    assert!(
        reopened_documents
            .iter()
            .all(|document| document.title.as_deref() == Some("root summary"))
    );
}

#[tokio::test]
async fn message_jobs_refresh_thread_summaries_for_full_threads() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let reply_job = ProcessEventJob {
        event_id: "evt_reply".to_owned(),
        event_time: 11,
        received_at: 12,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U456".to_owned()),
            text: Some("reply details".to_owned()),
            ts: "1700000000.000002".to_owned(),
            thread_ts: Some("1700000000.000001".to_owned()),
            files: vec![SharedFile {
                id: "F123".to_owned(),
                name: "brief.pdf".to_owned(),
                mimetype: Some("application/pdf".to_owned()),
                permalink: None,
                size: Some(42),
            }],
        },
    };
    let root_job = ProcessEventJob {
        event_id: "evt_root".to_owned(),
        event_time: 13,
        received_at: 14,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U123".to_owned()),
            text: Some("root summary".to_owned()),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            files: vec![],
        },
    };
    let reaction_job = ProcessEventJob {
        event_id: "evt_reaction".to_owned(),
        event_time: 15,
        received_at: 16,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::ReactionAdded {
            user_id: "U789".to_owned(),
            reaction: "eyes".to_owned(),
            item_ts: "1700000000.000002".to_owned(),
        },
    };

    store
        .record_process_event(&reply_job)
        .await
        .expect("insert reply");
    store
        .record_process_event(&root_job)
        .await
        .expect("insert root");
    store
        .record_process_event(&reaction_job)
        .await
        .expect("insert reaction");

    let thread_summaries = store.thread_summaries().await;
    assert_eq!(thread_summaries.len(), 1);
    assert_eq!(thread_summaries[0].reply_count, 1);
    assert_eq!(thread_summaries[0].participant_count, 3);
    assert_eq!(thread_summaries[0].reaction_count, 1);
    assert_eq!(thread_summaries[0].file_count, 1);
    assert_eq!(thread_summaries[0].last_activity_ts, "1700000000.000002");

    let reopened = JsonlEventStore::open(&path).await.expect("reopened");
    let reopened_summaries = reopened.thread_summaries().await;
    assert_eq!(reopened_summaries.len(), 1);
}

#[tokio::test]
async fn generated_thread_summaries_persist_across_reopens() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("events.jsonl");
    let store = JsonlEventStore::open(&path).await.expect("store");
    let row = GeneratedThreadSummaryRow {
        channel_id: "C123".to_owned(),
        root_ts: "1700000000.000001".to_owned(),
        summary: "Launch plan summary".to_owned(),
        why_it_mattered: Some("It captured the agreed rollout steps.".to_owned()),
        status: "answered".to_owned(),
        topic_tags: vec!["launch".to_owned(), "rollout".to_owned()],
        source_last_activity_ts: "1700000000.000002".to_owned(),
        model: "openai/gpt-oss-120b:free".to_owned(),
        generated_at: 1_700_000_123,
    };

    store
        .upsert_generated_thread_summary(&row)
        .await
        .expect("persist generated summary");

    let stored = store.generated_thread_summaries().await;
    assert_eq!(stored, vec![row.clone()]);

    let reopened = JsonlEventStore::open(&path).await.expect("reopened");
    let reopened_summaries = reopened.generated_thread_summaries().await;
    assert_eq!(reopened_summaries, vec![row]);
}
