use crate::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use domain::{Channel, File, Message, Reaction};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
pub(crate) struct ThreadListQuery {
    cursor: Option<String>,
    limit: Option<usize>,
    channel_id: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    sort: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ThreadListResponse {
    items: Vec<ThreadSummaryResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ThreadSummaryResponse {
    id: String,
    channel_id: String,
    channel_name: Option<String>,
    root_ts: String,
    title: String,
    preview: String,
    reply_count: usize,
    participant_count: usize,
    reaction_count: usize,
    file_count: usize,
    last_activity_ts: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreadSort {
    Activity,
    Newest,
}

impl ThreadSort {
    fn parse(value: Option<&str>) -> Option<Self> {
        match value.unwrap_or("activity") {
            "activity" => Some(Self::Activity),
            "newest" => Some(Self::Newest),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct ThreadSummary {
    id: String,
    channel_id: String,
    channel_name: Option<String>,
    root_ts: String,
    root_seconds: i64,
    title: String,
    preview: String,
    reply_count: usize,
    participants: HashSet<String>,
    reaction_count: usize,
    file_count: usize,
    last_activity_ts: String,
    last_activity_seconds: i64,
}

pub(crate) async fn thread_list(
    State(state): State<AppState>,
    Query(query): Query<ThreadListQuery>,
) -> Result<Json<ThreadListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let sort = ThreadSort::parse(query.sort.as_deref()).ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_sort",
        }),
    ))?;
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_cursor",
                }),
            )
        })?
        .unwrap_or_default();
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let items = build_thread_summaries(
        state.store.channels().await,
        state.store.messages().await,
        state.store.reactions().await,
        state.store.files().await,
        ThreadListFilters {
            channel_id: query.channel_id.as_deref(),
            date_from: query.date_from.as_deref(),
            date_to: query.date_to.as_deref(),
            sort,
        },
    );

    let page = items
        .iter()
        .skip(cursor)
        .take(limit)
        .map(|summary| ThreadSummaryResponse {
            id: summary.id.clone(),
            channel_id: summary.channel_id.clone(),
            channel_name: summary.channel_name.clone(),
            root_ts: summary.root_ts.clone(),
            title: summary.title.clone(),
            preview: summary.preview.clone(),
            reply_count: summary.reply_count,
            participant_count: summary.participants.len(),
            reaction_count: summary.reaction_count,
            file_count: summary.file_count,
            last_activity_ts: summary.last_activity_ts.clone(),
        })
        .collect::<Vec<_>>();
    let next_cursor =
        (cursor + page.len() < items.len()).then(|| (cursor + page.len()).to_string());

    Ok(Json(ThreadListResponse {
        items: page,
        next_cursor,
    }))
}

#[derive(Debug, Clone, Copy)]
struct ThreadListFilters<'a> {
    channel_id: Option<&'a str>,
    date_from: Option<&'a str>,
    date_to: Option<&'a str>,
    sort: ThreadSort,
}

