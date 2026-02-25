use anyhow::Result;
use tracing::{debug, info};

use crate::db::{MessageRecord, ReactionRecord, Repository, SlackEventRecord};
use crate::slack::types::{EventCallback, SlackEvent};
use crate::storage::R2Client;

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
    storage: Option<&R2Client>,
    slack_token: &str,
    cb: EventCallback,
    raw_payload: serde_json::Value,
) -> Result<()> {
    // 1. Dedup
    if repo.event_exists(&cb.event_id).await? {
        info!(event_id = %cb.event_id, "duplicate event, skipping");
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
                debug!(event_id = %cb.event_id, subtype = ?m.subtype, "ignored subtype");
                return Ok(());
            }

            // For message_changed the canonical ts/user/text live in the nested
            // `message` object; the top-level ts is just the event notification ts.
            let (ts, user_id, text, thread_ts, edited_ts) =
                if m.subtype.as_deref() == Some("message_changed") {
                    match m.message {
                        Some(inner) => (
                            inner.ts.clone(),
                            inner.user.clone(),
                            inner.text.clone().unwrap_or_default(),
                            inner.thread_ts.clone(),
                            inner.edited.as_ref().map(|e| e.ts.clone()),
                        ),
                        None => return Ok(()), // malformed event, nothing to update
                    }
                } else {
                    (
                        m.ts.clone(),
                        m.user.clone(),
                        m.text.unwrap_or_default(),
                        m.thread_ts.clone(),
                        None,
                    )
                };

            info!(event_id = %cb.event_id, channel_id = %m.channel, %ts, "message upserted");
            repo.upsert_message(&MessageRecord {
                team_id: cb.team_id.clone(),
                channel_id: m.channel.clone(),
                ts: ts.clone(),
                thread_ts,
                user_id,
                text,
                subtype: m.subtype,
                edited_ts,
                deleted: false,
                raw_json: raw_payload.clone(),
            })
            .await?;

            repo.upsert_thread_weekly_score(&m.channel, &ts).await?;

            // Archive any attached files to R2
            crate::slack::backfill::archive_files(
                repo,
                storage,
                slack_token,
                &m.channel,
                &ts,
                raw_payload["event"]["team"].as_str().unwrap_or(""),
                &raw_payload["event"]["files"],
            )
            .await?;
        }
        SlackEvent::ReactionAdded(r) => {
            info!(event_id = %cb.event_id, reaction = %r.reaction, "reaction inserted");
            let channel_id = r.item.channel.clone();
            let message_ts = r.item.ts.clone();
            repo.insert_reaction(&ReactionRecord {
                team_id: cb.team_id,
                channel_id: channel_id.clone(),
                message_ts: message_ts.clone(),
                user_id: r.user,
                reaction_name: r.reaction,
                event_ts: r.event_ts,
            })
            .await?;

            repo.upsert_thread_weekly_score(&channel_id, &message_ts)
                .await?;
        }
        SlackEvent::Unknown => {
            // Acknowledged and ignored — no storage needed.
        }
    }

    Ok(())
}
