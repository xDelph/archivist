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
#[path = "events_tests.rs"]
mod tests;
