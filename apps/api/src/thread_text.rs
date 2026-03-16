use domain::Message;
use std::collections::HashMap;

pub(crate) type ThreadKey = (String, String);

const NO_TEXT_PLACEHOLDER: &str = "(no text)";
const MAX_PREVIEW_CHARS: usize = 700;

pub(crate) fn build_root_message_text_map(messages: &[Message]) -> HashMap<ThreadKey, String> {
    messages
        .iter()
        .filter(|message| is_root_message(message))
        .map(|message| {
            (
                (message.channel_id.clone(), message.ts.clone()),
                normalize_thread_text(&message.text),
            )
        })
        .collect()
}

pub(crate) fn lookup_root_message_text(
    root_texts: &HashMap<ThreadKey, String>,
    channel_id: &str,
    root_ts: &str,
) -> String {
    root_texts
        .get(&(channel_id.to_owned(), root_ts.to_owned()))
        .cloned()
        .unwrap_or_else(|| NO_TEXT_PLACEHOLDER.to_owned())
}

pub(crate) fn thread_preview_text(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").trim().to_owned();
    if normalized.is_empty() {
        return NO_TEXT_PLACEHOLDER.to_owned();
    }

    truncate_text(&normalized, MAX_PREVIEW_CHARS)
}

pub(crate) fn normalize_thread_text(value: &str) -> String {
    thread_preview_text(value)
}

fn is_root_message(message: &Message) -> bool {
    message
        .thread_ts
        .as_deref()
        .is_none_or(|thread_ts| thread_ts == message.ts)
}

fn truncate_text(value: &str, limit: usize) -> String {
    let mut truncated = value.chars().take(limit).collect::<String>();
    if value.chars().count() > limit {
        truncated.push('…');
    }

    truncated
}

#[cfg(test)]
#[path = "thread_text_tests.rs"]
mod tests;
