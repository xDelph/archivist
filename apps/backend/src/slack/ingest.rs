use std::collections::HashSet;

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::db::{MessageRecord, ReactionRecord, Repository, SlackEventRecord, UserRecord};
use crate::slack::backfill::{SlackApi, SlackClient, SlackError, SlackMessage, archive_files};
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
    "message_deleted",
];

#[derive(Default)]
struct IngestUserLookupCache {
    refreshed: HashSet<String>,
    skipped: HashSet<String>,
}

struct CanonicalMessage {
    ts: String,
    thread_ts: Option<String>,
    raw: serde_json::Value,
}

fn canonical_message_from_event(
    team_id: &str,
    event: &crate::slack::types::MessageEvent,
    raw_payload: &serde_json::Value,
) -> Option<CanonicalMessage> {
    let subtype = event.subtype.as_deref();
    let prefer_nested = matches!(subtype, Some("message_changed" | "message_replied"));
    let (ts, user_id, text, thread_ts, edited_ts) = if prefer_nested {
        if let Some(inner) = event.message.as_ref() {
            (
                inner.ts.clone(),
                inner.user.clone(),
                inner.text.clone().unwrap_or_default(),
                inner.thread_ts.clone(),
                inner.edited.as_ref().map(|edited| edited.ts.clone()),
            )
        } else {
            (
                event.ts.clone(),
                event.user.clone(),
                event.text.clone().unwrap_or_default(),
                event.thread_ts.clone(),
                None,
            )
        }
    } else {
        (
            event.ts.clone(),
            event.user.clone(),
            event.text.clone().unwrap_or_default(),
            event.thread_ts.clone(),
            None,
        )
    };
    if ts.trim().is_empty() {
        return None;
    }

    let source = if prefer_nested {
        &raw_payload["event"]["message"]
    } else {
        &raw_payload["event"]
    };
    let mut raw = if source.is_object() {
        source.clone()
    } else {
        serde_json::json!({})
    };

    raw["ts"] = serde_json::Value::String(ts.clone());
    raw["team"] = serde_json::Value::String(team_id.to_owned());
    raw["text"] = serde_json::Value::String(text);
    if let Some(user_id) = user_id {
        raw["user"] = serde_json::Value::String(user_id);
    }
    if let Some(thread_ts) = thread_ts.clone() {
        raw["thread_ts"] = serde_json::Value::String(thread_ts);
    }
    if let Some(subtype) = subtype {
        raw["subtype"] = serde_json::Value::String(subtype.to_owned());
    }
    if let Some(edited_ts) = edited_ts {
        raw["edited"] = serde_json::json!({ "ts": edited_ts });
    }

    Some(CanonicalMessage { ts, thread_ts, raw })
}

