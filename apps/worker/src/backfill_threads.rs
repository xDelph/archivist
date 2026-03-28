use crate::{
    AppState, ErrorResponse,
    backfill_slack::{SlackHistoryMessage, fetch_thread_replies},
};
use axum::{Json, http::StatusCode};
use db::ThreadSummaryRow;
use std::collections::{HashMap, HashSet};

pub(super) async fn expand_thread_replies(
    state: &AppState,
    slack_user_token: &str,
    channel_id: &str,
    messages: Vec<SlackHistoryMessage>,
    revisit_known_threads: bool,
    replay_cutoff_ts: Option<&str>,
) -> Result<Vec<SlackHistoryMessage>, (StatusCode, Json<ErrorResponse>)> {
    let mut expanded = Vec::with_capacity(messages.len());
    let mut seen = HashSet::new();
    let roots_with_replies = messages
        .iter()
        .filter(|message| should_fetch_thread_replies(message))
        .count();
    let stored_summaries = if roots_with_replies > 0 || revisit_known_threads {
        load_thread_summaries(state, channel_id).await?
    } else {
        HashMap::new()
    };

    if roots_with_replies > 0 {
        tracing::info!(
            channel_id,
            roots_with_replies,
            "fetching thread replies for channel history page"
        );
    }

    for message in messages {
        if seen.insert(message.ts.clone()) {
            expanded.push(message.clone());
        }

        if !should_fetch_thread_replies(&message) {
            continue;
        }

        let thread_replies = fetch_all_thread_replies(
            &state.slack_api_base_url,
            slack_user_token,
            channel_id,
            &message.ts,
            stored_summaries
                .get(&message.ts)
                .and_then(|summary| replay_cutoff_ts.map(|_| summary.last_activity_ts.as_str())),
            stored_summaries.contains_key(&message.ts),
        )
        .await?;
        tracing::info!(
            channel_id,
            root_ts = %message.ts,
            fetched_replies = thread_replies.len(),
            "fetched thread replies for root message"
        );

        for reply in thread_replies {
            if seen.insert(reply.ts.clone()) {
                expanded.push(reply);
            }
        }
    }

    if revisit_known_threads {
        let replay_roots = load_existing_thread_roots(&stored_summaries, &seen);
        if !replay_roots.is_empty() {
            tracing::info!(
                channel_id,
                replay_roots = replay_roots.len(),
                "re-fetching replies for known stored threads during incremental backfill"
            );
        }

        for root_ts in replay_roots {
            let thread_replies = fetch_all_thread_replies(
                &state.slack_api_base_url,
                slack_user_token,
                channel_id,
                &root_ts,
                stored_summaries
                    .get(&root_ts)
                    .map(|summary| summary.last_activity_ts.as_str()),
                true,
            )
            .await?;
            tracing::info!(
                channel_id,
                root_ts = %root_ts,
                fetched_replies = thread_replies.len(),
                "re-fetched stored thread replies during incremental backfill"
            );

            for reply in thread_replies {
                if seen.insert(reply.ts.clone()) {
                    expanded.push(reply);
                }
            }
        }
    }

    Ok(expanded)
}

async fn fetch_all_thread_replies(
    slack_api_base_url: &str,
    slack_user_token: &str,
    channel_id: &str,
    root_ts: &str,
    oldest_ts: Option<&str>,
    inclusive: bool,
) -> Result<Vec<SlackHistoryMessage>, (StatusCode, Json<ErrorResponse>)> {
    let mut replies = Vec::new();
    let mut cursor = None::<String>;

    loop {
        let history = fetch_thread_replies(
            slack_api_base_url,
            slack_user_token,
            channel_id,
            root_ts,
            cursor.as_deref(),
            oldest_ts,
            inclusive,
        )
        .await?;
        let batch = history
            .messages
            .unwrap_or_default()
            .into_iter()
            .filter(|message| message.ts != root_ts)
            .collect::<Vec<_>>();
        let next_cursor = history
            .response_metadata
            .and_then(|metadata| metadata.next_cursor)
            .filter(|value| !value.trim().is_empty());
        tracing::info!(
            channel_id,
            root_ts,
            fetched_replies = batch.len(),
            next_cursor = next_cursor.as_deref().unwrap_or(""),
            "fetched slack thread replies page"
        );
        replies.extend(batch);

        if next_cursor.is_none() {
            break;
        }
        cursor = next_cursor;
    }

    Ok(replies)
}

fn should_fetch_thread_replies(message: &SlackHistoryMessage) -> bool {
    message
        .reply_count
        .is_some_and(|reply_count| reply_count > 0)
        && message
            .thread_ts
            .as_deref()
            .is_none_or(|thread_ts| thread_ts == message.ts)
}

fn load_existing_thread_roots(
    stored_summaries: &HashMap<String, ThreadSummaryRow>,
    seen: &HashSet<String>,
) -> Vec<String> {
    let mut roots = stored_summaries
        .values()
        .filter(|summary| !seen.contains(&summary.root_ts))
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| {
        slack_ts_value(&right.last_activity_ts)
            .partial_cmp(&slack_ts_value(&left.last_activity_ts))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.root_ts.cmp(&right.root_ts))
    });
    let mut roots = roots
        .into_iter()
        .map(|summary| summary.root_ts.clone())
        .collect::<Vec<_>>();
    roots.dedup();
    roots
}

async fn load_thread_summaries(
    state: &AppState,
    channel_id: &str,
) -> Result<HashMap<String, ThreadSummaryRow>, (StatusCode, Json<ErrorResponse>)> {
    Ok(state
        .store
        .thread_summaries()
        .await
        .map_err(thread_summary_lookup_failed)?
        .into_iter()
        .filter(|summary| summary.channel_id == channel_id && summary.reply_count > 0)
        .map(|summary| (summary.root_ts.clone(), summary))
        .collect())
}

fn slack_ts_value(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

fn thread_summary_lookup_failed(error: db::StoreError) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(?error, "failed to load stored thread summaries");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "store_failed",
        }),
    )
}
