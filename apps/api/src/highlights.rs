use crate::{
    AppState,
    auth::SessionClaims,
    highlight_store::HighlightedThreadRecord,
    slack_text,
    thread_text::{build_root_message_text_map, lookup_root_message_text},
    user_role_store::ADMIN_ROLE,
    view_models::UserSummaryResponse,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use db::ThreadSummaryRow;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Deserialize)]
pub(crate) struct ListHighlightsQuery {
    channel_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HighlightThreadRequest {
    thread_id: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct HighlightsResponse {
    items: Vec<HighlightItemResponse>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct HighlightMutationResponse {
    ok: bool,
    item: HighlightItemResponse,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct DeleteHighlightResponse {
    ok: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct HighlightItemResponse {
    id: String,
    thread_id: String,
    channel_id: String,
    channel_name: Option<String>,
    author: Option<UserSummaryResponse>,
    root_ts: String,
    title: String,
    preview: String,
    reply_count: i64,
    participant_count: i64,
    reaction_count: i64,
    file_count: i64,
    last_activity_ts: String,
    pinned_at: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

pub(crate) async fn list_highlights(
    State(state): State<AppState>,
    Query(query): Query<ListHighlightsQuery>,
    Extension(_claims): Extension<SessionClaims>,
) -> Result<Json<HighlightsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let channels = state.store.channels().await.map_err(store_failed)?;
    let messages = state.store.messages().await.map_err(store_failed)?;
    let thread_summaries = state.store.thread_summaries().await.map_err(store_failed)?;
    let root_texts = build_root_message_text_map(&messages);
    let channel_names = slack_text::build_channel_name_map(&channels);
    let summary_lookup = build_thread_summary_lookup(thread_summaries);
    let root_users = build_root_user_lookup(&messages);
    let items = state
        .highlight_store
        .list_threads()
        .await
        .into_iter()
        .filter(|item| {
            query
                .channel_id
                .as_deref()
                .is_none_or(|channel_id| channel_id == item.channel_id)
        })
        .collect::<Vec<_>>();
    let item_texts = items
        .iter()
        .map(|item| lookup_root_message_text(&root_texts, &item.channel_id, &item.root_ts))
        .collect::<Vec<_>>();
    let mentioned_user_ids =
        slack_text::collect_user_mention_ids(item_texts.iter().map(String::as_str))
            .into_iter()
            .collect::<Vec<_>>();
    let mut user_ids = mentioned_user_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    user_ids.extend(items.iter().filter_map(|item| {
        root_users
            .get(&(item.channel_id.clone(), item.root_ts.clone()))
            .and_then(|user_id| user_id.clone())
    }));
    let users = state
        .user_store
        .find_users(&user_ids.into_iter().collect::<Vec<_>>())
        .await;
    let items = items
        .into_iter()
        .map(|item| {
            highlight_item_response(
                item,
                &root_texts,
                &summary_lookup,
                &root_users,
                &channel_names,
                &users,
            )
        })
        .collect();

    Ok(Json(HighlightsResponse { items }))
}

pub(crate) async fn pin_highlight(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Json(payload): Json<HighlightThreadRequest>,
) -> Result<Json<HighlightMutationResponse>, (StatusCode, Json<ErrorResponse>)> {
    require_admin(&state, &claims).await?;

    let thread_id = payload.thread_id.trim();
    let (channel_id, root_ts) = parse_thread_id(thread_id).ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_thread_id",
        }),
    ))?;
    let thread_summary = state
        .store
        .thread_summaries()
        .await
        .map_err(store_failed)?
        .into_iter()
        .find(|summary| summary.channel_id == channel_id && summary.root_ts == root_ts)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "thread_not_found",
            }),
        ))?;
    let highlighted_thread = state
        .highlight_store
        .pin_thread(HighlightedThreadRecord {
            thread_id: thread_id.to_owned(),
            channel_id: channel_id.to_owned(),
            root_ts: root_ts.to_owned(),
            pinned_by_user_id: claims.slack_user_id.clone(),
            pinned_at: current_pinned_at(),
        })
        .await
        .map_err(highlight_store_failed)?;
    let messages = state.store.messages().await.map_err(store_failed)?;
    let root_texts = build_root_message_text_map(&messages);
    let root_users = build_root_user_lookup(&messages);
    let channels = state.store.channels().await.map_err(store_failed)?;
    let channel_names = slack_text::build_channel_name_map(&channels);
    let title = lookup_root_message_text(&root_texts, channel_id, root_ts);
    let mentioned_user_ids = slack_text::collect_user_mention_ids([title.as_str()])
        .into_iter()
        .collect::<Vec<_>>();
    let mut user_ids = mentioned_user_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    if let Some(user_id) = root_users
        .get(&(channel_id.to_owned(), root_ts.to_owned()))
        .and_then(|user_id| user_id.clone())
    {
        user_ids.insert(user_id);
    }
    let users = state
        .user_store
        .find_users(&user_ids.into_iter().collect::<Vec<_>>())
        .await;

    Ok(Json(HighlightMutationResponse {
        ok: true,
        item: highlight_item_response(
            highlighted_thread,
            &root_texts,
            &build_thread_summary_lookup(vec![thread_summary]),
            &root_users,
            &channel_names,
            &users,
        ),
    }))
}

