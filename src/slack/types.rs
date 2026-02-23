use serde::{Deserialize, Serialize};

// ── Top-level envelope ────────────────────────────────────────────────────────

/// Top-level Slack payload. Dispatches on the `"type"` field.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackEnvelope {
    UrlVerification(UrlVerification),
    EventCallback(Box<EventCallback>),
    /// Any event type we don't handle — received, ack'd, then ignored.
    #[serde(other)]
    Unknown,
}

// ── url_verification ──────────────────────────────────────────────────────────

/// Sent by Slack when first configuring the Events API Request URL.
/// Must be echoed back as `{"challenge": "..."}`.
#[derive(Debug, Deserialize, Serialize)]
pub struct UrlVerification {
    pub challenge: String,
}

// ── event_callback ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EventCallback {
    pub team_id: String,
    pub api_app_id: String,
    pub event_id: String,
    pub event_time: i64,
    pub event: SlackEvent,
}

// ── Event variants ────────────────────────────────────────────────────────────

/// Inner event object inside an `event_callback`. Dispatches on `event.type`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackEvent {
    Message(MessageEvent),
    ReactionAdded(ReactionEvent),
    /// Any unhandled event type — ack'd and ignored.
    #[serde(other)]
    Unknown,
}

// ── message ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MessageEvent {
    pub channel: String,
    /// Absent on some subtypes (e.g. `bot_message` without a user).
    pub user: Option<String>,
    /// Absent on some subtypes (e.g. `message_deleted`).
    pub text: Option<String>,
    /// Slack message timestamp, unique within a channel.
    pub ts: String,
    /// Set when the message belongs to a thread; equals `ts` for the parent.
    pub thread_ts: Option<String>,
    /// Non-null for edited, deleted, bot messages, etc.
    pub subtype: Option<String>,
}

// ── reaction_added ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReactionEvent {
    pub reaction: String,
    pub user: String,
    pub item: ReactionItem,
    pub event_ts: String,
}

#[derive(Debug, Deserialize)]
pub struct ReactionItem {
    #[serde(rename = "type")]
    pub item_type: String,
    pub channel: String,
    pub ts: String,
}
