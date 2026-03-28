use super::{AppState, ErrorResponse, store_failed};
use crate::ai_openrouter::request_summary;
use crate::backfill_range::{parse_backfill_request, resolve_oldest_ts};
use axum::{Json, body::Bytes, extract::State, http::StatusCode};
use db::{GeneratedThreadSummaryRow, ThreadSummaryRow};
use domain::Message;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_TOPIC_TAGS: usize = 5;
const MIN_STANDALONE_SUMMARY_CHARS: usize = 500;
const MIN_TEMPORARY_SUMMARY_REPLY_COUNT: i64 = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenRouterConfig {
    pub(crate) api_base_url: String,
    pub(crate) api_key: Option<String>,
    pub(crate) model: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct GenerateThreadSummariesResponse {
    ok: bool,
    generated: usize,
    skipped: usize,
    failed: usize,
}

#[derive(Debug)]
pub(crate) struct GeneratedSummaryResult {
    pub(crate) summary: String,
    pub(crate) full_summary: String,
    pub(crate) why_it_mattered: Option<String>,
    pub(crate) status: String,
    pub(crate) topic_tags: Vec<String>,
}

pub(crate) async fn generate_thread_summaries(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<GenerateThreadSummariesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request = parse_backfill_request(&body)?;
    let config = state
        .openrouter_config
        .ready()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, missing_openrouter_config()))?;
    tracing::info!(
        channel_id = request.channel_id.as_deref().unwrap_or(""),
        oldest_ts = request.oldest_ts.as_deref().unwrap_or(""),
        resume_from_last_message_ts = request.resume_from_last_message_ts,
        model = %config.model,
        "starting thread summary generation job"
    );
    if request.cursor.is_some() {
        tracing::warn!("ignoring cursor for thread summary generation job");
    }

    let thread_summaries = state.store.thread_summaries().await.map_err(store_failed)?;
    if thread_summaries.is_empty() {
        tracing::info!("skipping thread summary generation because no thread summaries exist");
        return Ok(Json(GenerateThreadSummariesResponse {
            ok: true,
            generated: 0,
            skipped: 0,
            failed: 0,
        }));
    }

    let existing = state
        .store
        .generated_thread_summaries()
        .await
        .map_err(store_failed)?
        .into_iter()
        .map(|row| ((row.channel_id.clone(), row.root_ts.clone()), row))
        .collect::<HashMap<_, _>>();
    let candidates =
        select_candidate_threads(&state, &request, thread_summaries, &existing, &config.model)
            .await?;
    if candidates.is_empty() {
        tracing::info!("skipping thread summary generation because no candidate threads matched");
        return Ok(Json(GenerateThreadSummariesResponse {
            ok: true,
            generated: 0,
            skipped: 0,
            failed: 0,
        }));
    }

    let candidate_keys = candidates
        .iter()
        .map(|summary| (summary.channel_id.clone(), summary.root_ts.clone()))
        .collect::<HashSet<_>>();
    let thread_messages = group_thread_messages(
        state.store.messages().await.map_err(store_failed)?,
        &candidate_keys,
    );
    tracing::info!(
        candidate_threads = candidates.len(),
        existing_generated_threads = existing.len(),
        threads_with_messages = thread_messages.len(),
        "resolved thread summary generation inputs"
    );

    let mut generated = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    for summary in candidates {
        let key = (summary.channel_id.clone(), summary.root_ts.clone());
        if !request.force_regenerate
            && existing.get(&key).is_some_and(|row| {
                row.model == config.model
                    && same_slack_ts(&row.source_last_activity_ts, &summary.last_activity_ts)
            })
        {
            tracing::info!(
                channel_id = %summary.channel_id,
                root_ts = %summary.root_ts,
                model = %config.model,
                source_last_activity_ts = %summary.last_activity_ts,
                "skipping thread summary generation because the stored summary is up to date"
            );
            skipped += 1;
            continue;
        }
        if request.force_regenerate {
            tracing::info!(
                channel_id = %summary.channel_id,
                root_ts = %summary.root_ts,
                model = %config.model,
                "forcing thread summary regeneration even if an up-to-date summary already exists"
            );
        }
        let Some(messages) = thread_messages.get(&key) else {
            tracing::info!(
                channel_id = %summary.channel_id,
                root_ts = %summary.root_ts,
                "skipping thread summary generation because no thread messages were found"
            );
            skipped += 1;
            continue;
        };
        if messages.is_empty() {
            tracing::info!(
                channel_id = %summary.channel_id,
                root_ts = %summary.root_ts,
                "skipping thread summary generation because the thread has no messages"
            );
            skipped += 1;
            continue;
        }
        let root_message_chars = root_message_char_count(&summary, messages);
        if summary.reply_count == 0
            && root_message_chars.is_none_or(|count| count <= MIN_STANDALONE_SUMMARY_CHARS)
        {
            tracing::info!(
                channel_id = %summary.channel_id,
                root_ts = %summary.root_ts,
                root_message_chars = root_message_chars.unwrap_or_default(),
                "skipping thread summary generation because the thread has no replies and the root message is too short"
            );
            skipped += 1;
            continue;
        }
        if summary.reply_count < MIN_TEMPORARY_SUMMARY_REPLY_COUNT {
            tracing::info!(
                channel_id = %summary.channel_id,
                root_ts = %summary.root_ts,
                reply_count = summary.reply_count,
                minimum_reply_count = MIN_TEMPORARY_SUMMARY_REPLY_COUNT,
                "skipping thread summary generation because the temporary high-volume test only targets large threads"
            );
            skipped += 1;
            continue;
        }

        tracing::info!(
            channel_id = %summary.channel_id,
            root_ts = %summary.root_ts,
            message_count = messages.len(),
            participant_count = summary.participant_count,
            reply_count = summary.reply_count,
            "requesting thread summary from OpenRouter"
        );
        match request_summary(&config, &summary, messages).await {
            Ok(result) => {
                let row = GeneratedThreadSummaryRow {
                    channel_id: summary.channel_id.clone(),
                    root_ts: summary.root_ts.clone(),
                    summary: result.summary,
                    full_summary: Some(result.full_summary),
                    why_it_mattered: result.why_it_mattered,
                    status: result.status,
                    topic_tags: result.topic_tags,
                    source_last_activity_ts: summary.last_activity_ts.clone(),
                    model: config.model.clone(),
                    generated_at: current_unix_timestamp(),
                };
                if let Err(error) = state.store.upsert_generated_thread_summary(&row).await {
                    tracing::error!(
                        ?error,
                        channel_id = %summary.channel_id,
                        root_ts = %summary.root_ts,
                        "failed to persist generated thread summary"
                    );
                    failed += 1;
                } else {
                    tracing::info!(
                        channel_id = %summary.channel_id,
                        root_ts = %summary.root_ts,
                        status = %row.status,
                        topic_tags = ?row.topic_tags,
                        "persisted generated thread summary"
                    );
                    generated += 1;
                }
            }
            Err(error) => {
                tracing::warn!(
                    ?error,
                    channel_id = %summary.channel_id,
                    root_ts = %summary.root_ts,
                    "failed to generate thread summary"
                );
                failed += 1;
            }
        }
    }

    tracing::info!(
        generated,
        skipped,
        failed,
        model = %config.model,
        "completed thread summary generation job"
    );
    Ok(Json(GenerateThreadSummariesResponse {
        ok: true,
        generated,
        skipped,
        failed,
    }))
}

