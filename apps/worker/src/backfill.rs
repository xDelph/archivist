use super::backfill_archive::persist_backfill_file_archives;
use super::backfill_range::{
    parse_backfill_request, resolve_oldest_ts, resolve_user_sync_oldest_ts, should_sync_user,
};
use super::backfill_slack::{SlackConversation, fetch_channel_history, fetch_public_channels};
use super::backfill_threads::expand_thread_replies;
use super::{AppState, ErrorResponse, store_failed};
use axum::{Json, body::Bytes, extract::State, http::StatusCode};
use db::BackfillBatchStats;
use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct BackfillChannelResponse {
    ok: bool,
    inserted: usize,
    duplicate: usize,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackUser {
    id: String,
    deleted: Option<bool>,
    is_bot: Option<bool>,
    updated: Option<i64>,
    profile: SlackUserProfile,
}

#[derive(Debug, Deserialize)]
struct SlackUserProfile {
    email: Option<String>,
    display_name: Option<String>,
    real_name: Option<String>,
    image_72: Option<String>,
}

#[derive(Debug, Default)]
struct BackfillTotals {
    messages_inserted: usize,
    messages_duplicate: usize,
    reactions_inserted: usize,
    reactions_duplicate: usize,
    files_seen: usize,
    files_archived: usize,
}

#[derive(Debug, Clone, Copy)]
struct BackfillMode {
    resume_from_last_message_ts: bool,
    revisit_known_threads: bool,
}

pub(crate) async fn backfill_channel(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<BackfillChannelResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request = parse_backfill_request(&body)?;
    let slack_user_token = state.slack_user_token.as_deref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_slack_user_token",
        }),
    ))?;

    if let Some(channel_id) = request
        .channel_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let oldest_ts = resolve_oldest_ts(&state.store, &request, channel_id).await?;
        sync_workspace_users(&state, slack_user_token, oldest_ts.as_deref()).await?;
        let channel_catalog = fetch_public_channels(&state.slack_api_base_url, slack_user_token)
            .await
            .unwrap_or_default();
        let resolved_channel = resolve_channel(channel_id, &channel_catalog);
        let (totals, next_cursor) = backfill_single_channel(
            &state,
            slack_user_token,
            channel_id,
            resolved_channel.as_ref(),
            request.cursor.as_deref(),
            oldest_ts.as_deref(),
            BackfillMode {
                resume_from_last_message_ts: request.resume_from_last_message_ts,
                revisit_known_threads: true,
            },
        )
        .await?;
        return Ok(Json(BackfillChannelResponse {
            ok: true,
            inserted: totals.messages_inserted,
            duplicate: totals.messages_duplicate,
            next_cursor,
        }));
    }

    if request.cursor.is_some() {
        tracing::warn!("ignoring cursor because full public-channel backfill was requested");
    }

    tracing::info!("starting workspace-wide public channel backfill");
    let channels = fetch_public_channels(&state.slack_api_base_url, slack_user_token).await?;
    tracing::info!(
        channels = channels.len(),
        "resolved public channels for backfill"
    );
    let workspace_channel_ids = channels
        .iter()
        .map(|channel| channel.id.clone())
        .collect::<Vec<_>>();
    let user_sync_oldest_ts =
        resolve_user_sync_oldest_ts(&state.store, &request, &workspace_channel_ids).await?;
    sync_workspace_users(&state, slack_user_token, user_sync_oldest_ts.as_deref()).await?;

    let mut totals = BackfillTotals::default();
    let total_channels = channels.len();
    for (index, channel) in channels.iter().enumerate() {
        let oldest_ts = resolve_oldest_ts(&state.store, &request, &channel.id).await?;
        tracing::info!(
            channel_index = index + 1,
            total_channels,
            channel_id = %channel.id,
            channel_name = channel.name.as_deref().unwrap_or(""),
            oldest_ts = oldest_ts.as_deref().unwrap_or(""),
            "starting workspace backfill channel"
        );
        let (channel_totals, _) = backfill_single_channel(
            &state,
            slack_user_token,
            &channel.id,
            Some(channel),
            None,
            oldest_ts.as_deref(),
            BackfillMode {
                resume_from_last_message_ts: request.resume_from_last_message_ts,
                revisit_known_threads: true,
            },
        )
        .await?;
        totals.messages_inserted += channel_totals.messages_inserted;
        totals.messages_duplicate += channel_totals.messages_duplicate;
        totals.reactions_inserted += channel_totals.reactions_inserted;
        totals.reactions_duplicate += channel_totals.reactions_duplicate;
        totals.files_seen += channel_totals.files_seen;
    }

    tracing::info!(
        inserted = totals.messages_inserted,
        duplicate = totals.messages_duplicate,
        reactions_inserted = totals.reactions_inserted,
        reactions_duplicate = totals.reactions_duplicate,
        files_seen = totals.files_seen,
        "completed workspace-wide public channel backfill"
    );
    Ok(Json(BackfillChannelResponse {
        ok: true,
        inserted: totals.messages_inserted,
        duplicate: totals.messages_duplicate,
        next_cursor: None,
    }))
}

