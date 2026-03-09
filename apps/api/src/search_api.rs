use crate::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use domain::{Channel, Message};
use search::{SearchFilters, SearchQuery, SearchSort, normalize_query_text};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    root_ts: String,
    message_ts: String,
    title: String,
    snippet: String,
    score: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchResult {
    id: String,
    thread_id: String,
    channel_id: String,
    channel_name: Option<String>,
    root_ts: String,
    root_seconds: i64,
    message_ts: String,
    message_seconds: i64,
    title: String,
    snippet: String,
    score: usize,
}

pub(crate) async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchApiQuery>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cursor = parse_cursor(query.cursor.as_deref())?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let search_query = parse_search_query(&query)?;
    let items = build_search_results(
        state.store.channels().await,
        state.store.messages().await,
        &search_query,
    );
    let page = items
        .iter()
        .skip(cursor)
        .take(limit)
        .map(|item| SearchResultResponse {
            id: item.id.clone(),
            thread_id: item.thread_id.clone(),
            channel_id: item.channel_id.clone(),
            channel_name: item.channel_name.clone(),
            root_ts: item.root_ts.clone(),
            message_ts: item.message_ts.clone(),
            title: item.title.clone(),
            snippet: item.snippet.clone(),
            score: item.score,
        })
        .collect::<Vec<_>>();
    let next_cursor =
        (cursor + page.len() < items.len()).then(|| (cursor + page.len()).to_string());

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
        "newest" => SearchSort::Newest,
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

fn build_search_results(
    channels: Vec<Channel>,
    messages: Vec<Message>,
    query: &SearchQuery,
) -> Vec<SearchResult> {
    let channel_names = channels
        .into_iter()
        .map(|channel| (channel.id, channel.name))
        .collect::<HashMap<_, _>>();
    let roots = messages
        .iter()
        .filter(|message| message.thread_ts.is_none())
        .map(|message| (message.ts.clone(), message))
        .collect::<HashMap<_, _>>();
    let tokens = query
        .text
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    let date_from = query
        .filters
        .date_from
        .as_deref()
        .and_then(parse_ts_seconds);
    let date_to = query.filters.date_to.as_deref().and_then(parse_ts_seconds);

    let mut results = messages
        .iter()
        .filter_map(|message| {
            if !query.filters.channel_ids.is_empty()
                && !query.filters.channel_ids.contains(&message.channel_id)
            {
                return None;
            }

            let message_seconds = parse_ts_seconds(&message.ts)?;
            if date_from.is_some_and(|date_from| message_seconds < date_from)
                || date_to.is_some_and(|date_to| message_seconds > date_to)
            {
                return None;
            }

            let score = score_message(&message.text, &tokens);
            if score == 0 {
                return None;
            }

            let root_ts = message
                .thread_ts
                .clone()
                .unwrap_or_else(|| message.ts.clone());
            let root = roots.get(&root_ts).copied().unwrap_or(message);

            Some(SearchResult {
                id: format!("{}:{}", message.channel_id, message.ts),
                thread_id: format!("{}:{root_ts}", message.channel_id),
                channel_id: message.channel_id.clone(),
                channel_name: channel_names
                    .get(&message.channel_id)
                    .cloned()
                    .unwrap_or_default(),
                root_ts: root_ts.clone(),
                root_seconds: parse_ts_seconds(&root_ts).unwrap_or(message_seconds),
                message_ts: message.ts.clone(),
                message_seconds,
                title: summarize_text(&root.text),
                snippet: build_snippet(&message.text, &tokens),
                score,
            })
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| match query.sort {
        SearchSort::Relevance => (
            right.score,
            right.message_seconds,
            right.root_seconds,
            right.message_ts.as_str(),
        )
            .cmp(&(
                left.score,
                left.message_seconds,
                left.root_seconds,
                left.message_ts.as_str(),
            )),
        SearchSort::Newest => (
            right.message_seconds,
            right.score,
            right.root_seconds,
            right.message_ts.as_str(),
        )
            .cmp(&(
                left.message_seconds,
                left.score,
                left.root_seconds,
                left.message_ts.as_str(),
            )),
    });
    results
}

fn score_message(message: &str, tokens: &[String]) -> usize {
    let haystack = message.to_lowercase();
    let mut score = 0;
    for token in tokens {
        if haystack.contains(token) {
            score += haystack.matches(token).count();
        }
    }
    if !tokens.is_empty() && haystack.contains(&tokens.join(" ")) {
        score += 2;
    }
    score
}

fn build_snippet(message: &str, tokens: &[String]) -> String {
    let normalized = normalize_query_text(message);
    if normalized.is_empty() {
        return "(no text)".to_owned();
    }

    let lowercase = normalized.to_lowercase();
    let start = tokens
        .iter()
        .filter_map(|token| lowercase.find(token))
        .min()
        .unwrap_or(0);
    let snippet = normalized.chars().skip(start).take(120).collect::<String>();

    if start > 0 {
        format!("...{snippet}")
    } else {
        snippet
    }
}

fn summarize_text(value: &str) -> String {
    let normalized = normalize_query_text(value);
    if normalized.is_empty() {
        return "(no text)".to_owned();
    }
    normalized.chars().take(80).collect()
}

fn parse_ts_seconds(value: &str) -> Option<i64> {
    value.split('.').next()?.parse().ok()
}

#[cfg(test)]
mod tests;