async fn select_candidate_threads(
    state: &AppState,
    request: &crate::backfill_range::BackfillChannelRequest,
    thread_summaries: Vec<ThreadSummaryRow>,
    existing: &HashMap<(String, String), GeneratedThreadSummaryRow>,
    model: &str,
) -> Result<Vec<ThreadSummaryRow>, (StatusCode, Json<ErrorResponse>)> {
    if request.resume_from_last_message_ts {
        let mut candidates = thread_summaries
            .into_iter()
            .filter(|summary| {
                summary_matches_channel_filter(summary, request.channel_id.as_deref())
            })
            .filter(|summary| {
                generated_summary_needs_refresh(
                    existing.get(&(summary.channel_id.clone(), summary.root_ts.clone())),
                    summary,
                    model,
                )
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            (&left.channel_id, &left.last_activity_ts, &left.root_ts).cmp(&(
                &right.channel_id,
                &right.last_activity_ts,
                &right.root_ts,
            ))
        });
        tracing::info!(
            selected_candidates = candidates.len(),
            channel_filter = request.channel_id.as_deref().unwrap_or(""),
            "selected stale or missing generated summaries for resume-mode generation"
        );
        return Ok(candidates);
    }

    let mut cutoffs = HashMap::<String, Option<String>>::new();
    if let Some(channel_id) = request
        .channel_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        cutoffs.insert(
            channel_id.to_owned(),
            resolve_oldest_ts(&state.store, request, channel_id).await?,
        );
    } else {
        let mut channel_ids = thread_summaries
            .iter()
            .map(|summary| summary.channel_id.clone())
            .collect::<Vec<_>>();
        channel_ids.sort();
        channel_ids.dedup();
        for channel_id in channel_ids {
            let oldest_ts = resolve_oldest_ts(&state.store, request, &channel_id).await?;
            cutoffs.insert(channel_id, oldest_ts);
        }
    }

    let mut candidates = thread_summaries
        .into_iter()
        .filter(|summary| {
            cutoffs
                .get(&summary.channel_id)
                .is_some_and(|cutoff| is_summary_in_range(summary, cutoff.as_deref()))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        (&left.channel_id, &left.last_activity_ts, &left.root_ts).cmp(&(
            &right.channel_id,
            &right.last_activity_ts,
            &right.root_ts,
        ))
    });
    tracing::info!(
        selected_candidates = candidates.len(),
        channel_count = cutoffs.len(),
        "selected candidate threads for summary generation"
    );
    Ok(candidates)
}

