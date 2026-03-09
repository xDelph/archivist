use crate::SearchDocumentRow;
use domain::Message;
use std::collections::HashMap;

pub(crate) type MessageMap = HashMap<(String, String), Message>;
pub(crate) type SearchDocumentMap = HashMap<(String, String, String), SearchDocumentRow>;

pub(crate) fn refresh_search_documents(
    search_documents: &mut SearchDocumentMap,
    messages: &MessageMap,
    team_id: &str,
    channel_id: &str,
    root_ts: &str,
) {
    let root_key = (channel_id.to_owned(), root_ts.to_owned());
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
                message_ts: message.ts.clone(),
                title: root_title.clone().or_else(|| optional_text(&message.text)),
                body: message.text.clone(),
            },
        );
    }
}

fn optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{MessageMap, SearchDocumentMap, refresh_search_documents};
    use domain::Message;

    #[test]
    fn refresh_search_documents_uses_root_titles_for_entire_threads() {
        let mut messages = MessageMap::new();
        messages.insert(
            ("C123".to_owned(), "1700000000.000001".to_owned()),
            Message {
                team_id: "T123".to_owned(),
                channel_id: "C123".to_owned(),
                ts: "1700000000.000001".to_owned(),
                thread_ts: None,
                user_id: Some("U123".to_owned()),
                text: "root summary".to_owned(),
            },
        );
        messages.insert(
            ("C123".to_owned(), "1700000000.000002".to_owned()),
            Message {
                team_id: "T123".to_owned(),
                channel_id: "C123".to_owned(),
                ts: "1700000000.000002".to_owned(),
                thread_ts: Some("1700000000.000001".to_owned()),
                user_id: Some("U456".to_owned()),
                text: "reply details".to_owned(),
            },
        );
        let mut search_documents = SearchDocumentMap::new();

        refresh_search_documents(
            &mut search_documents,
            &messages,
            "T123",
            "C123",
            "1700000000.000001",
        );

        assert_eq!(search_documents.len(), 2);
        assert_eq!(
            search_documents
                .get(&(
                    "T123".to_owned(),
                    "C123".to_owned(),
                    "1700000000.000002".to_owned()
                ))
                .and_then(|document| document.title.as_deref()),
            Some("root summary")
        );
    }
}