async fn backfill_single_channel(
    state: &AppState,
    slack_user_token: &str,
    channel_id: &str,
    channel: Option<&SlackConversation>,
    cursor: Option<&str>,
    oldest_ts: Option<&str>,
    mode: BackfillMode,
) -> Result<(BackfillTotals, Option<String>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        channel_id,
        channel_name = channel
            .and_then(|value| value.name.as_deref())
            .unwrap_or(""),
        cursor = cursor.unwrap_or(""),
        oldest_ts = oldest_ts.unwrap_or(""),
        "starting channel backfill"
    );
    let mut totals = BackfillTotals::default();
    let mut current_cursor = cursor.map(str::to_owned);
    let mut should_revisit_known_threads = mode.revisit_known_threads && oldest_ts.is_some();

    loop {
        let include_oldest =
            mode.resume_from_last_message_ts && current_cursor.is_none() && oldest_ts.is_some();
        let history = fetch_channel_history(
            &state.slack_api_base_url,
            slack_user_token,
            channel_id,
            current_cursor.as_deref(),
            oldest_ts,
            include_oldest,
        )
        .await?;
        tracing::info!(
            channel_id,
            fetched_messages = history.messages.as_ref().map_or(0, Vec::len),
            next_cursor = history
                .response_metadata
                .as_ref()
                .and_then(|metadata| metadata.next_cursor.as_deref())
                .unwrap_or(""),
            "fetched slack history page"
        );

        let mut messages = history.messages.unwrap_or_default();
        messages = expand_thread_replies(
            state,
            slack_user_token,
            channel_id,
            messages,
            should_revisit_known_threads,
            oldest_ts,
        )
        .await?;
        messages
            .sort_by(|left, right| parse_event_time(&left.ts).cmp(&parse_event_time(&right.ts)));
        let page_files_seen = messages
            .iter()
            .map(|message| message.files.len())
            .sum::<usize>();
        let reaction_users = messages
            .iter()
            .map(|message| {
                message
                    .reactions
                    .iter()
                    .map(|reaction| reaction.users.len())
                    .sum::<usize>()
            })
            .sum::<usize>();
        tracing::info!(
            channel_id,
            messages = messages.len(),
            reactions = reaction_users,
            files = page_files_seen,
            "persisting channel history payload"
        );
        let event_time = current_unix_timestamp();
        let channel_job = build_channel_job(channel_id, channel, event_time);
        let (message_jobs, reaction_jobs) = build_backfill_jobs(channel_id, &messages);
        let batch_stats = state
            .store
            .backfill_channel_jobs(channel_job.as_ref(), &message_jobs, &reaction_jobs)
            .await
            .map_err(store_failed)?;
        apply_batch_stats(&mut totals, batch_stats);
        totals.files_seen += page_files_seen;
        totals.files_archived +=
            persist_backfill_file_archives(state, slack_user_token, channel_id, &messages).await?;

        let next_cursor = history
            .response_metadata
            .and_then(|metadata| metadata.next_cursor)
            .filter(|value| !value.trim().is_empty());
        should_revisit_known_threads = false;
        let Some(next_cursor) = next_cursor else {
            break;
        };
        current_cursor = Some(next_cursor);
    }

    tracing::info!(
        channel_id,
        inserted = totals.messages_inserted,
        duplicate = totals.messages_duplicate,
        reactions_inserted = totals.reactions_inserted,
        reactions_duplicate = totals.reactions_duplicate,
        files_seen = totals.files_seen,
        files_archived = totals.files_archived,
        next_cursor = "",
        "completed channel backfill"
    );
    Ok((totals, None))
}

