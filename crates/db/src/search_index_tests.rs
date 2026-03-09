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
