use crate::SearchDocumentRow;
use domain::Message;
use std::collections::HashMap;

pub(crate) type MessageMap = HashMap<(String, String, String), Message>;
pub(crate) type SearchDocumentMap = HashMap<(String, String, String), SearchDocumentRow>;

pub(crate) fn refresh_search_documents(
    search_documents: &mut SearchDocumentMap,
    messages: &MessageMap,
    team_id: &str,
    channel_id: &str,
    root_ts: &str,
) {
    let root_key = (
        team_id.to_owned(),
        channel_id.to_owned(),
        root_ts.to_owned(),
    );
    let root_title = messages
        .get(&root_key)
        .map(|message| optional_text(&message.text))
        .unwrap_or(None);

    for message in messages.values().filter(|message| {
        message.team_id == team_id
            && message.channel_id == channel_id
            && message.thread_ts.as_deref().unwrap_or(&message.ts) == root_ts
    }) {
        search_documents.insert(
            (
                message.team_id.clone(),
                message.channel_id.clone(),
                message.ts.clone(),
            ),
            SearchDocumentRow {
                team_id: message.team_id.clone(),
                channel_id: message.channel_id.clone(),
                root_ts: root_ts.to_owned(),
                message_ts: message.ts.clone(),
                title: root_title.clone().or_else(|| optional_text(&message.text)),
                body: message.text.clone(),
                message_occurred_at: message.ts.clone(),
            },
        );
    }
}

fn optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
#[path = "search_index_tests.rs"]
mod tests;