fn resolve_thread_root_ts(message: &CanonicalMessage, raw_payload: &serde_json::Value) -> String {
    message
        .thread_ts
        .clone()
        .filter(|thread_ts| !thread_ts.is_empty())
        .or_else(|| {
            raw_payload["event"]["message"]["thread_ts"]
                .as_str()
                .filter(|thread_ts| !thread_ts.is_empty())
                .map(str::to_owned)
        })
        .or_else(|| {
            raw_payload["event"]["message"]["ts"]
                .as_str()
                .filter(|thread_ts| !thread_ts.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| message.ts.clone())
}

fn should_sync_thread_snapshot(
    event: &crate::slack::types::MessageEvent,
    message: &CanonicalMessage,
    raw_payload: &serde_json::Value,
) -> bool {
    if event.subtype.as_deref() == Some("message_replied") {
        return true;
    }
    if let Some(thread_ts) = message.thread_ts.as_deref()
        && thread_ts != message.ts
    {
        return true;
    }
    raw_payload["event"]["message"]["latest_reply"]
        .as_str()
        .is_some()
        || raw_payload["event"]["message"]["reply_count"]
            .as_i64()
            .unwrap_or(0)
            > 0
}

async fn hydrate_user_profile<R, S>(
    repo: &R,
    client: &S,
    user_id: Option<&str>,
    cache: &mut IngestUserLookupCache,
) -> Result<()>
where
    R: Repository,
    S: SlackApi,
{
    let Some(user_id) = user_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    if cache.refreshed.contains(user_id) || cache.skipped.contains(user_id) {
        return Ok(());
    }

    match client.users_info(user_id).await {
        Ok(Some(user)) => {
            repo.upsert_user(&UserRecord {
                user_id: user.user_id.clone(),
                team_id: user.team_id,
                display_name: user.display_name,
                avatar_url: user.avatar_url,
            })
            .await?;
            cache.refreshed.insert(user.user_id);
        }
        Ok(None) => {
            cache.skipped.insert(user_id.to_owned());
        }
        Err(SlackError::Api(err))
            if err == "users_not_found" || err == "user_not_found" || err == "missing_scope" =>
        {
            if err == "missing_scope" {
                warn!(
                    user_id,
                    "users.info requires users:read scope — skipping user profile hydration"
                );
            }
            cache.skipped.insert(user_id.to_owned());
        }
        Err(err) => {
            warn!(user_id, error = %err, "users.info failed; skipping user profile hydration");
            cache.skipped.insert(user_id.to_owned());
        }
    }

    Ok(())
}

async fn upsert_slack_message<R, S>(
    repo: &R,
    client: Option<&S>,
    user_lookup_cache: &mut IngestUserLookupCache,
    channel_id: &str,
    fallback_team_id: &str,
    msg: &SlackMessage,
) -> Result<()>
where
    R: Repository,
    S: SlackApi,
{
    if let Some(client) = client {
        hydrate_user_profile(repo, client, msg.raw["user"].as_str(), user_lookup_cache).await?;
    }

    let team_id = msg.raw["team"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback_team_id)
        .to_owned();
    repo.upsert_message(&MessageRecord {
        team_id,
        channel_id: channel_id.to_owned(),
        ts: msg.ts.clone(),
        thread_ts: msg.thread_ts.clone(),
        user_id: msg.raw["user"].as_str().map(str::to_owned),
        text: msg.raw["text"].as_str().unwrap_or("").to_owned(),
        subtype: msg.raw["subtype"].as_str().map(str::to_owned),
        edited_ts: msg.raw["edited"]["ts"].as_str().map(str::to_owned),
        deleted: false,
        raw_json: msg.raw.clone(),
    })
    .await?;
    Ok(())
}

#[derive(Clone, Copy)]
struct ThreadSyncContext<'a> {
    storage: Option<&'a R2Client>,
    slack_token: &'a str,
    channel_id: &'a str,
    fallback_team_id: &'a str,
    thread_ts: &'a str,
}

async fn sync_thread_snapshot<R, S>(
    repo: &R,
    client: &S,
    user_lookup_cache: &mut IngestUserLookupCache,
    ctx: ThreadSyncContext<'_>,
) -> Result<usize>
where
    R: Repository,
    S: SlackApi,
{
    let mut cursor: Option<String> = None;
    let mut processed_messages = 0usize;
    loop {
        let (messages, next) = client
            .conversations_replies(ctx.channel_id, ctx.thread_ts, cursor.as_deref())
            .await?;
        if messages.is_empty() && next.is_none() {
            break;
        }
        for msg in &messages {
            upsert_slack_message(
                repo,
                Some(client),
                user_lookup_cache,
                ctx.channel_id,
                ctx.fallback_team_id,
                msg,
            )
            .await?;
            let team_id = msg.raw["team"]
                .as_str()
                .filter(|value| !value.is_empty())
                .unwrap_or(ctx.fallback_team_id);
            if let Err(err) = archive_files(
                repo,
                ctx.storage,
                ctx.slack_token,
                ctx.channel_id,
                &msg.ts,
                team_id,
                &msg.raw["files"],
            )
            .await
            {
                warn!(
                    channel_id = ctx.channel_id,
                    thread_ts = ctx.thread_ts,
                    message_ts = %msg.ts,
                    error = %err,
                    "failed to archive files while syncing thread snapshot"
                );
            }
            processed_messages += 1;
        }
        match next {
            Some(next_cursor) => cursor = Some(next_cursor),
            None => break,
        }
    }

    Ok(processed_messages)
}

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
    let slack_client = (!slack_token.is_empty()).then(|| SlackClient::new(slack_token.to_owned()));
    let mut user_lookup_cache = IngestUserLookupCache::default();

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
            let Some(canonical) = canonical_message_from_event(&cb.team_id, &m, &raw_payload)
            else {
                warn!(event_id = %cb.event_id, channel_id = %m.channel, "message event missing ts");
                return Ok(());
            };
            let canonical_msg = SlackMessage {
                ts: canonical.ts.clone(),
                thread_ts: canonical.thread_ts.clone(),
                raw: canonical.raw.clone(),
            };

            upsert_slack_message(
                repo,
                slack_client.as_ref(),
                &mut user_lookup_cache,
                &m.channel,
                &cb.team_id,
                &canonical_msg,
            )
            .await?;
            info!(
                event_id = %cb.event_id,
                channel_id = %m.channel,
                ts = %canonical_msg.ts,
                "message upserted"
            );

            repo.upsert_thread_weekly_score(&m.channel, &canonical_msg.ts)
                .await?;
            repo.enqueue_thread_aggregation(&m.channel, &canonical_msg.ts, "slack_event")
                .await?;

            let team_id = canonical_msg.raw["team"]
                .as_str()
                .filter(|value| !value.is_empty())
                .unwrap_or(&cb.team_id);
            if let Err(err) = archive_files(
                repo,
                storage,
                slack_token,
                &m.channel,
                &canonical_msg.ts,
                team_id,
                &canonical_msg.raw["files"],
            )
            .await
            {
                warn!(
                    event_id = %cb.event_id,
                    channel_id = %m.channel,
                    ts = %canonical_msg.ts,
                    error = %err,
                    "failed to archive files for event message"
                );
            }

            if let Some(client) = slack_client.as_ref()
                && should_sync_thread_snapshot(&m, &canonical, &raw_payload)
            {
                let thread_ts = resolve_thread_root_ts(&canonical, &raw_payload);
                match sync_thread_snapshot(
                    repo,
                    client,
                    &mut user_lookup_cache,
                    ThreadSyncContext {
                        storage,
                        slack_token,
                        channel_id: &m.channel,
                        fallback_team_id: &cb.team_id,
                        thread_ts: &thread_ts,
                    },
                )
                .await
                {
                    Ok(processed) => info!(
                        event_id = %cb.event_id,
                        channel_id = %m.channel,
                        thread_ts,
                        processed_messages = processed,
                        "thread snapshot synced from Slack"
                    ),
                    Err(err) => warn!(
                        event_id = %cb.event_id,
                        channel_id = %m.channel,
                        thread_ts,
                        error = %err,
                        "failed to sync thread snapshot from Slack"
                    ),
                }
            }
        }
        SlackEvent::ReactionAdded(r) => {
            info!(event_id = %cb.event_id, reaction = %r.reaction, "reaction inserted");
            if let Some(client) = slack_client.as_ref() {
                hydrate_user_profile(repo, client, Some(&r.user), &mut user_lookup_cache).await?;
            }
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
            repo.enqueue_thread_aggregation(&channel_id, &message_ts, "slack_event")
                .await?;
        }
        SlackEvent::Unknown => {
            // Acknowledged and ignored — no storage needed.
        }
    }

    Ok(())
}
