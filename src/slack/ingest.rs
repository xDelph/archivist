use anyhow::Result;

use crate::db::{MessageRecord, ReactionRecord, Repository, SlackEventRecord};
use crate::slack::types::{EventCallback, SlackEvent};

/// Message subtypes that carry no archivable content.
const IGNORED_SUBTYPES: &[&str] = &[
    "bot_message",
    "channel_join",
    "channel_leave",
    "channel_topic",
    "channel_purpose",
    "channel_name",
    "channel_archive",
    "channel_unarchive",
    "message_replied",
];

/// Processes a single Slack `event_callback` payload.
///
/// Steps:
/// 1. Dedup — if `event_id` already seen, return immediately (idempotent).
/// 2. Persist raw payload to `slack_events` for audit.
/// 3. Dispatch on event type: upsert message or insert reaction.
///
/// `raw_payload` is the original JSON value before typed deserialization,
/// stored verbatim in `slack_events.payload_json`.
pub async fn handle_event<R: Repository>(
    repo: &R,
    cb: EventCallback,
    raw_payload: serde_json::Value,
) -> Result<()> {
    // 1. Dedup
    if repo.event_exists(&cb.event_id).await? {
        return Ok(());
    }

    // 2. Persist raw event
    repo.insert_slack_event(SlackEventRecord {
        event_id: &cb.event_id,
        team_id: &cb.team_id,
        event_time: cb.event_time,
        payload_json: &raw_payload,
    })
    .await?;

    // 3. Dispatch
    match cb.event {
        SlackEvent::Message(m) => {
            if m.subtype
                .as_deref()
                .is_some_and(|s| IGNORED_SUBTYPES.contains(&s))
            {
                return Ok(());
            }
            repo.upsert_message(&MessageRecord {
                team_id: cb.team_id,
                channel_id: m.channel,
                ts: m.ts,
                thread_ts: m.thread_ts,
                user_id: m.user,
                text: m.text.unwrap_or_default(),
                subtype: m.subtype,
                edited_ts: None,
                deleted: false,
                raw_json: raw_payload,
            })
            .await?;
        }
        SlackEvent::ReactionAdded(r) => {
            repo.insert_reaction(&ReactionRecord {
                team_id: cb.team_id,
                channel_id: r.item.channel,
                message_ts: r.item.ts,
                user_id: r.user,
                reaction_name: r.reaction,
                event_ts: r.event_ts,
            })
            .await?;
        }
        SlackEvent::Unknown => {
            // Acknowledged and ignored — no storage needed.
        }
    }

    Ok(())
}
