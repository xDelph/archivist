use crate::ThreadSummaryRow;
use domain::{File, Message};
use std::collections::{HashMap, HashSet};

type MessageKey = (String, String);
type ReactionKey = (String, String, String, String);
type FileKey = (String, String, String);
type ThreadSummaryKey = (String, String);

pub(crate) type ThreadSummaryMap = HashMap<ThreadSummaryKey, ThreadSummaryRow>;

#[derive(Debug)]
struct ThreadSummaryAggregate {
    channel_id: String,
    root_ts: String,
    reply_count: usize,
    participants: HashSet<String>,
    reaction_count: usize,
    file_count: usize,
    last_activity_ts: String,
    last_activity_micros: i64,
}

pub(crate) fn build_thread_summaries(
    messages: &HashMap<MessageKey, Message>,
    reactions: &HashSet<ReactionKey>,
    files: &HashMap<FileKey, File>,
) -> ThreadSummaryMap {
    let root_messages = messages
        .values()
        .filter(|message| is_root_message(message))
        .map(|message| ((message.channel_id.clone(), message.ts.clone()), message))
        .collect::<HashMap<_, _>>();
    let message_lookup = messages
        .values()
        .map(|message| ((message.channel_id.clone(), message.ts.clone()), message))
        .collect::<HashMap<_, _>>();
    let mut aggregates = HashMap::<ThreadSummaryKey, ThreadSummaryAggregate>::new();

    for message in messages.values() {
        let root_ts = normalized_root_ts(message);
        if !root_messages.contains_key(&(message.channel_id.clone(), root_ts.clone())) {
            continue;
        }
        let entry = aggregates
            .entry((message.channel_id.clone(), root_ts.clone()))
            .or_insert_with(|| ThreadSummaryAggregate {
                channel_id: message.channel_id.clone(),
                root_ts: root_ts.clone(),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: message.ts.clone(),
                last_activity_micros: parse_ts_micros(&message.ts),
            });

        if !is_root_message(message) {
            entry.reply_count += 1;
        }
        if let Some(user_id) = &message.user_id {
            entry.participants.insert(user_id.clone());
        }
        update_last_activity(entry, &message.ts);
    }

    for (channel_id, message_ts, user_id, _) in reactions {
        let Some(message) = message_lookup.get(&(channel_id.clone(), message_ts.clone())) else {
            continue;
        };
        let root_ts = normalized_root_ts(message);
        let Some(root) = root_messages.get(&(message.channel_id.clone(), root_ts.clone())) else {
            continue;
        };
        let entry = aggregates
            .entry((message.channel_id.clone(), root_ts))
            .or_insert_with(|| ThreadSummaryAggregate {
                channel_id: message.channel_id.clone(),
                root_ts: root.ts.clone(),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: message_ts.clone(),
                last_activity_micros: parse_ts_micros(message_ts),
            });

        entry.reaction_count += 1;
        entry.participants.insert(user_id.clone());
        update_last_activity(entry, message_ts);
    }

    for file in files.values() {
        let Some(message) = message_lookup.get(&(file.channel_id.clone(), file.message_ts.clone()))
        else {
            continue;
        };
        let root_ts = normalized_root_ts(message);
        let Some(root) = root_messages.get(&(message.channel_id.clone(), root_ts.clone())) else {
            continue;
        };
        let entry = aggregates
            .entry((message.channel_id.clone(), root_ts))
            .or_insert_with(|| ThreadSummaryAggregate {
                channel_id: message.channel_id.clone(),
                root_ts: root.ts.clone(),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: file.message_ts.clone(),
                last_activity_micros: parse_ts_micros(&file.message_ts),
            });

        entry.file_count += 1;
        update_last_activity(entry, &file.message_ts);
    }

    let mut thread_summaries = ThreadSummaryMap::new();
    for aggregate in aggregates.into_values() {
        thread_summaries.insert((aggregate.channel_id.clone(), aggregate.root_ts.clone()), {
            let root_ts = aggregate.root_ts.clone();
            ThreadSummaryRow {
                channel_id: aggregate.channel_id,
                root_ts,
                reply_count: aggregate.reply_count as i64,
                participant_count: aggregate.participants.len() as i64,
                reaction_count: aggregate.reaction_count as i64,
                file_count: aggregate.file_count as i64,
                root_message_at: aggregate.root_ts,
                last_activity_ts: aggregate.last_activity_ts,
            }
        });
    }

    thread_summaries
}

fn is_root_message(message: &Message) -> bool {
    message
        .thread_ts
        .as_deref()
        .is_none_or(|thread_ts| thread_ts == message.ts)
}

fn normalized_root_ts(message: &Message) -> String {
    if is_root_message(message) {
        return message.ts.clone();
    }

    message
        .thread_ts
        .clone()
        .unwrap_or_else(|| message.ts.clone())
}

fn parse_ts_micros(value: &str) -> i64 {
    let (seconds, fraction) = value.trim().split_once('.').unwrap_or((value.trim(), "0"));
    let seconds = seconds.parse::<i64>().unwrap_or_default();
    let mut micros = fraction
        .chars()
        .take(6)
        .filter(char::is_ascii_digit)
        .collect::<String>();
    while micros.len() < 6 {
        micros.push('0');
    }
    seconds
        .saturating_mul(1_000_000)
        .saturating_add(micros.parse::<i64>().unwrap_or_default())
}

fn update_last_activity(aggregate: &mut ThreadSummaryAggregate, ts: &str) {
    let micros = parse_ts_micros(ts);
    if micros >= aggregate.last_activity_micros {
        aggregate.last_activity_micros = micros;
        aggregate.last_activity_ts = ts.to_owned();
    }
}

#[cfg(test)]
#[path = "thread_summary_index_tests.rs"]
mod tests;
