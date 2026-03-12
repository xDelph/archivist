use crate::{
    AppState, analytics::record_analytics, auth::SessionClaims, view_models::UserSummaryResponse,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use domain::{File, Message, Reaction};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ThreadDetailResponse {
    id: String,
    channel_id: String,
    root_ts: String,
    reply_count: usize,
    messages: Vec<ThreadMessageResponse>,
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
        state.store.messages().await.map_err(store_failed)?,
        state.store.reactions().await.map_err(store_failed)?,
        state.store.files().await.map_err(store_failed)?,
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
    messages: Vec<Message>,
    reactions: Vec<Reaction>,
    files: Vec<File>,
    user_store: &crate::user_store::UserStore,
) -> Option<ThreadDetailResponse> {
    let mut thread_messages = messages
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

    let users = user_store
        .find_users(
            &thread_messages
                .iter()
                .filter_map(|message| message.user_id.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
        )
        .await;

    let message_ids = thread_messages
        .iter()
        .map(|message| message.ts.clone())
        .collect::<HashSet<_>>();
    let mut reactions_by_message = HashMap::<String, Vec<ThreadReactionResponse>>::new();
    for reaction in reactions.into_iter().filter(|reaction| {
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
    for file in files
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
            text: message.text,
        })
        .collect::<Vec<_>>();
    let reply_count = messages.len().saturating_sub(1);

    Some(ThreadDetailResponse {
        id: id.to_owned(),
        channel_id: channel_id.to_owned(),
        root_ts: root_ts.to_owned(),
        reply_count,
        messages,
    })
}

#[cfg(test)]
#[path = "threads_tests.rs"]
mod tests;
