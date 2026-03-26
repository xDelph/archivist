use crate::{
    AppState,
    auth::SessionClaims,
    saved_store::SavedItemRecord,
    slack_text,
    thread_preview::{build_generated_summary_lookup, resolve_thread_preview},
    thread_text::{build_root_message_text_map, lookup_root_message_text},
    view_models::UserSummaryResponse,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use db::{GeneratedThreadSummaryRow, ThreadSummaryRow};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Deserialize)]
pub(crate) struct SaveItemRequest {
    thread_id: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct SavedItemsResponse {
    items: Vec<SavedItemResponse>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct SavedMutationResponse {
    ok: bool,
    item: SavedItemResponse,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct DeleteSavedItemResponse {
    ok: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct SavedItemResponse {
    id: String,
    thread_id: String,
    channel_id: String,
    channel_name: Option<String>,
    author: Option<UserSummaryResponse>,
    root_ts: String,
    title: String,
    preview: String,
    summary_preview: Option<String>,
    preview_source: String,
    reply_count: i64,
    participant_count: i64,
    reaction_count: i64,
    file_count: i64,
    last_activity_ts: String,
    saved_at: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

pub(crate) async fn list_saved_items(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
) -> Result<Json<SavedItemsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let channels = state.store.channels().await.map_err(store_failed)?;
    let messages = state.store.messages().await.map_err(store_failed)?;
    let thread_summaries = state.store.thread_summaries().await.map_err(store_failed)?;
    let generated_thread_summaries = state
        .store
        .generated_thread_summaries()
        .await
        .map_err(store_failed)?;
    let root_texts = build_root_message_text_map(&messages);
    let channel_names = slack_text::build_channel_name_map(&channels);
    let summary_lookup = build_thread_summary_lookup(thread_summaries);
    let generated_summary_lookup = build_generated_summary_lookup(generated_thread_summaries);
    let root_users = build_root_user_lookup(&messages);
    let items = state.saved_store.list_items(&claims.slack_user_id).await;
    let item_texts = items
        .iter()
        .map(|item| lookup_root_message_text(&root_texts, &item.channel_id, &item.root_ts))
        .collect::<Vec<_>>();
    let mentioned_user_ids = slack_text::collect_user_mention_ids(
        item_texts.iter().map(String::as_str).chain(
            generated_summary_lookup
                .values()
                .map(|summary| summary.summary.as_str()),
        ),
    )
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
            saved_item_response(
                item,
                &root_texts,
                &summary_lookup,
                &generated_summary_lookup,
                &root_users,
                &channel_names,
                &users,
            )
        })
        .collect();

    Ok(Json(SavedItemsResponse { items }))
}

pub(crate) async fn save_item(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Json(payload): Json<SaveItemRequest>,
) -> Result<Json<SavedMutationResponse>, (StatusCode, Json<ErrorResponse>)> {
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
    let messages = state.store.messages().await.map_err(store_failed)?;
    let root_texts = build_root_message_text_map(&messages);
    let root_users = build_root_user_lookup(&messages);
    let channels = state.store.channels().await.map_err(store_failed)?;
    let channel_names = slack_text::build_channel_name_map(&channels);
    let generated_summary_lookup = build_generated_summary_lookup(
        state
            .store
            .generated_thread_summaries()
            .await
            .map_err(store_failed)?
            .into_iter()
            .filter(|summary| summary.channel_id == channel_id && summary.root_ts == root_ts)
            .collect(),
    );
    let title = lookup_root_message_text(&root_texts, channel_id, root_ts);
    let saved_item = state
        .saved_store
        .upsert_item(SavedItemRecord {
            slack_user_id: claims.slack_user_id.clone(),
            thread_id: thread_id.to_owned(),
            channel_id: channel_id.to_owned(),
            root_ts: root_ts.to_owned(),
            last_activity_ts: thread_summary.last_activity_ts.clone(),
            saved_at: current_saved_at(),
        })
        .await
        .map_err(saved_store_error)?;
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

    Ok(Json(SavedMutationResponse {
        ok: true,
        item: saved_item_response(
            saved_item,
            &root_texts,
            &build_thread_summary_lookup(vec![thread_summary]),
            &generated_summary_lookup,
            &root_users,
            &channel_names,
            &users,
        ),
    }))
}

pub(crate) async fn delete_saved_item(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Path(id): Path<String>,
) -> Result<Json<DeleteSavedItemResponse>, (StatusCode, Json<ErrorResponse>)> {
    if parse_thread_id(&id).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_thread_id",
            }),
        ));
    }

    let removed = state
        .saved_store
        .remove_item(&claims.slack_user_id, &id)
        .await
        .map_err(saved_store_error)?;
    if !removed {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "saved_item_not_found",
            }),
        ));
    }

    Ok(Json(DeleteSavedItemResponse { ok: true }))
}

fn saved_item_response(
    item: SavedItemRecord,
    root_texts: &HashMap<(String, String), String>,
    summary_lookup: &HashMap<(String, String), ThreadSummaryRow>,
    generated_summary_lookup: &HashMap<(String, String), GeneratedThreadSummaryRow>,
    root_users: &HashMap<(String, String), Option<String>>,
    channel_names: &HashMap<String, Option<String>>,
    users: &HashMap<String, crate::user_store::SyncedUserRecord>,
) -> SavedItemResponse {
    let root_text = lookup_root_message_text(root_texts, &item.channel_id, &item.root_ts);
    let summary = summary_lookup.get(&(item.channel_id.clone(), item.root_ts.clone()));
    let preview = resolve_thread_preview(generated_summary_lookup, &item.channel_id, &item.root_ts);
    SavedItemResponse {
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
        summary_preview: preview
            .text
            .as_deref()
            .map(|text| slack_text::render_slack_text(text, users, channel_names)),
        preview_source: preview.source.to_owned(),
        reply_count: summary.map_or(0, |summary| summary.reply_count),
        participant_count: summary.map_or(0, |summary| summary.participant_count),
        reaction_count: summary.map_or(0, |summary| summary.reaction_count),
        file_count: summary.map_or(0, |summary| summary.file_count),
        last_activity_ts: item.last_activity_ts,
        saved_at: item.saved_at,
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

fn parse_thread_id(thread_id: &str) -> Option<(&str, &str)> {
    let (channel_id, root_ts) = thread_id.split_once(':')?;
    if channel_id.is_empty() || root_ts.is_empty() {
        return None;
    }
    Some((channel_id, root_ts))
}

fn current_saved_at() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch");
    format!("{}", duration.as_secs())
}

fn saved_store_error(
    _error: crate::saved_store::SavedItemStoreError,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "saved_store_failed",
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
#[path = "saved_tests.rs"]
mod tests;
