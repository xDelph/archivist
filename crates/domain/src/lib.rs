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
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EventPayload {
    Message {
        user_id: Option<String>,
        text: Option<String>,
        ts: String,
        thread_ts: Option<String>,
    },
    ReactionAdded {
        user_id: String,
        reaction: String,
        item_ts: String,
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
    use super::{ChannelKind, EventPayload, ProcessEventJob, WorkspaceMode};

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
            },
        };

        assert!(job.targets_public_channel());
    }
}
