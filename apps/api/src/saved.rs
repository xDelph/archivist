use crate::{
    AppState,
    auth::SessionClaims,
    saved_store::SavedItemRecord,
    slack_text,
    thread_card_lookup::{ThreadCardKey, build_thread_card_lookup, lookup_thread_card},
    thread_preview::{build_generated_summary_lookup, resolve_thread_preview},
    view_models::UserSummaryResponse,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use db::{GeneratedThreadSummaryRow, ThreadCardRow};
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
    let thread_cards = state.store.thread_cards().await.map_err(store_failed)?;
    let generated_thread_summaries = state
        .store
        .generated_thread_summaries()
        .await
        .map_err(store_failed)?;
    let card_lookup = build_thread_card_lookup(thread_cards);
    let channel_names = slack_text::build_channel_name_map(&channels);
    let generated_summary_lookup = build_generated_summary_lookup(generated_thread_summaries);
    let items = state.saved_store.list_items(&claims.slack_user_id).await;
    let item_texts = items
        .iter()
        .map(|item| {
            lookup_thread_card(&card_lookup, &item.channel_id, &item.root_ts)
                .map(|card| card.title.clone())
                .unwrap_or_else(|| "(no text)".to_owned())
        })
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
        lookup_thread_card(&card_lookup, &item.channel_id, &item.root_ts)
            .and_then(|card| card.author_user_id.clone())
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
                &card_lookup,
                &generated_summary_lookup,
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
    let thread_card = state
        .store
        .thread_cards()
        .await
        .map_err(store_failed)?
        .into_iter()
        .find(|card| card.channel_id == channel_id && card.root_ts == root_ts)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "thread_not_found",
            }),
        ))?;
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
    let saved_item = state
        .saved_store
        .upsert_item(SavedItemRecord {
            slack_user_id: claims.slack_user_id.clone(),
            thread_id: thread_id.to_owned(),
            channel_id: channel_id.to_owned(),
            root_ts: root_ts.to_owned(),
            last_activity_ts: thread_card.last_activity_ts.clone(),
            saved_at: current_saved_at(),
        })
        .await
        .map_err(saved_store_error)?;
    let mentioned_user_ids = slack_text::collect_user_mention_ids([thread_card.title.as_str()])
        .into_iter()
        .collect::<Vec<_>>();
    let mut user_ids = mentioned_user_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    if let Some(user_id) = thread_card.author_user_id.clone() {
        user_ids.insert(user_id);
    }
    let users = state
        .user_store
        .find_users(&user_ids.into_iter().collect::<Vec<_>>())
        .await;
    let card_lookup = build_thread_card_lookup(vec![thread_card]);

    Ok(Json(SavedMutationResponse {
        ok: true,
        item: saved_item_response(
            saved_item,
            &card_lookup,
            &generated_summary_lookup,
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
    card_lookup: &HashMap<ThreadCardKey, ThreadCardRow>,
    generated_summary_lookup: &HashMap<(String, String), GeneratedThreadSummaryRow>,
    channel_names: &HashMap<String, Option<String>>,
    users: &HashMap<String, crate::user_store::SyncedUserRecord>,
) -> SavedItemResponse {
    let card = lookup_thread_card(card_lookup, &item.channel_id, &item.root_ts);
    let preview = resolve_thread_preview(generated_summary_lookup, &item.channel_id, &item.root_ts);
    SavedItemResponse {
        id: item.thread_id.clone(),
        thread_id: item.thread_id,
        channel_id: item.channel_id.clone(),
        channel_name: channel_names
            .get(&item.channel_id)
            .cloned()
            .unwrap_or_default(),
        author: card
            .and_then(|card| card.author_user_id.as_ref())
            .and_then(|user_id| users.get(user_id))
            .cloned()
            .map(Into::into),
        root_ts: item.root_ts,
        title: slack_text::render_slack_text(
            card.map_or("(no text)", |card| card.title.as_str()),
            users,
            channel_names,
        ),
        preview: slack_text::render_slack_text(
            card.map_or("(no text)", |card| card.preview.as_str()),
            users,
            channel_names,
        ),
        summary_preview: preview
            .text
            .as_deref()
            .map(|text| slack_text::render_slack_text(text, users, channel_names)),
        preview_source: preview.source.to_owned(),
        reply_count: card.map_or(0, |card| card.reply_count),
        participant_count: card.map_or(0, |card| card.participant_count),
        reaction_count: card.map_or(0, |card| card.reaction_count),
        file_count: card.map_or(0, |card| card.file_count),
        last_activity_ts: card
            .map(|card| card.last_activity_ts.clone())
            .unwrap_or(item.last_activity_ts),
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
