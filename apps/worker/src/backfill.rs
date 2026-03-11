use super::backfill_slack::{SlackConversation, fetch_channel_history, fetch_public_channels};
use super::{AppState, ErrorResponse, store_failed};
use axum::{Json, body::Bytes, extract::State, http::StatusCode};
use db::StoreOutcome;
use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct BackfillChannelRequest {
    channel_id: Option<String>,
    cursor: Option<String>,
}

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
    sync_workspace_users(&state, slack_user_token).await?;

    if let Some(channel_id) = request
        .channel_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
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

    let mut totals = BackfillTotals::default();
    let total_channels = channels.len();
    for (index, channel) in channels.iter().enumerate() {
        tracing::info!(
            channel_index = index + 1,
            total_channels,
            channel_id = %channel.id,
            channel_name = channel.name.as_deref().unwrap_or(""),
            "starting workspace backfill channel"
        );
        let (channel_totals, _) =
            backfill_single_channel(&state, slack_user_token, &channel.id, Some(channel), None)
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
) -> Result<(BackfillTotals, Option<String>), (StatusCode, Json<ErrorResponse>)> {
    record_channel_metadata(state, channel_id, channel).await?;
    tracing::info!(
        channel_id,
        channel_name = channel
            .and_then(|value| value.name.as_deref())
            .unwrap_or(""),
        cursor = cursor.unwrap_or(""),
        "starting channel backfill"
    );
    let history = fetch_channel_history(
        &state.slack_api_base_url,
        slack_user_token,
        channel_id,
        cursor,
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

    let mut totals = BackfillTotals::default();
    let mut messages = history.messages.unwrap_or_default();
    messages.sort_by(|left, right| parse_event_time(&left.ts).cmp(&parse_event_time(&right.ts)));
    totals.files_seen = messages
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
        files = totals.files_seen,
        "persisting channel history payload"
    );

    for message in messages {
        let message_ts = message.ts.clone();
        let outcome = state
            .store
            .record_process_event(&ProcessEventJob {
                event_id: format!("backfill:{channel_id}:{message_ts}"),
                event_time: parse_event_time(&message_ts),
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
                    files: message
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
                        .collect(),
                },
            })
            .await
            .map_err(|error| {
                tracing::error!(
                    ?error,
                    channel_id,
                    message_ts = %message_ts,
                    "failed to persist backfill message"
                );
                store_failed(error)
            })?;

        match outcome {
            StoreOutcome::Inserted => totals.messages_inserted += 1,
            StoreOutcome::Duplicate => totals.messages_duplicate += 1,
        }

        for reaction in message.reactions {
            for user_id in reaction.users {
                let reaction_outcome = state
                    .store
                    .record_process_event(&ProcessEventJob {
                        event_id: format!(
                            "backfill:{channel_id}:{message_ts}:reaction:{}:{user_id}",
                            reaction.name
                        ),
                        event_time: parse_event_time(&message_ts),
                        received_at: current_unix_timestamp(),
                        channel_id: channel_id.to_owned(),
                        channel_kind: ChannelKind::from_channel_id(channel_id),
                        payload: EventPayload::ReactionAdded {
                            user_id,
                            reaction: reaction.name.clone(),
                            item_ts: message_ts.clone(),
                        },
                    })
                    .await
                    .map_err(|error| {
                        tracing::error!(
                            ?error,
                            channel_id,
                            message_ts = %message_ts,
                            reaction = %reaction.name,
                            "failed to persist backfill reaction"
                        );
                        store_failed(error)
                    })?;

                match reaction_outcome {
                    StoreOutcome::Inserted => totals.reactions_inserted += 1,
                    StoreOutcome::Duplicate => totals.reactions_duplicate += 1,
                }
            }
        }
    }

    let next_cursor = history
        .response_metadata
        .and_then(|metadata| metadata.next_cursor)
        .filter(|value| !value.trim().is_empty());
    tracing::info!(
        channel_id,
        inserted = totals.messages_inserted,
        duplicate = totals.messages_duplicate,
        reactions_inserted = totals.reactions_inserted,
        reactions_duplicate = totals.reactions_duplicate,
        files_seen = totals.files_seen,
        next_cursor = next_cursor.as_deref().unwrap_or(""),
        "completed channel backfill"
    );
    Ok((totals, next_cursor))
}

async fn sync_workspace_users(
    state: &AppState,
    slack_user_token: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if state.store.postgres_pool().is_none() {
        tracing::debug!("skipping workspace user sync for non-postgres store");
        return Ok(());
    }

    tracing::info!("starting workspace user sync");
    let users = fetch_workspace_users(&state.slack_api_base_url, slack_user_token).await?;
    tracing::info!(
        fetched_users = users.len(),
        "persisting fetched slack users"
    );
    let mut seen = HashSet::new();
    let mut synced = 0usize;
    let mut skipped_bots = 0usize;
    let mut skipped_duplicates = 0usize;
    for user in users {
        if user.is_bot.unwrap_or(false) || user.id == "USLACKBOT" {
            skipped_bots += 1;
            continue;
        }
        if !seen.insert(user.id.clone()) {
            skipped_duplicates += 1;
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
                "workspace user sync progress"
            );
        }
    }
    tracing::info!(
        synced_users = synced,
        skipped_bots,
        skipped_duplicates,
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

async fn record_channel_metadata(
    state: &AppState,
    channel_id: &str,
    channel: Option<&SlackConversation>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let event_time = current_unix_timestamp();
    let outcome = state
        .store
        .record_process_event(&ProcessEventJob {
            event_id: format!("backfill:channel:{channel_id}:metadata:{event_time}"),
            event_time,
            received_at: event_time,
            channel_id: channel_id.to_owned(),
            channel_kind: ChannelKind::from_channel_id(channel_id),
            payload: EventPayload::ChannelUpdated {
                name: channel
                    .and_then(|value| value.name.as_deref())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned),
                is_archived: Some(channel.and_then(|value| value.is_archived).unwrap_or(false)),
            },
        })
        .await
        .map_err(store_failed)?;

    tracing::debug!(
        channel_id,
        channel_name = channel
            .and_then(|value| value.name.as_deref())
            .unwrap_or(""),
        duplicate = matches!(outcome, StoreOutcome::Duplicate),
        "recorded channel metadata before history backfill"
    );
    Ok(())
}

fn resolve_channel(channel_id: &str, channels: &[SlackConversation]) -> Option<SlackConversation> {
    channels
        .iter()
        .find(|channel| channel.id == channel_id)
        .cloned()
}

fn parse_backfill_request(
    body: &Bytes,
) -> Result<BackfillChannelRequest, (StatusCode, Json<ErrorResponse>)> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(BackfillChannelRequest::default());
    }

    serde_json::from_slice(body).map_err(|error| {
        tracing::warn!(?error, "received invalid backfill request body");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_backfill_request",
            }),
        )
    })
}

#[cfg(test)]
#[path = "backfill_tests.rs"]
mod tests;
