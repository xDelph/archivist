use crate::ThreadSummaryRow;
use domain::{File, Message};
use std::collections::{HashMap, HashSet};

type MessageKey = (String, String, String);
type ReactionKey = (String, String, String, String, String);
type FileKey = (String, String);
type ThreadSummaryKey = (String, String, String);

pub(crate) type ThreadSummaryMap = HashMap<ThreadSummaryKey, ThreadSummaryRow>;

#[derive(Debug)]
struct ThreadSummaryAggregate {
    team_id: String,
    channel_id: String,
    root_ts: String,
    title: String,
    preview: String,
    reply_count: usize,
    participants: HashSet<String>,
    reaction_count: usize,
    file_count: usize,
    last_activity_ts: String,
    last_activity_seconds: i64,
}

pub(crate) fn build_thread_summaries(
    messages: &HashMap<MessageKey, Message>,
    reactions: &HashSet<ReactionKey>,
    files: &HashMap<FileKey, File>,
) -> ThreadSummaryMap {
    let root_messages = messages
        .values()
        .filter(|message| message.thread_ts.is_none())
        .map(|message| {
            (
                (
                    message.team_id.clone(),
                    message.channel_id.clone(),
                    message.ts.clone(),
                ),
                message,
            )
        })
        .collect::<HashMap<_, _>>();
    let message_lookup = messages
        .values()
        .map(|message| {
            (
                (
                    message.team_id.clone(),
                    message.channel_id.clone(),
                    message.ts.clone(),
                ),
                message,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut aggregates = HashMap::<ThreadSummaryKey, ThreadSummaryAggregate>::new();

    for message in messages.values() {
        let root_ts = message
            .thread_ts
            .clone()
            .unwrap_or_else(|| message.ts.clone());
        let Some(root) = root_messages.get(&(
            message.team_id.clone(),
            message.channel_id.clone(),
            root_ts.clone(),
        )) else {
            continue;
        };
        let entry = aggregates
            .entry((
                message.team_id.clone(),
                message.channel_id.clone(),
                root_ts.clone(),
            ))
            .or_insert_with(|| ThreadSummaryAggregate {
                team_id: message.team_id.clone(),
                channel_id: message.channel_id.clone(),
                root_ts: root_ts.clone(),
                title: summarize_text(&root.text),
                preview: summarize_text(&root.text),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: message.ts.clone(),
                last_activity_seconds: parse_ts_seconds(&message.ts),
            });

        if message.ts != root_ts {
            entry.reply_count += 1;
        }
        if let Some(user_id) = &message.user_id {
            entry.participants.insert(user_id.clone());
        }
        update_last_activity(entry, &message.ts);
    }

    for (team_id, channel_id, message_ts, user_id, _) in reactions {
        let Some(message) =
            message_lookup.get(&(team_id.clone(), channel_id.clone(), message_ts.clone()))
        else {
            continue;
        };
        let root_ts = message
            .thread_ts
            .clone()
            .unwrap_or_else(|| message.ts.clone());
        let Some(root) = root_messages.get(&(
            message.team_id.clone(),
            message.channel_id.clone(),
            root_ts.clone(),
        )) else {
            continue;
        };
        let entry = aggregates
            .entry((message.team_id.clone(), message.channel_id.clone(), root_ts))
            .or_insert_with(|| ThreadSummaryAggregate {
                team_id: message.team_id.clone(),
                channel_id: message.channel_id.clone(),
                root_ts: root.ts.clone(),
                title: summarize_text(&root.text),
                preview: summarize_text(&root.text),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: message_ts.clone(),
                last_activity_seconds: parse_ts_seconds(message_ts),
            });

        entry.reaction_count += 1;
        entry.participants.insert(user_id.clone());
        update_last_activity(entry, message_ts);
    }

    for file in files.values() {
        let Some(message) = message_lookup.get(&(
            file.team_id.clone(),
            file.channel_id.clone(),
            file.message_ts.clone(),
        )) else {
            continue;
        };
        let root_ts = message
            .thread_ts
            .clone()
            .unwrap_or_else(|| message.ts.clone());
        let Some(root) = root_messages.get(&(
            message.team_id.clone(),
            message.channel_id.clone(),
            root_ts.clone(),
        )) else {
            continue;
        };
        let entry = aggregates
            .entry((message.team_id.clone(), message.channel_id.clone(), root_ts))
            .or_insert_with(|| ThreadSummaryAggregate {
                team_id: message.team_id.clone(),
                channel_id: message.channel_id.clone(),
                root_ts: root.ts.clone(),
                title: summarize_text(&root.text),
                preview: summarize_text(&root.text),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: file.message_ts.clone(),
                last_activity_seconds: parse_ts_seconds(&file.message_ts),
            });

        entry.file_count += 1;
        update_last_activity(entry, &file.message_ts);
    }

    let mut thread_summaries = ThreadSummaryMap::new();
    for aggregate in aggregates.into_values() {
        thread_summaries.insert(
            (
                aggregate.team_id.clone(),
                aggregate.channel_id.clone(),
                aggregate.root_ts.clone(),
            ),
            ThreadSummaryRow {
                team_id: aggregate.team_id,
                channel_id: aggregate.channel_id,
                root_ts: aggregate.root_ts,
                title: aggregate.title,
                preview: aggregate.preview,
                reply_count: aggregate.reply_count as i64,
                participant_count: aggregate.participants.len() as i64,
                reaction_count: aggregate.reaction_count as i64,
                file_count: aggregate.file_count as i64,
                last_activity_ts: aggregate.last_activity_ts,
            },
        );
    }

    thread_summaries
}

fn summarize_text(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "(no text)".to_owned();
    }
    normalized.chars().take(80).collect()
}

fn parse_ts_seconds(value: &str) -> i64 {
    value
        .split('.')
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
}

fn update_last_activity(aggregate: &mut ThreadSummaryAggregate, ts: &str) {
    let seconds = parse_ts_seconds(ts);
    if seconds >= aggregate.last_activity_seconds {
        aggregate.last_activity_seconds = seconds;
        aggregate.last_activity_ts = ts.to_owned();
    }
}

#[cfg(test)]
#[path = "thread_summary_index_tests.rs"]
mod tests;
