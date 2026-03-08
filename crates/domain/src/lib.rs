use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMode {
    SingleWorkspace,
}

impl WorkspaceMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleWorkspace => "single_workspace",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Public,
    Private,
    Direct,
    Unknown,
}

impl ChannelKind {
    pub fn from_channel_id(channel_id: &str) -> Self {
        match channel_id.chars().next() {
            Some('C') => Self::Public,
            Some('G') => Self::Private,
            Some('D') => Self::Direct,
            _ => Self::Unknown,
        }
    }

    pub fn from_channel_type(channel_type: &str) -> Self {
        match channel_type {
            "channel" | "public_channel" => Self::Public,
            "group" | "private_channel" => Self::Private,
            "im" | "mpim" => Self::Direct,
            _ => Self::Unknown,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Direct => "direct",
            Self::Unknown => "unknown",
        }
    }

    pub const fn is_public(self) -> bool {
        matches!(self, Self::Public)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub team_id: String,
    pub channel_id: String,
    pub ts: String,
    pub thread_ts: Option<String>,
    pub user_id: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reaction {
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub user_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct File {
    pub id: String,
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub name: String,
    pub mimetype: Option<String>,
    pub permalink: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedFile {
    pub id: String,
    pub name: String,
    pub mimetype: Option<String>,
    pub permalink: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thread {
    pub channel_id: String,
    pub root_ts: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    pub team_id: String,
    pub id: String,
    pub kind: ChannelKind,
    pub name: Option<String>,
    pub is_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub team_id: String,
    pub id: String,
    pub display_name: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EventPayload {
    Message {
        user_id: Option<String>,
        text: Option<String>,
        ts: String,
        thread_ts: Option<String>,
        files: Vec<SharedFile>,
    },
    ReactionAdded {
        user_id: String,
        reaction: String,
        item_ts: String,
    },
    ChannelUpdated {
        name: Option<String>,
        is_archived: Option<bool>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessEventJob {
    pub event_id: String,
    pub team_id: String,
    pub event_time: i64,
    pub received_at: i64,
    pub channel_id: String,
    pub channel_kind: ChannelKind,
    pub payload: EventPayload,
}

impl ProcessEventJob {
    pub const fn targets_public_channel(&self) -> bool {
        self.channel_kind.is_public()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Channel, ChannelKind, EventPayload, File, Message, ProcessEventJob, Reaction, SharedFile,
        Thread, User, WorkspaceMode,
    };

    #[test]
    fn workspace_mode_is_single_workspace() {
        assert_eq!(WorkspaceMode::SingleWorkspace.as_str(), "single_workspace");
    }

    #[test]
    fn channel_kind_can_be_inferred_from_ids_and_types() {
        assert_eq!(ChannelKind::from_channel_id("C123"), ChannelKind::Public);
        assert_eq!(ChannelKind::from_channel_id("G123"), ChannelKind::Private);
        assert_eq!(
            ChannelKind::from_channel_type("public_channel"),
            ChannelKind::Public
        );
        assert_eq!(ChannelKind::from_channel_type("im"), ChannelKind::Direct);
    }

    #[test]
    fn process_event_jobs_report_public_scope() {
        let job = ProcessEventJob {
            event_id: "evt_1".to_owned(),
            team_id: "team_1".to_owned(),
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
        };

        assert!(job.targets_public_channel());
    }

    #[test]
    fn core_entities_are_constructible_for_single_workspace_scope() {
        let channel = Channel {
            team_id: "T123".to_owned(),
            id: "C123".to_owned(),
            kind: ChannelKind::Public,
            name: Some("general".to_owned()),
            is_archived: false,
        };
        let user = User {
            team_id: "T123".to_owned(),
            id: "U123".to_owned(),
            display_name: Some("Thomas".to_owned()),
            is_active: true,
        };
        let thread = Thread {
            channel_id: channel.id.clone(),
            root_ts: "1700000000.000001".to_owned(),
        };
        let message = Message {
            team_id: user.team_id.clone(),
            channel_id: channel.id.clone(),
            ts: thread.root_ts.clone(),
            thread_ts: Some(thread.root_ts.clone()),
            user_id: Some(user.id.clone()),
            text: "hello".to_owned(),
        };
        let reaction = Reaction {
            team_id: user.team_id,
            channel_id: channel.id,
            message_ts: thread.root_ts,
            user_id: user.id,
            name: "thumbsup".to_owned(),
        };
        let file = File {
            id: "F123".to_owned(),
            team_id: "T123".to_owned(),
            channel_id: "C123".to_owned(),
            message_ts: "1700000000.000001".to_owned(),
            name: "brief.pdf".to_owned(),
            mimetype: Some("application/pdf".to_owned()),
            permalink: Some("https://files.example.com/brief.pdf".to_owned()),
            size: Some(42),
        };
        let shared_file = SharedFile {
            id: file.id.clone(),
            name: file.name.clone(),
            mimetype: file.mimetype.clone(),
            permalink: file.permalink.clone(),
            size: file.size,
        };

        assert_eq!(message.text, "hello");
        assert_eq!(reaction.name, "thumbsup");
        assert_eq!(channel.name.as_deref(), Some("general"));
        assert_eq!(shared_file.name, "brief.pdf");
    }
}