pub(crate) async fn delete_highlight(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Path(id): Path<String>,
) -> Result<Json<DeleteHighlightResponse>, (StatusCode, Json<ErrorResponse>)> {
    require_admin(&state, &claims).await?;

    if parse_thread_id(&id).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_thread_id",
            }),
        ));
    }

    let removed = state
        .highlight_store
        .unpin_thread(&id)
        .await
        .map_err(highlight_store_failed)?;
    if !removed {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "highlight_not_found",
            }),
        ));
    }

    Ok(Json(DeleteHighlightResponse { ok: true }))
}

fn highlight_item_response(
    item: HighlightedThreadRecord,
    root_texts: &HashMap<(String, String), String>,
    summary_lookup: &HashMap<(String, String), ThreadSummaryRow>,
    root_users: &HashMap<(String, String), Option<String>>,
    channel_names: &HashMap<String, Option<String>>,
    users: &HashMap<String, crate::user_store::SyncedUserRecord>,
) -> HighlightItemResponse {
    let root_text = lookup_root_message_text(root_texts, &item.channel_id, &item.root_ts);
    let summary = summary_lookup.get(&(item.channel_id.clone(), item.root_ts.clone()));
    HighlightItemResponse {
        id: item.thread_id.clone(),
        thread_id: item.thread_id,
        channel_id: item.channel_id.clone(),
        channel_name: channel_names
            .get(&item.channel_id)
            .cloned()
            .unwrap_or_default(),
        author: root_users
            .get(&(item.channel_id.clone(), item.root_ts.clone()))
            .and_then(|user_id| user_id.as_ref())
            .and_then(|user_id| users.get(user_id))
            .cloned()
            .map(Into::into),
        root_ts: item.root_ts,
        title: slack_text::render_slack_text(&root_text, users, channel_names),
        preview: slack_text::render_slack_text(&root_text, users, channel_names),
        reply_count: summary.map_or(0, |summary| summary.reply_count),
        participant_count: summary.map_or(0, |summary| summary.participant_count),
        reaction_count: summary.map_or(0, |summary| summary.reaction_count),
        file_count: summary.map_or(0, |summary| summary.file_count),
        last_activity_ts: summary
            .map(|summary| summary.last_activity_ts.clone())
            .unwrap_or_default(),
        pinned_at: item.pinned_at,
    }
}

fn build_thread_summary_lookup(
    thread_summaries: Vec<ThreadSummaryRow>,
) -> HashMap<(String, String), ThreadSummaryRow> {
    thread_summaries
        .into_iter()
        .map(|summary| {
            (
                (summary.channel_id.clone(), summary.root_ts.clone()),
                summary,
            )
        })
        .collect()
}

fn build_root_user_lookup(
    messages: &[domain::Message],
) -> HashMap<(String, String), Option<String>> {
    messages
        .iter()
        .filter(|message| {
            message
                .thread_ts
                .as_deref()
                .is_none_or(|thread_ts| thread_ts == message.ts)
        })
        .map(|message| {
            (
                (message.channel_id.clone(), message.ts.clone()),
                message.user_id.clone(),
            )
        })
        .collect()
}

async fn require_admin(
    state: &AppState,
    claims: &SessionClaims,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let is_admin = state
        .user_role_store
        .has_role(&claims.slack_user_id, ADMIN_ROLE)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "user_roles_unavailable",
                }),
            )
        })?;
    if !is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "admin_required",
            }),
        ));
    }
    Ok(())
}

fn parse_thread_id(thread_id: &str) -> Option<(&str, &str)> {
    let (channel_id, root_ts) = thread_id.split_once(':')?;
    if channel_id.is_empty() || root_ts.is_empty() {
        return None;
    }
    Some((channel_id, root_ts))
}

fn current_pinned_at() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch");
    format!("{}", duration.as_secs())
}

fn highlight_store_failed(
    _error: crate::highlight_store::HighlightStoreError,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "highlight_store_failed",
        }),
    )
}

fn store_failed(_error: db::StoreError) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "store_failed",
        }),
    )
}

#[cfg(test)]
#[path = "highlights_tests.rs"]
mod tests;
