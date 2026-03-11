use super::{MessageMap, SearchDocumentMap, refresh_search_documents};
use domain::Message;

#[test]
fn refresh_search_documents_uses_root_titles_for_entire_threads() {
    let mut messages = MessageMap::new();
    messages.insert(
        ("C123".to_owned(), "1700000000.000001".to_owned()),
        Message {
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
        "C123",
        "1700000000.000001",
    );

    assert_eq!(search_documents.len(), 2);
    assert_eq!(
        search_documents
            .get(&("C123".to_owned(), "1700000000.000002".to_owned()))
            .and_then(|document| document.title.as_deref()),
        Some("root summary")
    );
}

#[test]
fn refresh_search_documents_keeps_channel_roots_isolated() {
    let mut messages = MessageMap::new();
    messages.insert(
        ("C123".to_owned(), "1700000000.000001".to_owned()),
        Message {
            channel_id: "C123".to_owned(),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            user_id: Some("U123".to_owned()),
            text: "team one root".to_owned(),
        },
    );
    messages.insert(
        ("C999".to_owned(), "1700000000.000001".to_owned()),
        Message {
            channel_id: "C999".to_owned(),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            user_id: Some("U999".to_owned()),
            text: "other channel root".to_owned(),
        },
    );
    messages.insert(
        ("C123".to_owned(), "1700000000.000002".to_owned()),
        Message {
            channel_id: "C123".to_owned(),
            ts: "1700000000.000002".to_owned(),
            thread_ts: Some("1700000000.000001".to_owned()),
            user_id: Some("U456".to_owned()),
            text: "channel one reply".to_owned(),
        },
    );
    let mut search_documents = SearchDocumentMap::new();

    refresh_search_documents(
        &mut search_documents,
        &messages,
        "C123",
        "1700000000.000001",
    );

    assert_eq!(
        search_documents
            .get(&("C123".to_owned(), "1700000000.000002".to_owned()))
            .and_then(|document| document.title.as_deref()),
        Some("team one root")
    );
    assert!(!search_documents.contains_key(&("C999".to_owned(), "1700000000.000001".to_owned())));
}
