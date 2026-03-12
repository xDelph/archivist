use crate::{
    AppState, ErrorResponse,
    backfill_slack::{SlackHistoryMessage, fetch_thread_replies},
};
use axum::{Json, http::StatusCode};
use std::collections::HashSet;

pub(super) async fn expand_thread_replies(
    state: &AppState,
    slack_user_token: &str,
    channel_id: &str,
    messages: Vec<SlackHistoryMessage>,
) -> Result<Vec<SlackHistoryMessage>, (StatusCode, Json<ErrorResponse>)> {
    let mut expanded = Vec::with_capacity(messages.len());
    let mut seen = HashSet::new();
    let roots_with_replies = messages
        .iter()
        .filter(|message| should_fetch_thread_replies(message))
        .count();

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

    Ok(expanded)
}

async fn fetch_all_thread_replies(
    slack_api_base_url: &str,
    slack_user_token: &str,
    channel_id: &str,
    root_ts: &str,
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
