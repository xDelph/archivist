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
#[path = "lib_tests.rs"]
mod tests;
