use crate::{
    AppState,
    auth::SessionClaims,
    highlight_store::HighlightedThreadRecord,
    slack_text,
    thread_card_lookup::{ThreadCardKey, build_thread_card_lookup, lookup_thread_card},
    thread_preview::{build_generated_summary_lookup, resolve_thread_preview},
    user_role_store::ADMIN_ROLE,
    view_models::UserSummaryResponse,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use db::{GeneratedThreadSummaryRow, ThreadCardRow};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Deserialize)]
pub(crate) struct ListHighlightsQuery {
    pub(crate) channel_id: Option<String>,
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
    summary_preview: Option<String>,
    preview_source: String,
    reply_count: i64,
    participant_count: i64,
    reaction_count: i64,
    file_count: i64,
    last_activity_ts: String,
    pinned_at: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    pub(crate) error: &'static str,
}

pub(crate) async fn list_highlights(
    State(state): State<AppState>,
    Query(query): Query<ListHighlightsQuery>,
    Extension(_claims): Extension<SessionClaims>,
) -> Result<Json<HighlightsResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(list_highlights_response(&state, &query).await?))
}

pub(crate) async fn list_highlights_response(
    state: &AppState,
    query: &ListHighlightsQuery,
) -> Result<HighlightsResponse, (StatusCode, Json<ErrorResponse>)> {
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
            highlight_item_response(
                item,
                &card_lookup,
                &generated_summary_lookup,
                &channel_names,
                &users,
            )
        })
        .collect();

    Ok(HighlightsResponse { items })
}

pub(crate) async fn pin_highlight(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Json(payload): Json<HighlightThreadRequest>,
) -> Result<Json<HighlightMutationResponse>, (StatusCode, Json<ErrorResponse>)> {
    require_admin_user_id(&state, &claims.slack_user_id).await?;
    pin_highlight_for_user(&state, &claims.slack_user_id, &payload.thread_id).await
}

pub(crate) async fn delete_highlight(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Path(id): Path<String>,
) -> Result<Json<DeleteHighlightResponse>, (StatusCode, Json<ErrorResponse>)> {
    require_admin(&state, &claims).await?;
    delete_highlight_by_id(&state, &id).await
}

pub(crate) async fn delete_highlight_by_id(
    state: &AppState,
    id: &str,
) -> Result<Json<DeleteHighlightResponse>, (StatusCode, Json<ErrorResponse>)> {
    if parse_thread_id(id).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_thread_id",
            }),
        ));
    }

    let removed = state
        .highlight_store
        .unpin_thread(id)
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
    card_lookup: &HashMap<ThreadCardKey, ThreadCardRow>,
    generated_summary_lookup: &HashMap<(String, String), GeneratedThreadSummaryRow>,
    channel_names: &HashMap<String, Option<String>>,
    users: &HashMap<String, crate::user_store::SyncedUserRecord>,
) -> HighlightItemResponse {
    let card = lookup_thread_card(card_lookup, &item.channel_id, &item.root_ts);
    let preview = resolve_thread_preview(generated_summary_lookup, &item.channel_id, &item.root_ts);
    HighlightItemResponse {
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
            .unwrap_or_default(),
        pinned_at: item.pinned_at,
    }
}

async fn require_admin(
    state: &AppState,
    claims: &SessionClaims,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    require_admin_user_id(state, &claims.slack_user_id).await
}

pub(crate) async fn require_admin_user_id(
    state: &AppState,
    slack_user_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let is_admin = state
        .user_role_store
        .has_role(slack_user_id, ADMIN_ROLE)
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

pub(crate) fn parse_thread_id(thread_id: &str) -> Option<(&str, &str)> {
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

pub(crate) async fn pin_highlight_for_user(
    state: &AppState,
    pinned_by_user_id: &str,
    thread_id: &str,
) -> Result<Json<HighlightMutationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let thread_id = thread_id.trim();
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
        .find(|card| card.channel_id == channel_id && card.root_ts == root_ts);
    if thread_card.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "thread_not_found",
            }),
        ));
    }
    let highlighted_thread = state
        .highlight_store
        .pin_thread(HighlightedThreadRecord {
            thread_id: thread_id.to_owned(),
            channel_id: channel_id.to_owned(),
            root_ts: root_ts.to_owned(),
            pinned_by_user_id: pinned_by_user_id.to_owned(),
            pinned_at: current_pinned_at(),
        })
        .await
        .map_err(highlight_store_failed)?;
    let thread_card = thread_card.expect("checked above");
    let channels = state.store.channels().await.map_err(store_failed)?;
    let channel_names = slack_text::build_channel_name_map(&channels);
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

    Ok(Json(HighlightMutationResponse {
        ok: true,
        item: highlight_item_response(
            highlighted_thread,
            &card_lookup,
            &generated_summary_lookup,
            &channel_names,
            &users,
        ),
    }))
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
