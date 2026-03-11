use super::InMemoryEventStore;
use crate::StoreOutcome;
use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};

#[tokio::test]
async fn in_memory_store_deduplicates_and_tracks_entities() {
    let store = InMemoryEventStore::new();
    let rename = ProcessEventJob {
        event_id: "evt_channel".to_owned(),
        event_time: 1,
        received_at: 2,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::ChannelUpdated {
            name: Some("announcements".to_owned()),
            is_archived: Some(false),
        },
    };
    let message = ProcessEventJob {
        event_id: "evt_message".to_owned(),
        event_time: 3,
        received_at: 4,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::Message {
            user_id: Some("U123".to_owned()),
            text: Some("hello".to_owned()),
            ts: "1700000000.000001".to_owned(),
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

    assert_eq!(
        store.record_process_event(&rename).await,
        StoreOutcome::Inserted
    );
    assert_eq!(
        store.record_process_event(&message).await,
        StoreOutcome::Inserted
    );
    assert_eq!(
        store.record_process_event(&message).await,
        StoreOutcome::Duplicate
    );

    let health = store.health().await;
    let files = store.files().await;
    let channels = store.channels().await;

    assert_eq!(health.tracked_events, 2);
    assert_eq!(health.tracked_messages, 1);
    assert_eq!(health.tracked_files, 1);
    assert_eq!(health.tracked_channels, 1);
    assert_eq!(files[0].name, "brief.pdf");
    assert_eq!(channels[0].name.as_deref(), Some("announcements"));
}

#[tokio::test]
async fn in_memory_store_tracks_reactions() {
    let store = InMemoryEventStore::new();
    let reaction = ProcessEventJob {
        event_id: "evt_reaction".to_owned(),
        event_time: 1,
        received_at: 2,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::ReactionAdded {
            user_id: "U123".to_owned(),
            reaction: "thumbsup".to_owned(),
            item_ts: "1700000000.000001".to_owned(),
        },
    };

    assert_eq!(
        store.record_process_event(&reaction).await,
        StoreOutcome::Inserted
    );

    let reactions = store.reactions().await;
    let health = store.health().await;

    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0].name, "thumbsup");
    assert_eq!(health.tracked_reactions, 1);
}

#[tokio::test]
async fn in_memory_store_refreshes_search_documents_for_threads() {
    let store = InMemoryEventStore::new();
    let reply = ProcessEventJob {
        event_id: "evt_reply".to_owned(),
        event_time: 5,
        received_at: 6,
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
    let root = ProcessEventJob {
        event_id: "evt_root".to_owned(),
        event_time: 7,
        received_at: 8,
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

    assert_eq!(
        store.record_process_event(&reply).await,
        StoreOutcome::Inserted
    );
    assert_eq!(
        store.record_process_event(&root).await,
        StoreOutcome::Inserted
    );

    let search_documents = store.search_documents().await;

    assert_eq!(search_documents.len(), 2);
    assert!(
        search_documents
            .iter()
            .all(|document| document.title.as_deref() == Some("root summary"))
    );
}

#[tokio::test]
async fn in_memory_store_refreshes_thread_summaries_for_threads() {
    let store = InMemoryEventStore::new();
    let reply = ProcessEventJob {
        event_id: "evt_reply".to_owned(),
        event_time: 5,
        received_at: 6,
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
    let root = ProcessEventJob {
        event_id: "evt_root".to_owned(),
        event_time: 7,
        received_at: 8,
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
    let reaction = ProcessEventJob {
        event_id: "evt_reaction".to_owned(),
        event_time: 9,
        received_at: 10,
        channel_id: "C123".to_owned(),
        channel_kind: ChannelKind::Public,
        payload: EventPayload::ReactionAdded {
            user_id: "U789".to_owned(),
            reaction: "eyes".to_owned(),
            item_ts: "1700000000.000002".to_owned(),
        },
    };

    assert_eq!(
        store.record_process_event(&reply).await,
        StoreOutcome::Inserted
    );
    assert_eq!(
        store.record_process_event(&root).await,
        StoreOutcome::Inserted
    );
    assert_eq!(
        store.record_process_event(&reaction).await,
        StoreOutcome::Inserted
    );

    let thread_summaries = store.thread_summaries().await;

    assert_eq!(thread_summaries.len(), 1);
    assert_eq!(thread_summaries[0].title, "root summary");
    assert_eq!(thread_summaries[0].reply_count, 1);
    assert_eq!(thread_summaries[0].participant_count, 3);
    assert_eq!(thread_summaries[0].reaction_count, 1);
    assert_eq!(thread_summaries[0].file_count, 1);
}