fn summary_matches_channel_filter(summary: &ThreadSummaryRow, channel_id: Option<&str>) -> bool {
    channel_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none_or(|channel_id| summary.channel_id == channel_id)
}

fn is_summary_in_range(summary: &ThreadSummaryRow, oldest_ts: Option<&str>) -> bool {
    oldest_ts.is_none_or(|oldest_ts| {
        slack_ts_value(&summary.last_activity_ts) >= slack_ts_value(oldest_ts)
    })
}

fn generated_summary_needs_refresh(
    row: Option<&GeneratedThreadSummaryRow>,
    summary: &ThreadSummaryRow,
    model: &str,
) -> bool {
    row.is_none_or(|row| {
        row.model != model
            || !same_slack_ts(&row.source_last_activity_ts, &summary.last_activity_ts)
    })
}

fn group_thread_messages(
    messages: Vec<Message>,
    candidate_keys: &HashSet<(String, String)>,
) -> HashMap<(String, String), Vec<Message>> {
    let mut grouped = HashMap::<(String, String), Vec<Message>>::new();
    for message in messages {
        let root_ts = normalized_root_ts(&message);
        let key = (message.channel_id.clone(), root_ts);
        if !candidate_keys.contains(&key) {
            continue;
        }
        grouped.entry(key).or_default().push(message);
    }
    for messages in grouped.values_mut() {
        messages.sort_by(|left, right| left.ts.cmp(&right.ts));
    }
    grouped
}

pub(crate) fn compact_text(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "(no text)".to_owned();
    }
    normalized.chars().take(max_chars).collect()
}

pub(crate) fn normalize_text(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    (!normalized.is_empty()).then_some(normalized)
}

pub(crate) fn normalize_markdown_text(value: &str) -> Option<String> {
    let normalized = value.trim().replace("\r\n", "\n");
    (!normalized.is_empty()).then_some(normalized)
}

pub(crate) fn normalize_status(value: Option<&str>) -> String {
    match value
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "discussion".to_owned())
        .as_str()
    {
        "answered" => "answered".to_owned(),
        "unresolved" => "unresolved".to_owned(),
        "announcement" => "announcement".to_owned(),
        "debate" => "debate".to_owned(),
        "resource" => "resource".to_owned(),
        _ => "discussion".to_owned(),
    }
}

pub(crate) fn normalize_topic_tags(topic_tags: Vec<String>) -> Vec<String> {
    let mut tags = Vec::new();
    let mut seen = HashSet::new();
    for tag in topic_tags {
        let normalized = tag.trim().trim_start_matches('#').to_ascii_lowercase();
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        tags.push(normalized);
        if tags.len() == MAX_TOPIC_TAGS {
            break;
        }
    }
    tags
}

fn normalized_root_ts(message: &Message) -> String {
    message
        .thread_ts
        .clone()
        .unwrap_or_else(|| message.ts.clone())
}

fn root_message_char_count(summary: &ThreadSummaryRow, messages: &[Message]) -> Option<usize> {
    messages
        .iter()
        .find(|message| message.ts == summary.root_ts)
        .map(|message| message.text.chars().count())
}

fn slack_ts_value(value: &str) -> f64 {
    value.trim().parse::<f64>().unwrap_or_default()
}

fn same_slack_ts(left: &str, right: &str) -> bool {
    match (
        left.trim().parse::<f64>().ok(),
        right.trim().parse::<f64>().ok(),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

fn missing_openrouter_config() -> Json<ErrorResponse> {
    Json(ErrorResponse {
        error: "missing_openrouter_config",
    })
}

#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;
