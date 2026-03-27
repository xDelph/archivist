use crate::{
    thread_card_lookup::build_thread_card_lookup,
    thread_preview::{build_generated_summary_lookup, resolve_thread_preview},
};
use db::{GeneratedThreadSummaryRow, SearchDocumentRow, ThreadCardRow};
use domain::Channel;
use search::{SearchQuery, SearchSort, normalize_query_text};
use std::{cmp::Ordering, collections::HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchResult {
    pub(crate) id: String,
    pub(crate) thread_id: String,
    pub(crate) channel_id: String,
    pub(crate) channel_name: Option<String>,
    pub(crate) author_id: Option<String>,
    pub(crate) root_ts: String,
    pub(crate) root_seconds: i64,
    pub(crate) message_ts: String,
    pub(crate) message_seconds: i64,
    pub(crate) title: String,
    pub(crate) preview: String,
    pub(crate) snippet: String,
    pub(crate) summary_preview: Option<String>,
    pub(crate) preview_source: &'static str,
    pub(crate) last_activity_ts: String,
    pub(crate) last_activity_seconds: i64,
    pub(crate) reply_count: usize,
    pub(crate) participant_count: usize,
    pub(crate) reaction_count: usize,
    pub(crate) file_count: usize,
    pub(crate) score: usize,
}

pub(crate) fn build_search_results(
    channels: Vec<Channel>,
    thread_cards: Vec<ThreadCardRow>,
    search_documents: Vec<SearchDocumentRow>,
    generated_thread_summaries: Vec<GeneratedThreadSummaryRow>,
    query: &SearchQuery,
) -> Vec<SearchResult> {
    let channel_names = channels
        .into_iter()
        .map(|channel| (channel.id, channel.name))
        .collect::<HashMap<_, _>>();
    let card_lookup = build_thread_card_lookup(thread_cards);
    let generated_summary_lookup = build_generated_summary_lookup(generated_thread_summaries);
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

    let mut grouped = HashMap::<(String, String), SearchResult>::new();

    for candidate in search_documents.iter().filter_map(|document| {
        if !query.filters.channel_ids.is_empty()
            && !query.filters.channel_ids.contains(&document.channel_id)
        {
            return None;
        }

        let message_seconds = parse_ts_seconds(&document.message_ts)?;
        if date_from.is_some_and(|date_from| message_seconds < date_from)
            || date_to.is_some_and(|date_to| message_seconds > date_to)
        {
            return None;
        }

        let card = card_lookup.get(&(document.channel_id.clone(), document.root_ts.clone()));
        let title = card
            .map(|card| card.title.clone())
            .or_else(|| document.title.clone())
            .unwrap_or_else(|| normalize_query_text(&document.body));
        let preview = card
            .map(|card| card.preview.clone())
            .unwrap_or_else(|| title.clone());
        let score = score_document(&title, &document.body, &tokens);
        if score == 0 {
            return None;
        }

        let preview_summary = resolve_thread_preview(
            &generated_summary_lookup,
            &document.channel_id,
            &document.root_ts,
        );
        let last_activity_ts = card
            .map(|card| card.last_activity_ts.clone())
            .unwrap_or_else(|| document.message_ts.clone());

        Some(SearchResult {
            id: format!("{}:{}", document.channel_id, document.root_ts),
            thread_id: format!("{}:{}", document.channel_id, document.root_ts),
            channel_id: document.channel_id.clone(),
            channel_name: channel_names
                .get(&document.channel_id)
                .cloned()
                .unwrap_or_default(),
            author_id: card.and_then(|card| card.author_user_id.clone()),
            root_ts: document.root_ts.clone(),
            root_seconds: parse_ts_seconds(&document.root_ts).unwrap_or(message_seconds),
            message_ts: document.message_ts.clone(),
            message_seconds,
            title,
            preview,
            snippet: build_snippet(&document.body, &tokens),
            summary_preview: preview_summary.text,
            preview_source: preview_summary.source,
            last_activity_seconds: parse_ts_seconds(&last_activity_ts).unwrap_or(message_seconds),
            last_activity_ts,
            reply_count: card.map_or(0, |card| as_count(card.reply_count)),
            participant_count: card.map_or(0, |card| as_count(card.participant_count)),
            reaction_count: card.map_or(0, |card| as_count(card.reaction_count)),
            file_count: card.map_or(0, |card| as_count(card.file_count)),
            score,
        })
    }) {
        let key = (candidate.channel_id.clone(), candidate.root_ts.clone());
        match grouped.get_mut(&key) {
            Some(existing) => merge_search_results(existing, candidate, query.sort),
            None => {
                grouped.insert(key, candidate);
            }
        }
    }

    let mut results = grouped.into_values().collect::<Vec<_>>();
    results.sort_by(|left, right| match query.sort {
        SearchSort::Relevance => compare_relevance(left, right),
        SearchSort::Date => compare_date(left, right),
        SearchSort::Replies => compare_replies(left, right),
        SearchSort::Reactions => compare_reactions(left, right),
        SearchSort::People => compare_people(left, right),
    });
    results
}

fn merge_search_results(existing: &mut SearchResult, candidate: SearchResult, sort: SearchSort) {
    let should_replace = match sort {
        SearchSort::Relevance => compare_relevance(&candidate, existing) == Ordering::Less,
        SearchSort::Date | SearchSort::Replies | SearchSort::Reactions | SearchSort::People => {
            compare_latest_match(&candidate, existing) == Ordering::Less
        }
    };
    existing.score += candidate.score;

    if should_replace {
        existing.id = candidate.id;
        existing.message_ts = candidate.message_ts;
        existing.message_seconds = candidate.message_seconds;
        existing.snippet = candidate.snippet;
        existing.summary_preview = candidate.summary_preview;
        existing.preview_source = candidate.preview_source;
        existing.author_id = candidate.author_id;
    }
}

fn compare_relevance(left: &SearchResult, right: &SearchResult) -> Ordering {
    (
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
        ))
}

