use crate::{AppState, auth::SessionClaims, saved_store::SavedItemRecord, slack_text};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
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
    root_ts: String,
    title: String,
    preview: String,
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
    let channel_names = slack_text::build_channel_name_map(&channels);
    let items = state.saved_store.list_items(&claims.slack_user_id).await;
    let mentioned_user_ids = slack_text::collect_user_mention_ids(
        items
            .iter()
            .flat_map(|item| [item.title.as_str(), item.preview.as_str()]),
    )
    .into_iter()
    .collect::<Vec<_>>();
    let users = state.user_store.find_users(&mentioned_user_ids).await;
    let items = items
        .into_iter()
        .map(|item| saved_item_response(item, &channel_names, &users))
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
    let channels = state.store.channels().await.map_err(store_failed)?;
    let channel_names = slack_text::build_channel_name_map(&channels);
    let saved_item = state
        .saved_store
        .upsert_item(SavedItemRecord {
            slack_user_id: claims.slack_user_id.clone(),
            thread_id: thread_id.to_owned(),
            channel_id: channel_id.to_owned(),
            root_ts: root_ts.to_owned(),
            title: thread_summary.title,
            preview: thread_summary.preview,
            last_activity_ts: thread_summary.last_activity_ts,
            saved_at: current_saved_at(),
        })
        .await
        .map_err(saved_store_error)?;
    let mentioned_user_ids = slack_text::collect_user_mention_ids([
        saved_item.title.as_str(),
        saved_item.preview.as_str(),
    ])
    .into_iter()
    .collect::<Vec<_>>();
    let users = state.user_store.find_users(&mentioned_user_ids).await;

    Ok(Json(SavedMutationResponse {
        ok: true,
        item: saved_item_response(saved_item, &channel_names, &users),
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
    channel_names: &HashMap<String, Option<String>>,
    users: &HashMap<String, crate::user_store::SyncedUserRecord>,
) -> SavedItemResponse {
    SavedItemResponse {
        id: item.thread_id.clone(),
        thread_id: item.thread_id,
        channel_id: item.channel_id.clone(),
        channel_name: channel_names
            .get(&item.channel_id)
            .cloned()
            .unwrap_or_default(),
        root_ts: item.root_ts,
        title: slack_text::render_slack_text(&item.title, users, channel_names),
        preview: slack_text::render_slack_text(&item.preview, users, channel_names),
        last_activity_ts: item.last_activity_ts,
        saved_at: item.saved_at,
    }
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
