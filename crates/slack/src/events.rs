use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackEnvelope {
    UrlVerification(UrlVerification),
    EventCallback(Box<EventCallback>),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UrlVerification {
    pub challenge: String,
}

#[derive(Debug, Deserialize)]
pub struct EventCallback {
    pub team_id: String,
    pub event_id: String,
    pub event_time: i64,
    pub event: SlackEvent,
}

impl EventCallback {
    pub fn into_job(self, received_at: i64) -> Option<ProcessEventJob> {
        match self.event {
            SlackEvent::Message(message) => {
                let channel_kind = message.channel_kind();
                if !channel_kind.is_public() || !message.supported_subtype() {
                    return None;
                }
                let shared_files = message.shared_files();

                Some(ProcessEventJob {
                    event_id: self.event_id,
                    team_id: self.team_id,
                    event_time: self.event_time,
                    received_at,
                    channel_id: message.channel,
                    channel_kind,
                    payload: EventPayload::Message {
                        user_id: message.user,
                        text: message.text,
                        ts: message.ts,
                        thread_ts: message.thread_ts,
                        files: shared_files,
                    },
                })
            }
            SlackEvent::ReactionAdded(reaction) => {
                let channel_kind = reaction.channel_kind();
                if !channel_kind.is_public() {
                    return None;
                }

                Some(ProcessEventJob {
                    event_id: self.event_id,
                    team_id: self.team_id,
                    event_time: self.event_time,
                    received_at,
                    channel_id: reaction.item.channel,
                    channel_kind,
                    payload: EventPayload::ReactionAdded {
                        user_id: reaction.user,
                        reaction: reaction.reaction,
                        item_ts: reaction.item.ts,
                    },
                })
            }
            SlackEvent::ChannelRename(rename) => Some(ProcessEventJob {
                event_id: self.event_id,
                team_id: self.team_id,
                event_time: self.event_time,
                received_at,
                channel_id: rename.channel.id,
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ChannelUpdated {
                    name: Some(rename.channel.name),
                    is_archived: None,
                },
            }),
            SlackEvent::ChannelArchive(archive) => Some(ProcessEventJob {
                event_id: self.event_id,
                team_id: self.team_id,
                event_time: self.event_time,
                received_at,
                channel_id: archive.channel,
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ChannelUpdated {
                    name: None,
                    is_archived: Some(true),
                },
            }),
            SlackEvent::ChannelUnarchive(unarchive) => Some(ProcessEventJob {
                event_id: self.event_id,
                team_id: self.team_id,
                event_time: self.event_time,
                received_at,
                channel_id: unarchive.channel,
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ChannelUpdated {
                    name: None,
                    is_archived: Some(false),
                },
            }),
            SlackEvent::Unknown => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackEvent {
    Message(MessageEvent),
    ReactionAdded(ReactionEvent),
    ChannelRename(ChannelRenameEvent),
    ChannelArchive(ChannelArchiveEvent),
    ChannelUnarchive(ChannelUnarchiveEvent),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct MessageEvent {
    pub channel: String,
    pub channel_type: Option<String>,
    pub user: Option<String>,
    pub text: Option<String>,
    pub ts: String,
    pub thread_ts: Option<String>,
    pub subtype: Option<String>,
    #[serde(default)]
    pub files: Vec<SlackFile>,
}

impl MessageEvent {
    fn channel_kind(&self) -> ChannelKind {
        self.channel_type
            .as_deref()
            .map(ChannelKind::from_channel_type)
            .unwrap_or_else(|| ChannelKind::from_channel_id(&self.channel))
    }

    fn supported_subtype(&self) -> bool {
        self.subtype
            .as_deref()
            .is_none_or(|subtype| subtype == "file_share")
    }

    fn shared_files(&self) -> Vec<SharedFile> {
        self.files
            .iter()
            .map(|file| SharedFile {
                id: file.id.clone(),
                name: file.name.clone(),
                mimetype: file.mimetype.clone(),
                permalink: file.permalink.clone(),
                size: file.size,
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
pub struct ReactionEvent {
    pub reaction: String,
    pub user: String,
    pub item: ReactionItem,
}

impl ReactionEvent {
    fn channel_kind(&self) -> ChannelKind {
        ChannelKind::from_channel_id(&self.item.channel)
    }
}

#[derive(Debug, Deserialize)]
pub struct ReactionItem {
    pub channel: String,
    pub ts: String,
}

#[derive(Debug, Deserialize)]
pub struct SlackFile {
    pub id: String,
    pub name: String,
    pub mimetype: Option<String>,
    pub permalink: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ChannelRenameEvent {
    pub channel: RenamedChannel,
}

#[derive(Debug, Deserialize)]
pub struct RenamedChannel {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ChannelArchiveEvent {
    pub channel: String,
}

#[derive(Debug, Deserialize)]
pub struct ChannelUnarchiveEvent {
    pub channel: String,
}

#[cfg(test)]
mod tests {
    use super::EventCallback;
    use domain::{ChannelKind, EventPayload, SharedFile};

    #[test]
    fn public_messages_become_jobs() {
        let callback: EventCallback = serde_json::from_str(
            r#"{
                "team_id": "T123",
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
                "team_id": "T123",
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
                "team_id": "T123",
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
                "team_id": "T123",
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
                "team_id": "T123",
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
                "team_id": "T123",
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
}