fn compare_latest_match(left: &SearchResult, right: &SearchResult) -> Ordering {
    (
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
        ))
}

fn compare_date(left: &SearchResult, right: &SearchResult) -> Ordering {
    (
        right.last_activity_seconds,
        right.root_seconds,
        right.reply_count,
        right.reaction_count,
        right.participant_count,
        left.thread_id.as_str(),
    )
        .cmp(&(
            left.last_activity_seconds,
            left.root_seconds,
            left.reply_count,
            left.reaction_count,
            left.participant_count,
            right.thread_id.as_str(),
        ))
}

fn compare_replies(left: &SearchResult, right: &SearchResult) -> Ordering {
    (
        right.reply_count,
        right.reaction_count,
        right.participant_count,
        right.last_activity_seconds,
        right.root_seconds,
        left.thread_id.as_str(),
    )
        .cmp(&(
            left.reply_count,
            left.reaction_count,
            left.participant_count,
            left.last_activity_seconds,
            left.root_seconds,
            right.thread_id.as_str(),
        ))
}

fn compare_reactions(left: &SearchResult, right: &SearchResult) -> Ordering {
    (
        right.reaction_count,
        right.reply_count,
        right.participant_count,
        right.last_activity_seconds,
        right.root_seconds,
        left.thread_id.as_str(),
    )
        .cmp(&(
            left.reaction_count,
            left.reply_count,
            left.participant_count,
            left.last_activity_seconds,
            left.root_seconds,
            right.thread_id.as_str(),
        ))
}

fn compare_people(left: &SearchResult, right: &SearchResult) -> Ordering {
    (
        right.participant_count,
        right.reply_count,
        right.reaction_count,
        right.last_activity_seconds,
        right.root_seconds,
        left.thread_id.as_str(),
    )
        .cmp(&(
            left.participant_count,
            left.reply_count,
            left.reaction_count,
            left.last_activity_seconds,
            left.root_seconds,
            right.thread_id.as_str(),
        ))
}

fn score_document(title: &str, body: &str, tokens: &[String]) -> usize {
    score_text(body, tokens) + (score_text(title, tokens) * 2)
}

fn score_text(value: &str, tokens: &[String]) -> usize {
    let haystack = value.to_lowercase();
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

fn parse_ts_seconds(value: &str) -> Option<i64> {
    value.split('.').next()?.parse().ok()
}

fn as_count(value: i64) -> usize {
    value.max(0) as usize
}
