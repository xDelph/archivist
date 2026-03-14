use crate::{
    AppState, analytics::record_analytics, auth::SessionClaims, slack_text,
    view_models::UserSummaryResponse,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use db::{GeneratedThreadSummaryRow, ThreadSummaryRow};
use domain::{Channel, File, Message, Reaction};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ThreadDetailResponse {
    id: String,
    channel_id: String,
    channel_name: Option<String>,
    root_ts: String,
    title: Option<String>,
    preview: Option<String>,
    last_activity_ts: Option<String>,
    reply_count: usize,
    participant_count: usize,
    reaction_count: usize,
    file_count: usize,
    summary: ThreadSummaryBlockResponse,
    messages: Vec<ThreadMessageResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ThreadSummaryBlockResponse {
    text: Option<String>,
    why_it_mattered: Option<String>,
    status: Option<String>,
    topic_tags: Vec<String>,
    model: Option<String>,
    generated_at: Option<i64>,
    is_stale: bool,
    source: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ThreadMessageResponse {
    ts: String,
    thread_ts: Option<String>,
    user_id: Option<String>,
    author: Option<UserSummaryResponse>,
    text: String,
    reactions: Vec<ThreadReactionResponse>,
    files: Vec<ThreadFileResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ThreadReactionResponse {
    user_id: String,
    name: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ThreadFileResponse {
    id: String,
    name: String,
    mimetype: Option<String>,
    permalink: Option<String>,
    size: Option<u64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

struct ThreadDetailData {
    channels: Vec<Channel>,
    messages: Vec<Message>,
    reactions: Vec<Reaction>,
    files: Vec<File>,
    thread_summaries: Vec<ThreadSummaryRow>,
    generated_thread_summaries: Vec<GeneratedThreadSummaryRow>,
}

pub(crate) async fn thread_detail(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
) -> Result<Json<ThreadDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (channel_id, root_ts) = parse_thread_id(&id).ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_thread_id",
        }),
    ))?;

    let response = build_thread_detail(
        &id,
        channel_id,
        root_ts,
        ThreadDetailData {
            channels: state.store.channels().await.map_err(store_failed)?,
            messages: state.store.messages().await.map_err(store_failed)?,
            reactions: state.store.reactions().await.map_err(store_failed)?,
            files: state.store.files().await.map_err(store_failed)?,
            thread_summaries: state.store.thread_summaries().await.map_err(store_failed)?,
            generated_thread_summaries: state
                .store
                .generated_thread_summaries()
                .await
                .map_err(store_failed)?,
        },
        &state.user_store,
    )
    .await
    .ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "thread_not_found",
        }),
    ))?;

    record_analytics(
        &state,
        "thread_view",
        Some(&claims.slack_user_id),
        serde_json::json!({ "thread_id": &id }),
    );

    Ok(Json(response))
}

fn parse_thread_id(thread_id: &str) -> Option<(&str, &str)> {
    let (channel_id, root_ts) = thread_id.split_once(':')?;
    if channel_id.is_empty() || root_ts.is_empty() {
        return None;
    }
    Some((channel_id, root_ts))
}

fn store_failed(_error: db::StoreError) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "store_failed",
        }),
    )
}

