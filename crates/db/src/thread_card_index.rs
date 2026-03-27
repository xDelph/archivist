use crate::{ThreadCardRow, ThreadSummaryRow};
use domain::Message;
use std::collections::HashMap;

type MessageKey = (String, String);
type ThreadCardKey = (String, String);
type ThreadSummaryKey = (String, String);

pub(crate) type ThreadCardMap = HashMap<ThreadCardKey, ThreadCardRow>;

const NO_TEXT_PLACEHOLDER: &str = "(no text)";
const MAX_PREVIEW_CHARS: usize = 700;

pub(crate) fn build_thread_cards(
    messages: &HashMap<MessageKey, Message>,
    thread_summaries: &HashMap<ThreadSummaryKey, ThreadSummaryRow>,
) -> ThreadCardMap {
    thread_summaries
        .values()
        .filter_map(|summary| {
            let root = messages.get(&(summary.channel_id.clone(), summary.root_ts.clone()))?;
            Some((
                (summary.channel_id.clone(), summary.root_ts.clone()),
                build_thread_card(root, summary),
            ))
        })
        .collect()
}

pub(crate) fn build_thread_card(root: &Message, summary: &ThreadSummaryRow) -> ThreadCardRow {
    let text = normalize_thread_card_text(&root.text);

    ThreadCardRow {
        channel_id: summary.channel_id.clone(),
        root_ts: summary.root_ts.clone(),
        author_user_id: root.user_id.clone(),
        title: text.clone(),
        preview: text,
        reply_count: summary.reply_count,
        participant_count: summary.participant_count,
        reaction_count: summary.reaction_count,
        file_count: summary.file_count,
        root_message_at: summary.root_message_at.clone(),
        last_activity_ts: summary.last_activity_ts.clone(),
    }
}

fn normalize_thread_card_text(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").trim().to_owned();
    if normalized.is_empty() {
        return NO_TEXT_PLACEHOLDER.to_owned();
    }

    truncate_text(&normalized, MAX_PREVIEW_CHARS)
}

fn truncate_text(value: &str, limit: usize) -> String {
    let mut truncated = value.chars().take(limit).collect::<String>();
    if value.chars().count() > limit {
        truncated.push('…');
    }

    truncated
}

#[cfg(test)]
#[path = "thread_card_index_tests.rs"]
mod tests;
