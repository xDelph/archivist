use super::EventCallback;
use domain::{ChannelKind, EventPayload, SharedFile};

#[test]
fn public_messages_become_jobs() {
    let callback: EventCallback = serde_json::from_str(
        r#"{
            "event_id": "Ev123",
            "event_time": 1700000000,
            "event": {
                "type": "message",
                "channel": "C123",
                "channel_type": "channel",
                "user": "U123",
                "text": "hello",
                "ts": "1700000000.000001",
                "thread_ts": null,
                "subtype": null
            }
        }"#,
    )
    .expect("callback");

    let job = callback.into_job(1_700_000_005).expect("job");

    assert_eq!(job.channel_kind, ChannelKind::Public);
    assert_eq!(
        job.payload,
        EventPayload::Message {
            user_id: Some("U123".to_owned()),
            text: Some("hello".to_owned()),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            files: vec![],
        }
    );
}

#[test]
fn public_file_share_messages_become_jobs_with_files() {
    let callback: EventCallback = serde_json::from_str(
        r#"{
            "event_id": "Ev321",
            "event_time": 1700000300,
            "event": {
                "type": "message",
                "channel": "C123",
                "channel_type": "channel",
                "user": "U123",
                "text": "uploaded brief",
                "ts": "1700000000.000002",
                "thread_ts": null,
                "subtype": "file_share",
                "files": [
                    {
                        "id": "F123",
                        "name": "brief.pdf",
                        "mimetype": "application/pdf",
                        "permalink": "https://files.example.com/brief.pdf",
                        "size": 42
                    }
                ]
            }
        }"#,
    )
    .expect("callback");

    let job = callback.into_job(1_700_000_305).expect("job");

    assert_eq!(job.channel_kind, ChannelKind::Public);
    assert_eq!(
        job.payload,
        EventPayload::Message {
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
        }
    );
}

#[test]
fn public_reactions_become_jobs() {
    let callback: EventCallback = serde_json::from_str(
        r#"{
            "event_id": "Ev456",
            "event_time": 1700000100,
            "event": {
                "type": "reaction_added",
                "reaction": "thumbsup",
                "user": "U123",
                "item": {
                    "channel": "C123",
                    "ts": "1700000000.000001"
                }
            }
        }"#,
    )
    .expect("callback");

    let job = callback.into_job(1_700_000_105).expect("job");

    assert_eq!(job.channel_kind, ChannelKind::Public);
    assert_eq!(
        job.payload,
        EventPayload::ReactionAdded {
            user_id: "U123".to_owned(),
            reaction: "thumbsup".to_owned(),
            item_ts: "1700000000.000001".to_owned(),
        }
    );
}

#[test]
fn public_channel_rename_becomes_job() {
    let callback: EventCallback = serde_json::from_str(
        r#"{
            "event_id": "Ev654",
            "event_time": 1700000400,
            "event": {
                "type": "channel_rename",
                "channel": {
                    "id": "C123",
                    "name": "announcements"
                }
            }
        }"#,
    )
    .expect("callback");

    let job = callback.into_job(1_700_000_405).expect("job");

    assert_eq!(job.channel_kind, ChannelKind::Public);
    assert_eq!(
        job.payload,
        EventPayload::ChannelUpdated {
            name: Some("announcements".to_owned()),
            is_archived: None,
        }
    );
}

#[test]
fn public_channel_archive_becomes_job() {
    let callback: EventCallback = serde_json::from_str(
        r#"{
            "event_id": "Ev655",
            "event_time": 1700000500,
            "event": {
                "type": "channel_archive",
                "channel": "C123"
            }
        }"#,
    )
    .expect("callback");

    let job = callback.into_job(1_700_000_505).expect("job");

    assert_eq!(job.channel_kind, ChannelKind::Public);
    assert_eq!(
        job.payload,
        EventPayload::ChannelUpdated {
            name: None,
            is_archived: Some(true),
        }
    );
}

#[test]
fn non_public_messages_are_ignored() {
    let callback: EventCallback = serde_json::from_str(
        r#"{
            "event_id": "Ev789",
            "event_time": 1700000200,
            "event": {
                "type": "message",
                "channel": "D123",
                "user": "U123",
                "text": "private",
                "ts": "1700000000.000001",
                "thread_ts": null,
                "subtype": null
            }
        }"#,
    )
    .expect("callback");

    assert!(callback.into_job(1_700_000_205).is_none());
}