async fn sync_workspace_users(
    state: &AppState,
    slack_user_token: &str,
    updated_since: Option<&str>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if state.store.postgres_pool().is_none() {
        tracing::debug!("skipping workspace user sync for non-postgres store");
        return Ok(());
    }

    tracing::info!(
        updated_since = updated_since.unwrap_or(""),
        "starting workspace user sync"
    );
    let users = fetch_workspace_users(&state.slack_api_base_url, slack_user_token).await?;
    tracing::info!(
        fetched_users = users.len(),
        "persisting fetched slack users"
    );
    let mut seen = HashSet::new();
    let mut synced = 0usize;
    let mut skipped_bots = 0usize;
    let mut skipped_duplicates = 0usize;
    let mut skipped_stale = 0usize;
    for user in users {
        if user.is_bot.unwrap_or(false) || user.id == "USLACKBOT" {
            skipped_bots += 1;
            continue;
        }
        if !seen.insert(user.id.clone()) {
            skipped_duplicates += 1;
            continue;
        }
        if !should_sync_user(user.updated, updated_since) {
            skipped_stale += 1;
            continue;
        }
        let display_name = user
            .profile
            .display_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| {
                user.profile
                    .real_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
            });
        let avatar_url = user
            .profile
            .image_72
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        let email = user
            .profile
            .email
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        state
            .store
            .upsert_user_profile(
                &user.id,
                email,
                display_name.as_deref(),
                avatar_url.as_deref(),
                !user.deleted.unwrap_or(false),
            )
            .await
            .map_err(|error| {
                tracing::error!(?error, user_id = %user.id, "failed to upsert slack user");
                store_failed(error)
            })?;
        synced += 1;

        if synced == 1 || synced.is_multiple_of(250) {
            tracing::info!(
                synced_users = synced,
                skipped_bots,
                skipped_duplicates,
                skipped_stale,
                "workspace user sync progress"
            );
        }
    }
    tracing::info!(
        synced_users = synced,
        skipped_bots,
        skipped_duplicates,
        skipped_stale,
        "completed workspace user sync"
    );
    Ok(())
}

async fn fetch_workspace_users(
    slack_api_base_url: &str,
    slack_user_token: &str,
) -> Result<Vec<SlackUser>, (StatusCode, Json<ErrorResponse>)> {
    super::backfill_slack::fetch_workspace_users(slack_api_base_url, slack_user_token).await
}

fn parse_event_time(ts: &str) -> i64 {
    ts.split('.')
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

fn resolve_channel(channel_id: &str, channels: &[SlackConversation]) -> Option<SlackConversation> {
    channels
        .iter()
        .find(|channel| channel.id == channel_id)
        .cloned()
}

fn build_channel_job(
    channel_id: &str,
    channel: Option<&SlackConversation>,
    event_time: i64,
) -> Option<ProcessEventJob> {
    let channel_name = channel
        .and_then(|value| value.name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let is_archived = channel.and_then(|value| value.is_archived).unwrap_or(false);
    if channel_name.is_none() && !is_archived {
        return None;
    }

    Some(ProcessEventJob {
        event_id: format!("backfill:channel:{channel_id}:metadata:{event_time}"),
        event_time,
        received_at: event_time,
        channel_id: channel_id.to_owned(),
        channel_kind: ChannelKind::from_channel_id(channel_id),
        payload: EventPayload::ChannelUpdated {
            name: channel_name,
            is_archived: Some(is_archived),
        },
    })
}

fn build_backfill_jobs(
    channel_id: &str,
    messages: &[super::backfill_slack::SlackHistoryMessage],
) -> (Vec<ProcessEventJob>, Vec<ProcessEventJob>) {
    let mut message_jobs = Vec::with_capacity(messages.len());
    let mut reaction_jobs = Vec::new();

    for message in messages.iter().cloned() {
        let message_ts = message.ts.clone();
        let event_time = parse_event_time(&message_ts);
        let files = message
            .files
            .into_iter()
            .map(|file| SharedFile {
                name: file
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| file.id.clone()),
                id: file.id,
                mimetype: file.mimetype,
                permalink: file.permalink,
                size: file.size,
            })
            .collect::<Vec<_>>();

        message_jobs.push(ProcessEventJob {
            event_id: format!("backfill:{channel_id}:{message_ts}"),
            event_time,
            received_at: current_unix_timestamp(),
            channel_id: channel_id.to_owned(),
            channel_kind: ChannelKind::from_channel_id(channel_id),
            payload: EventPayload::Message {
                user_id: message.user,
                text: message.text,
                ts: message_ts.clone(),
                thread_ts: message
                    .thread_ts
                    .filter(|thread_ts| thread_ts != &message_ts),
                files,
            },
        });

        for reaction in message.reactions {
            for user_id in reaction.users {
                reaction_jobs.push(ProcessEventJob {
                    event_id: format!(
                        "backfill:{channel_id}:{message_ts}:reaction:{}:{user_id}",
                        reaction.name
                    ),
                    event_time,
                    received_at: current_unix_timestamp(),
                    channel_id: channel_id.to_owned(),
                    channel_kind: ChannelKind::from_channel_id(channel_id),
                    payload: EventPayload::ReactionAdded {
                        user_id,
                        reaction: reaction.name.clone(),
                        item_ts: message_ts.clone(),
                    },
                });
            }
        }
    }

    (message_jobs, reaction_jobs)
}

fn apply_batch_stats(totals: &mut BackfillTotals, stats: BackfillBatchStats) {
    totals.messages_inserted += stats.messages_inserted;
    totals.messages_duplicate += stats.messages_duplicate;
    totals.reactions_inserted += stats.reactions_inserted;
    totals.reactions_duplicate += stats.reactions_duplicate;
}

#[cfg(test)]
#[path = "backfill_tests.rs"]
mod tests;
