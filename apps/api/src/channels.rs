use crate::{AppState, auth::SessionClaims};
use axum::{Extension, Json, extract::State};
use domain::{Channel, ChannelKind, File, Message, Reaction};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ChannelSummaryResponse {
    id: String,
    name: Option<String>,
    kind: &'static str,
    is_archived: bool,
    message_count: usize,
    reaction_count: usize,
    file_count: usize,
    last_message_ts: Option<String>,
}

#[derive(Debug)]
struct ChannelSummaryBuilder {
    id: String,
    name: Option<String>,
    kind: ChannelKind,
    is_archived: bool,
    message_count: usize,
    reaction_count: usize,
    file_count: usize,
    last_message_ts: Option<String>,
}

impl Default for ChannelSummaryBuilder {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: None,
            kind: ChannelKind::Unknown,
            is_archived: false,
            message_count: 0,
            reaction_count: 0,
            file_count: 0,
            last_message_ts: None,
        }
    }
}

pub(crate) async fn channels(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
) -> Json<Vec<ChannelSummaryResponse>> {
    Json(
        build_channel_summaries(
            state
                .store
                .channels()
                .await
                .into_iter()
                .filter(|channel| channel.team_id == claims.team_id)
                .collect(),
            state
                .store
                .messages()
                .await
                .into_iter()
                .filter(|message| message.team_id == claims.team_id)
                .collect(),
            state
                .store
                .reactions()
                .await
                .into_iter()
                .filter(|reaction| reaction.team_id == claims.team_id)
                .collect(),
            state
                .store
                .files()
                .await
                .into_iter()
                .filter(|file| file.team_id == claims.team_id)
                .collect(),
        )
        .into_iter()
        .map(|summary| ChannelSummaryResponse {
            id: summary.id,
            name: summary.name,
            kind: summary.kind.as_str(),
            is_archived: summary.is_archived,
            message_count: summary.message_count,
            reaction_count: summary.reaction_count,
            file_count: summary.file_count,
            last_message_ts: summary.last_message_ts,
        })
        .collect(),
    )
}

fn build_channel_summaries(
    channels: Vec<Channel>,
    messages: Vec<Message>,
    reactions: Vec<Reaction>,
    files: Vec<File>,
) -> Vec<ChannelSummaryBuilder> {
    let mut summaries = HashMap::<String, ChannelSummaryBuilder>::new();

    for channel in channels {
        let summary =
            summaries
                .entry(channel.id.clone())
                .or_insert_with(|| ChannelSummaryBuilder {
                    id: channel.id.clone(),
                    kind: channel.kind,
                    ..ChannelSummaryBuilder::default()
                });
        summary.name = channel.name;
        summary.kind = channel.kind;
        summary.is_archived = channel.is_archived;
    }

    for message in messages {
        let summary = summaries
            .entry(message.channel_id.clone())
            .or_insert_with(|| inferred_channel_summary(&message.channel_id));
        summary.message_count += 1;
        if summary
            .last_message_ts
            .as_deref()
            .is_none_or(|current| current < message.ts.as_str())
        {
            summary.last_message_ts = Some(message.ts);
        }
    }

    for reaction in reactions {
        let summary = summaries
            .entry(reaction.channel_id.clone())
            .or_insert_with(|| inferred_channel_summary(&reaction.channel_id));
        summary.reaction_count += 1;
    }

    for file in files {
        let summary = summaries
            .entry(file.channel_id.clone())
            .or_insert_with(|| inferred_channel_summary(&file.channel_id));
        summary.file_count += 1;
        if summary
            .last_message_ts
            .as_deref()
            .is_none_or(|current| current < file.message_ts.as_str())
        {
            summary.last_message_ts = Some(file.message_ts);
        }
    }

    let mut summaries = summaries.into_values().collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        (
            right.last_message_ts.as_deref(),
            left.is_archived,
            left.name.as_deref(),
            left.id.as_str(),
        )
            .cmp(&(
                left.last_message_ts.as_deref(),
                right.is_archived,
                right.name.as_deref(),
                right.id.as_str(),
            ))
    });
    summaries
}

fn inferred_channel_summary(channel_id: &str) -> ChannelSummaryBuilder {
    ChannelSummaryBuilder {
        id: channel_id.to_owned(),
        kind: ChannelKind::from_channel_id(channel_id),
        ..ChannelSummaryBuilder::default()
    }
}

#[cfg(test)]
#[path = "channels_tests.rs"]
mod tests;
