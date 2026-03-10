use crate::{AppState, analytics::record_analytics, auth::SessionClaims};
use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use db::ThreadSummaryRow;
use domain::{Channel, ChannelKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const DAY_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Deserialize)]
pub(crate) struct CatchUpQuery {
    window: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct CatchUpResponse {
    window: &'static str,
    channels: Vec<CatchUpChannelResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CatchUpChannelResponse {
    id: String,
    name: Option<String>,
    kind: &'static str,
    is_archived: bool,
    thread_count: usize,
    threads: Vec<CatchUpThreadResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CatchUpThreadResponse {
    id: String,
    root_ts: String,
    title: String,
    preview: String,
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
    let channels = build_catch_up(
        state
            .store
            .channels()
            .await
            .map_err(store_failed)?
            .into_iter()
            .filter(|channel| channel.team_id == claims.team_id)
            .collect(),
        state
            .store
            .thread_summaries()
            .await
            .map_err(store_failed)?
            .into_iter()
            .filter(|summary| summary.team_id == claims.team_id)
            .collect(),
        window,
        current_unix_timestamp(),
    );

    record_analytics(
        &state,
        "catch_up_view",
        Some(&claims.slack_user_id),
        serde_json::json!({ "window": window.as_str() }),
    );

    Ok(Json(CatchUpResponse {
        window: window.as_str(),
        channels,
    }))
}

fn build_catch_up(
    channels: Vec<Channel>,
    thread_summaries: Vec<ThreadSummaryRow>,
    window: CatchUpWindow,
    now: i64,
) -> Vec<CatchUpChannelResponse> {
    let cutoff = window.cutoff(now);
    let channel_metadata = channels
        .into_iter()
        .map(|channel| (channel.id.clone(), channel))
        .collect::<HashMap<_, _>>();
    let mut grouped = HashMap::<String, Vec<ThreadSummaryRow>>::new();

    for summary in thread_summaries
        .into_iter()
        .filter(|summary| parse_ts_seconds(&summary.last_activity_ts) >= cutoff)
    {
        grouped
            .entry(summary.channel_id.clone())
            .or_default()
            .push(summary);
    }

    let mut channels = grouped
        .into_iter()
        .map(|(channel_id, mut summaries)| {
            summaries.sort_by(|left, right| {
                (
                    parse_ts_seconds(&right.last_activity_ts),
                    right.reply_count,
                    right.reaction_count,
                    right.file_count,
                    right.root_ts.as_str(),
                )
                    .cmp(&(
                        parse_ts_seconds(&left.last_activity_ts),
                        left.reply_count,
                        left.reaction_count,
                        left.file_count,
                        left.root_ts.as_str(),
                    ))
            });
            let metadata = channel_metadata.get(&channel_id);

            (
                summaries
                    .first()
                    .map(|summary| parse_ts_seconds(&summary.last_activity_ts))
                    .unwrap_or_default(),
                CatchUpChannelResponse {
                    id: channel_id.clone(),
                    name: metadata.and_then(|channel| channel.name.clone()),
                    kind: metadata
                        .map(|channel| channel.kind.as_str())
                        .unwrap_or_else(|| ChannelKind::from_channel_id(&channel_id).as_str()),
                    is_archived: metadata.is_some_and(|channel| channel.is_archived),
                    thread_count: summaries.len(),
                    threads: summaries
                        .into_iter()
                        .map(|summary| CatchUpThreadResponse {
                            id: format!("{}:{}", summary.channel_id, summary.root_ts),
                            root_ts: summary.root_ts,
                            title: summary.title,
                            preview: summary.preview,
                            reply_count: summary.reply_count,
                            participant_count: summary.participant_count,
                            reaction_count: summary.reaction_count,
                            file_count: summary.file_count,
                            last_activity_ts: summary.last_activity_ts,
                        })
                        .collect(),
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

    channels.into_iter().map(|(_, channel)| channel).collect()
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