async fn build_thread_detail(
    id: &str,
    channel_id: &str,
    root_ts: &str,
    data: ThreadDetailData,
    user_store: &crate::user_store::UserStore,
) -> Option<ThreadDetailResponse> {
    let channel_name = data
        .channels
        .iter()
        .find(|channel| channel.id == channel_id)
        .and_then(|channel| channel.name.as_deref().and_then(normalize_text));
    let thread_summary = data
        .thread_summaries
        .into_iter()
        .find(|summary| summary.channel_id == channel_id && summary.root_ts == root_ts);
    let generated_summary = data
        .generated_thread_summaries
        .into_iter()
        .find(|summary| summary.channel_id == channel_id && summary.root_ts == root_ts);

    let mut thread_messages = data
        .messages
        .into_iter()
        .filter(|message| {
            message.channel_id == channel_id
                && (message.ts == root_ts || message.thread_ts.as_deref() == Some(root_ts))
        })
        .collect::<Vec<_>>();
    thread_messages.sort_by(|left, right| left.ts.cmp(&right.ts));

    if !thread_messages.iter().any(|message| message.ts == root_ts) {
        return None;
    }

    let mut user_ids = thread_messages
        .iter()
        .filter_map(|message| message.user_id.clone())
        .collect::<std::collections::HashSet<_>>();
    user_ids.extend(slack_text::collect_user_mention_ids(
        thread_messages.iter().map(|message| message.text.as_str()),
    ));
    let users = user_store
        .find_users(&user_ids.into_iter().collect::<Vec<_>>())
        .await;
    let channel_names = if slack_text::collect_channel_mention_ids(
        thread_messages.iter().map(|message| message.text.as_str()),
    )
    .is_empty()
    {
        HashMap::new()
    } else {
        slack_text::build_channel_name_map(&data.channels)
    };

    let message_ids = thread_messages
        .iter()
        .map(|message| message.ts.clone())
        .collect::<HashSet<_>>();
    let mut reactions_by_message = HashMap::<String, Vec<ThreadReactionResponse>>::new();
    for reaction in data.reactions.into_iter().filter(|reaction| {
        reaction.channel_id == channel_id && message_ids.contains(&reaction.message_ts)
    }) {
        reactions_by_message
            .entry(reaction.message_ts)
            .or_default()
            .push(ThreadReactionResponse {
                user_id: reaction.user_id,
                name: reaction.name,
            });
    }
    for reactions in reactions_by_message.values_mut() {
        reactions
            .sort_by(|left, right| (&left.name, &left.user_id).cmp(&(&right.name, &right.user_id)));
    }

    let mut files_by_message = HashMap::<String, Vec<ThreadFileResponse>>::new();
    for file in data
        .files
        .into_iter()
        .filter(|file| file.channel_id == channel_id && message_ids.contains(&file.message_ts))
    {
        files_by_message
            .entry(file.message_ts)
            .or_default()
            .push(ThreadFileResponse {
                id: file.id,
                name: file.name,
                mimetype: file.mimetype,
                permalink: file.permalink,
                size: file.size,
            });
    }
    for files in files_by_message.values_mut() {
        files.sort_by(|left, right| left.id.cmp(&right.id));
    }

    let messages = thread_messages
        .into_iter()
        .map(|message| ThreadMessageResponse {
            reactions: reactions_by_message.remove(&message.ts).unwrap_or_default(),
            files: files_by_message.remove(&message.ts).unwrap_or_default(),
            author: message
                .user_id
                .as_ref()
                .and_then(|user_id| users.get(user_id))
                .cloned()
                .map(Into::into),
            ts: message.ts,
            thread_ts: message.thread_ts,
            user_id: message.user_id,
            text: slack_text::render_slack_text(&message.text, &users, &channel_names),
        })
        .collect::<Vec<_>>();
    let reply_count = messages.len().saturating_sub(1);
    let participant_count = messages
        .iter()
        .filter_map(|message| message.user_id.as_deref())
        .collect::<HashSet<_>>()
        .len();
    let reaction_count = messages
        .iter()
        .map(|message| message.reactions.len())
        .sum::<usize>();
    let file_count = messages
        .iter()
        .map(|message| message.files.len())
        .sum::<usize>();
    let title = thread_summary.as_ref().and_then(|summary| {
        normalize_text(&slack_text::render_slack_text(
            &summary.title,
            &users,
            &channel_names,
        ))
    });
    let preview = thread_summary.as_ref().and_then(|summary| {
        normalize_text(&slack_text::render_slack_text(
            &summary.preview,
            &users,
            &channel_names,
        ))
    });
    let last_activity_ts = messages
        .last()
        .map(|message| message.ts.clone())
        .or_else(|| {
            thread_summary
                .as_ref()
                .map(|summary| summary.last_activity_ts.clone())
        });
    let summary = resolve_thread_summary(
        last_activity_ts.as_deref(),
        generated_summary.as_ref(),
        title.clone(),
        preview.clone(),
    );

    Some(ThreadDetailResponse {
        id: id.to_owned(),
        channel_id: channel_id.to_owned(),
        channel_name,
        root_ts: root_ts.to_owned(),
        title,
        preview,
        last_activity_ts,
        reply_count,
        participant_count,
        reaction_count,
        file_count,
        summary,
        messages,
    })
}

fn resolve_thread_summary(
    current_last_activity_ts: Option<&str>,
    generated_summary: Option<&GeneratedThreadSummaryRow>,
    title: Option<String>,
    preview: Option<String>,
) -> ThreadSummaryBlockResponse {
    if let Some(summary) = generated_summary {
        let is_stale = current_last_activity_ts.is_some_and(|last_activity_ts| {
            !same_slack_ts(&summary.source_last_activity_ts, last_activity_ts)
        });
        return ThreadSummaryBlockResponse {
            text: normalize_text(&summary.summary),
            why_it_mattered: summary.why_it_mattered.as_deref().and_then(normalize_text),
            status: normalize_text(&summary.status),
            topic_tags: summary.topic_tags.clone(),
            model: normalize_text(&summary.model),
            generated_at: Some(summary.generated_at),
            is_stale,
            source: "ai",
        };
    }

    let text = title.or(preview);
    ThreadSummaryBlockResponse {
        is_stale: false,
        source: if text.is_some() { "fallback" } else { "none" },
        text,
        why_it_mattered: None,
        status: None,
        topic_tags: vec![],
        model: None,
        generated_at: None,
    }
}

fn normalize_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn same_slack_ts(left: &str, right: &str) -> bool {
    match (
        left.trim().parse::<f64>().ok(),
        right.trim().parse::<f64>().ok(),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(test)]
#[path = "threads_tests.rs"]
mod tests;