fn build_thread_summaries(
    channels: Vec<Channel>,
    messages: Vec<Message>,
    reactions: Vec<Reaction>,
    files: Vec<File>,
    filters: ThreadListFilters<'_>,
) -> Vec<ThreadSummary> {
    let channel_filter = filters
        .channel_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let date_from = filters.date_from.and_then(parse_ts_seconds);
    let date_to = filters.date_to.and_then(parse_ts_seconds);
    let channel_names = channels
        .into_iter()
        .map(|channel| (channel.id, channel.name))
        .collect::<HashMap<_, _>>();
    let message_lookup = messages
        .iter()
        .map(|message| (message.ts.clone(), message))
        .collect::<HashMap<_, _>>();
    let root_messages = messages
        .iter()
        .filter(|message| message.thread_ts.is_none())
        .map(|message| (message.ts.clone(), message))
        .collect::<HashMap<_, _>>();
    let mut threads = HashMap::<(String, String), ThreadSummary>::new();

    for message in &messages {
        let root_ts = message
            .thread_ts
            .clone()
            .unwrap_or_else(|| message.ts.clone());
        if !root_messages.contains_key(&root_ts) {
            continue;
        }
        if channel_filter.is_some_and(|channel_id| channel_id != message.channel_id) {
            continue;
        }

        let entry = threads
            .entry((message.channel_id.clone(), root_ts.clone()))
            .or_insert_with(|| ThreadSummary {
                id: format!("{}:{root_ts}", message.channel_id),
                channel_id: message.channel_id.clone(),
                channel_name: channel_names
                    .get(&message.channel_id)
                    .cloned()
                    .unwrap_or_default(),
                root_ts: root_ts.clone(),
                root_seconds: parse_ts_seconds(&root_ts).unwrap_or_default(),
                title: summarize_text(root_text(&root_messages, &root_ts)),
                preview: summarize_text(root_text(&root_messages, &root_ts)),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: message.ts.clone(),
                last_activity_seconds: parse_ts_seconds(&message.ts).unwrap_or_default(),
            });

        if message.ts != root_ts {
            entry.reply_count += 1;
        }
        if let Some(user_id) = &message.user_id {
            entry.participants.insert(user_id.clone());
        }
        update_last_activity(
            entry,
            &message.ts,
            parse_ts_seconds(&message.ts).unwrap_or_default(),
        );
    }

    for reaction in reactions {
        let Some(message) = message_lookup.get(&reaction.message_ts) else {
            continue;
        };
        let root_ts = message
            .thread_ts
            .clone()
            .unwrap_or_else(|| message.ts.clone());
        if !root_messages.contains_key(&root_ts) {
            continue;
        }
        if channel_filter.is_some_and(|channel_id| channel_id != reaction.channel_id) {
            continue;
        }

        let entry = threads
            .entry((reaction.channel_id.clone(), root_ts.clone()))
            .or_insert_with(|| ThreadSummary {
                id: format!("{}:{root_ts}", reaction.channel_id),
                channel_id: reaction.channel_id.clone(),
                channel_name: channel_names
                    .get(&reaction.channel_id)
                    .cloned()
                    .unwrap_or_default(),
                root_ts: root_ts.clone(),
                root_seconds: parse_ts_seconds(&root_ts).unwrap_or_default(),
                title: summarize_text(&message.text),
                preview: summarize_text(&message.text),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: reaction.message_ts.clone(),
                last_activity_seconds: parse_ts_seconds(&reaction.message_ts).unwrap_or_default(),
            });

        entry.reaction_count += 1;
        entry.participants.insert(reaction.user_id);
        update_last_activity(
            entry,
            &reaction.message_ts,
            parse_ts_seconds(&reaction.message_ts).unwrap_or_default(),
        );
    }

    for file in files {
        let Some(message) = message_lookup.get(&file.message_ts) else {
            continue;
        };
        let root_ts = message
            .thread_ts
            .clone()
            .unwrap_or_else(|| message.ts.clone());
        if !root_messages.contains_key(&root_ts) {
            continue;
        }
        if channel_filter.is_some_and(|channel_id| channel_id != file.channel_id) {
            continue;
        }

        let entry = threads
            .entry((file.channel_id.clone(), root_ts.clone()))
            .or_insert_with(|| ThreadSummary {
                id: format!("{}:{root_ts}", file.channel_id),
                channel_id: file.channel_id.clone(),
                channel_name: channel_names
                    .get(&file.channel_id)
                    .cloned()
                    .unwrap_or_default(),
                root_ts: root_ts.clone(),
                root_seconds: parse_ts_seconds(&root_ts).unwrap_or_default(),
                title: summarize_text(&message.text),
                preview: summarize_text(&message.text),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: file.message_ts.clone(),
                last_activity_seconds: parse_ts_seconds(&file.message_ts).unwrap_or_default(),
            });

        entry.file_count += 1;
        update_last_activity(
            entry,
            &file.message_ts,
            parse_ts_seconds(&file.message_ts).unwrap_or_default(),
        );
    }

    let mut threads = threads
        .into_values()
        .filter(|thread| {
            date_from.is_none_or(|date_from| thread.last_activity_seconds >= date_from)
                && date_to.is_none_or(|date_to| thread.last_activity_seconds <= date_to)
        })
        .collect::<Vec<_>>();
    threads.sort_by(|left, right| match filters.sort {
        ThreadSort::Activity => (
            right.last_activity_seconds,
            right.reply_count,
            right.reaction_count,
            right.file_count,
            right.root_ts.as_str(),
        )
            .cmp(&(
                left.last_activity_seconds,
                left.reply_count,
                left.reaction_count,
                left.file_count,
                left.root_ts.as_str(),
            )),
        ThreadSort::Newest => (
            right.root_seconds,
            right.last_activity_seconds,
            right.root_ts.as_str(),
        )
            .cmp(&(
                left.root_seconds,
                left.last_activity_seconds,
                left.root_ts.as_str(),
            )),
    });

    threads
}

fn update_last_activity(thread: &mut ThreadSummary, ts: &str, seconds: i64) {
    if seconds >= thread.last_activity_seconds {
        thread.last_activity_seconds = seconds;
        thread.last_activity_ts = ts.to_owned();
    }
}

fn summarize_text(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "(no text)".to_owned();
    }
    normalized.chars().take(80).collect()
}

fn root_text<'a>(root_messages: &'a HashMap<String, &'a Message>, root_ts: &str) -> &'a str {
    root_messages
        .get(root_ts)
        .map(|root| root.text.as_str())
        .unwrap_or_default()
}

fn parse_ts_seconds(value: &str) -> Option<i64> {
    value.split('.').next()?.parse().ok()
}

#[cfg(test)]
#[path = "thread_list_tests.rs"]
mod tests;
