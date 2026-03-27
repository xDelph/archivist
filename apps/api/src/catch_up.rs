use crate::{
    AppState,
    analytics::record_analytics,
    auth::SessionClaims,
    slack_text,
    thread_preview::{build_generated_summary_lookup, resolve_thread_preview},
    view_models::UserSummaryResponse,
};
use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use db::{GeneratedThreadSummaryRow, ThreadCardRow};
use domain::{Channel, ChannelKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const DAY_SECONDS: i64 = 24 * 60 * 60;
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
pub(crate) struct CatchUpQuery {
    window: Option<String>,
    channel_id: Option<String>,
    cursor: Option<String>,
    limit: Option<usize>,
    sort: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct CatchUpResponse {
    window: &'static str,
    channels: Vec<CatchUpChannelResponse>,
    items: Vec<CatchUpThreadResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct CatchUpChannelResponse {
    id: String,
    name: Option<String>,
    kind: &'static str,
    is_archived: bool,
    thread_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct CatchUpThreadResponse {
    id: String,
    channel_id: String,
    channel_name: Option<String>,
    root_ts: String,
    author: Option<UserSummaryResponse>,
    title: String,
    preview: String,
    summary_preview: Option<String>,
    preview_source: String,
    reply_count: i64,
    participant_count: i64,
    reaction_count: i64,
    file_count: i64,
    last_activity_ts: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CatchUpData {
    channels: Vec<CatchUpChannelResponse>,
    items: Vec<CatchUpThreadResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CatchUpThreadItem {
    response: CatchUpThreadResponse,
    last_activity_seconds: i64,
    root_seconds: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CatchUpWindow {
    Day,
    Week,
}

impl CatchUpWindow {
    fn parse(value: Option<&str>) -> Option<Self> {
        match value.unwrap_or("24h") {
            "24h" => Some(Self::Day),
            "7d" => Some(Self::Week),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Day => "24h",
            Self::Week => "7d",
        }
    }

    const fn cutoff(self, now: i64) -> i64 {
        match self {
            Self::Day => now - DAY_SECONDS,
            Self::Week => now - (7 * DAY_SECONDS),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CatchUpSort {
    Date,
    Replies,
    Reactions,
    People,
}

impl CatchUpSort {
    fn parse(value: Option<&str>) -> Option<Self> {
        match value.unwrap_or("date") {
            "date" => Some(Self::Date),
            "replies" => Some(Self::Replies),
            "reactions" => Some(Self::Reactions),
            "people" => Some(Self::People),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Date => "date",
            Self::Replies => "replies",
            Self::Reactions => "reactions",
            Self::People => "people",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CatchUpFilters<'a> {
    window: CatchUpWindow,
    channel_id: Option<&'a str>,
    sort: CatchUpSort,
}

pub(crate) async fn catch_up(
    State(state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
    Query(query): Query<CatchUpQuery>,
) -> Result<Json<CatchUpResponse>, (StatusCode, Json<ErrorResponse>)> {
    let window = CatchUpWindow::parse(query.window.as_deref()).ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_window",
        }),
    ))?;
    let sort = CatchUpSort::parse(query.sort.as_deref()).ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_sort",
        }),
    ))?;
    let cursor = parse_cursor(query.cursor.as_deref())?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let channels = state.store.channels().await.map_err(store_failed)?;
    let catch_up = build_catch_up(
        channels,
        state.store.thread_cards().await.map_err(store_failed)?,
        state
            .store
            .generated_thread_summaries()
            .await
            .map_err(store_failed)?,
        &state.user_store,
        CatchUpFilters {
            window,
            channel_id: query.channel_id.as_deref(),
            sort,
        },
        current_unix_timestamp(),
    )
    .await;
    let items = catch_up
        .items
        .iter()
        .skip(cursor)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let next_cursor =
        (cursor + items.len() < catch_up.items.len()).then(|| (cursor + items.len()).to_string());

    record_analytics(
        &state,
        "catch_up_view",
        Some(&claims.slack_user_id),
        serde_json::json!({
            "window": window.as_str(),
            "sort": sort.as_str(),
            "channel_id": query.channel_id,
        }),
    );

    Ok(Json(CatchUpResponse {
        window: window.as_str(),
        channels: catch_up.channels,
        items,
        next_cursor,
    }))
}

async fn build_catch_up(
    channels: Vec<Channel>,
    thread_cards: Vec<ThreadCardRow>,
    generated_thread_summaries: Vec<GeneratedThreadSummaryRow>,
    user_store: &crate::user_store::UserStore,
    filters: CatchUpFilters<'_>,
    now: i64,
) -> CatchUpData {
    let cutoff = filters.window.cutoff(now);
    let channel_filter = filters
        .channel_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let channel_names = slack_text::build_channel_name_map(&channels);
    let channel_metadata = channels
        .into_iter()
        .map(|channel| (channel.id.clone(), channel))
        .collect::<HashMap<_, _>>();
    let generated_summary_lookup = build_generated_summary_lookup(generated_thread_summaries);
    let filtered_cards = thread_cards
        .into_iter()
        .filter(|card| parse_ts_seconds(&card.last_activity_ts) >= cutoff)
        .collect::<Vec<_>>();
    let mut user_ids = filtered_cards
        .iter()
        .filter_map(|card| card.author_user_id.clone())
        .collect::<std::collections::HashSet<_>>();
    user_ids.extend(slack_text::collect_user_mention_ids(
        filtered_cards.iter().map(|card| card.title.as_str()).chain(
            generated_summary_lookup
                .values()
                .map(|summary| summary.summary.as_str()),
        ),
    ));
    let users = user_store
        .find_users(&user_ids.into_iter().collect::<Vec<_>>())
        .await;
    let mut channel_counts = HashMap::<String, usize>::new();
    let mut channel_activity = HashMap::<String, i64>::new();
    let mut items = Vec::<CatchUpThreadItem>::new();

    for card in filtered_cards {
        let last_activity_seconds = parse_ts_seconds(&card.last_activity_ts);
        *channel_counts.entry(card.channel_id.clone()).or_default() += 1;
        channel_activity
            .entry(card.channel_id.clone())
            .and_modify(|current| *current = (*current).max(last_activity_seconds))
            .or_insert(last_activity_seconds);

        if channel_filter.is_some_and(|channel_id| channel_id != card.channel_id) {
            continue;
        }

        let preview =
            resolve_thread_preview(&generated_summary_lookup, &card.channel_id, &card.root_ts);
        let channel_name = channel_metadata
            .get(&card.channel_id)
            .and_then(|channel| channel.name.clone());
        let response = CatchUpThreadResponse {
            id: format!("{}:{}", card.channel_id, card.root_ts),
            channel_id: card.channel_id.clone(),
            channel_name,
            author: card
                .author_user_id
                .as_ref()
                .and_then(|user_id| users.get(user_id))
                .cloned()
                .map(Into::into),
            root_ts: card.root_ts.clone(),
            title: slack_text::render_slack_text(&card.title, &users, &channel_names),
            preview: slack_text::render_slack_text(&card.preview, &users, &channel_names),
            summary_preview: preview
                .text
                .as_deref()
                .map(|text| slack_text::render_slack_text(text, &users, &channel_names)),
            preview_source: preview.source.to_owned(),
            reply_count: card.reply_count,
            participant_count: card.participant_count,
            reaction_count: card.reaction_count,
            file_count: card.file_count,
            last_activity_ts: card.last_activity_ts,
        };

        items.push(CatchUpThreadItem {
            last_activity_seconds,
            root_seconds: parse_ts_seconds(&response.root_ts),
            response,
        });
    }

    items.sort_by(|left, right| compare_catch_up_items(left, right, filters.sort));

    let mut channels = channel_counts
        .into_iter()
        .map(|(channel_id, thread_count)| {
            let metadata = channel_metadata.get(&channel_id);

            (
                channel_activity
                    .get(&channel_id)
                    .copied()
                    .unwrap_or_default(),
                CatchUpChannelResponse {
                    id: channel_id.clone(),
                    name: metadata.and_then(|channel| channel.name.clone()),
                    kind: metadata
                        .map(|channel| channel.kind.as_str())
                        .unwrap_or_else(|| ChannelKind::from_channel_id(&channel_id).as_str()),
                    is_archived: metadata.is_some_and(|channel| channel.is_archived),
                    thread_count,
                },
            )
        })
        .collect::<Vec<_>>();
    channels.sort_by(|left, right| {
        (
            right.0,
            left.1.is_archived,
            left.1.name.as_deref(),
            left.1.id.as_str(),
        )
            .cmp(&(
                left.0,
                right.1.is_archived,
                right.1.name.as_deref(),
                right.1.id.as_str(),
            ))
    });

    CatchUpData {
        channels: channels.into_iter().map(|(_, channel)| channel).collect(),
        items: items.into_iter().map(|item| item.response).collect(),
    }
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

fn compare_catch_up_items(
    left: &CatchUpThreadItem,
    right: &CatchUpThreadItem,
    sort: CatchUpSort,
) -> std::cmp::Ordering {
    match sort {
        CatchUpSort::Date => (
            right.last_activity_seconds,
            right.root_seconds,
            right.response.reply_count,
            right.response.reaction_count,
            right.response.participant_count,
            left.response.id.as_str(),
        )
            .cmp(&(
                left.last_activity_seconds,
                left.root_seconds,
                left.response.reply_count,
                left.response.reaction_count,
                left.response.participant_count,
                right.response.id.as_str(),
            )),
        CatchUpSort::Replies => (
            right.response.reply_count,
            right.response.reaction_count,
            right.response.participant_count,
            right.last_activity_seconds,
            right.root_seconds,
            left.response.id.as_str(),
        )
            .cmp(&(
                left.response.reply_count,
                left.response.reaction_count,
                left.response.participant_count,
                left.last_activity_seconds,
                left.root_seconds,
                right.response.id.as_str(),
            )),
        CatchUpSort::Reactions => (
            right.response.reaction_count,
            right.response.reply_count,
            right.response.participant_count,
            right.last_activity_seconds,
            right.root_seconds,
            left.response.id.as_str(),
        )
            .cmp(&(
                left.response.reaction_count,
                left.response.reply_count,
                left.response.participant_count,
                left.last_activity_seconds,
                left.root_seconds,
                right.response.id.as_str(),
            )),
        CatchUpSort::People => (
            right.response.participant_count,
            right.response.reply_count,
            right.response.reaction_count,
            right.last_activity_seconds,
            right.root_seconds,
            left.response.id.as_str(),
        )
            .cmp(&(
                left.response.participant_count,
                left.response.reply_count,
                left.response.reaction_count,
                left.last_activity_seconds,
                left.root_seconds,
                right.response.id.as_str(),
            )),
    }
}

fn parse_ts_seconds(value: &str) -> i64 {
    value
        .split('.')
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or_default()
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
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
#[path = "catch_up_tests.rs"]
mod tests;
