use crate::{
    AppState, analytics::record_analytics, auth::SessionClaims, slack_text,
    view_models::UserSummaryResponse,
};
use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use search::{SearchFilters, SearchQuery, SearchSort, normalize_query_text};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub(crate) use crate::search_results::build_search_results;

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
pub(crate) struct SearchApiQuery {
    q: Option<String>,
    channel_id: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    sort: Option<String>,
    cursor: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct SearchResponse {
    query: String,
    items: Vec<SearchResultResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct SearchResultResponse {
    id: String,
    thread_id: String,
    channel_id: String,
    channel_name: Option<String>,
    author: Option<UserSummaryResponse>,
    root_ts: String,
    message_ts: String,
    title: String,
    preview: String,
    snippet: String,
    summary_preview: Option<String>,
    preview_source: String,
    last_activity_ts: String,
    reply_count: usize,
    participant_count: usize,
    reaction_count: usize,
    file_count: usize,
    score: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

pub(crate) async fn search(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Query(query): Query<SearchApiQuery>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cursor = parse_cursor(query.cursor.as_deref())?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let search_query = parse_search_query(&query)?;
    let channels = state.store.channels().await.map_err(store_failed)?;
    let items = build_search_results(
        channels.clone(),
        state.store.thread_cards().await.map_err(store_failed)?,
        state.store.search_documents().await.map_err(store_failed)?,
        state
            .store
            .generated_thread_summaries()
            .await
            .map_err(store_failed)?,
        &search_query,
    );
    let channel_names = slack_text::build_channel_name_map(&channels);
    let mut user_ids = items
        .iter()
        .filter_map(|item| item.author_id.clone())
        .collect::<HashSet<_>>();
    user_ids.extend(slack_text::collect_user_mention_ids(
        items
            .iter()
            .flat_map(|item| {
                [
                    Some(item.title.as_str()),
                    Some(item.snippet.as_str()),
                    item.summary_preview.as_deref(),
                ]
            })
            .flatten(),
    ));
    let users = state
        .user_store
        .find_users(&user_ids.into_iter().collect::<Vec<_>>())
        .await;
    let page = items
        .iter()
        .skip(cursor)
        .take(limit)
        .map(|item| SearchResultResponse {
            id: item.id.clone(),
            thread_id: item.thread_id.clone(),
            channel_id: item.channel_id.clone(),
            channel_name: item.channel_name.clone(),
            author: item
                .author_id
                .as_ref()
                .and_then(|user_id| users.get(user_id))
                .cloned()
                .map(Into::into),
            root_ts: item.root_ts.clone(),
            message_ts: item.message_ts.clone(),
            title: slack_text::render_slack_text(&item.title, &users, &channel_names),
            preview: slack_text::render_slack_text(&item.preview, &users, &channel_names),
            snippet: slack_text::render_slack_text(&item.snippet, &users, &channel_names),
            summary_preview: item
                .summary_preview
                .as_deref()
                .map(|text| slack_text::render_slack_text(text, &users, &channel_names)),
            preview_source: item.preview_source.to_owned(),
            last_activity_ts: item.last_activity_ts.clone(),
            reply_count: item.reply_count,
            participant_count: item.participant_count,
            reaction_count: item.reaction_count,
            file_count: item.file_count,
            score: item.score,
        })
        .collect::<Vec<_>>();
    let next_cursor =
        (cursor + page.len() < items.len()).then(|| (cursor + page.len()).to_string());

    record_analytics(
        &state,
        "search_query",
        Some(&claims.slack_user_id),
        serde_json::json!({ "query": &search_query.text, "result_count": items.len() }),
    );

    Ok(Json(SearchResponse {
        query: search_query.text,
        items: page,
        next_cursor,
    }))
}

fn parse_search_query(
    query: &SearchApiQuery,
) -> Result<SearchQuery, (StatusCode, Json<ErrorResponse>)> {
    let normalized = normalize_query_text(query.q.as_deref().unwrap_or_default());
    if normalized.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing_query",
            }),
        ));
    }

    let sort = match query.sort.as_deref().unwrap_or("relevance") {
        "relevance" => SearchSort::Relevance,
        "date" => SearchSort::Date,
        "replies" => SearchSort::Replies,
        "reactions" => SearchSort::Reactions,
        "people" => SearchSort::People,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_sort",
                }),
            ));
        }
    };

    Ok(SearchQuery {
        text: normalized,
        filters: SearchFilters {
            channel_ids: query
                .channel_id
                .iter()
                .map(|channel_id| channel_id.trim().to_owned())
                .filter(|channel_id| !channel_id.is_empty())
                .collect(),
            date_from: query.date_from.clone(),
            date_to: query.date_to.clone(),
        },
        sort,
    })
}

fn parse_cursor(cursor: Option<&str>) -> Result<usize, (StatusCode, Json<ErrorResponse>)> {
    cursor
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_cursor",
                }),
            )
        })
        .map(|value| value.unwrap_or_default())
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
#[path = "search_api_tests.rs"]
mod tests;
