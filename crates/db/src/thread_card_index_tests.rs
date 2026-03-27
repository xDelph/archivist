use super::{build_thread_card, build_thread_cards};
use crate::ThreadSummaryRow;
use domain::Message;
use std::collections::HashMap;

#[test]
fn build_thread_card_uses_root_author_and_rollup_counts() {
    let root = Message {
        channel_id: "C123".to_owned(),
        ts: "1700000000.000001".to_owned(),
        thread_ts: None,
        user_id: Some("U123".to_owned()),
        text: "Thread root".to_owned(),
    };
    let summary = ThreadSummaryRow {
        channel_id: "C123".to_owned(),
        root_ts: "1700000000.000001".to_owned(),
        reply_count: 3,
        participant_count: 4,
        reaction_count: 2,
        file_count: 1,
        root_message_at: "1700000000.000001".to_owned(),
        last_activity_ts: "1700000005.000001".to_owned(),
    };

    let card = build_thread_card(&root, &summary);

    assert_eq!(card.author_user_id.as_deref(), Some("U123"));
    assert_eq!(card.title, "Thread root");
    assert_eq!(card.preview, "Thread root");
    assert_eq!(card.reply_count, 3);
    assert_eq!(card.participant_count, 4);
    assert_eq!(card.reaction_count, 2);
    assert_eq!(card.file_count, 1);
    assert_eq!(card.last_activity_ts, "1700000005.000001");
}

#[test]
fn build_thread_cards_skips_threads_without_a_root_message() {
    let messages = HashMap::from([(
        ("C123".to_owned(), "1700000001.000001".to_owned()),
        Message {
            channel_id: "C123".to_owned(),
            ts: "1700000001.000001".to_owned(),
            thread_ts: Some("1700000000.000001".to_owned()),
            user_id: Some("U123".to_owned()),
            text: "reply".to_owned(),
        },
    )]);
    let summaries = HashMap::from([(
        ("C123".to_owned(), "1700000000.000001".to_owned()),
        ThreadSummaryRow {
            channel_id: "C123".to_owned(),
            root_ts: "1700000000.000001".to_owned(),
            reply_count: 1,
            participant_count: 1,
            reaction_count: 0,
            file_count: 0,
            root_message_at: "1700000000.000001".to_owned(),
            last_activity_ts: "1700000001.000001".to_owned(),
        },
    )]);

    let cards = build_thread_cards(&messages, &summaries);

    assert!(cards.is_empty());
}

#[test]
fn build_thread_card_uses_placeholder_for_empty_text() {
    let root = Message {
        channel_id: "C123".to_owned(),
        ts: "1700000000.000001".to_owned(),
        thread_ts: None,
        user_id: None,
        text: "   ".to_owned(),
    };
    let summary = ThreadSummaryRow {
        channel_id: "C123".to_owned(),
        root_ts: "1700000000.000001".to_owned(),
        reply_count: 0,
        participant_count: 0,
        reaction_count: 0,
        file_count: 0,
        root_message_at: "1700000000.000001".to_owned(),
        last_activity_ts: "1700000000.000001".to_owned(),
    };

    let card = build_thread_card(&root, &summary);

    assert_eq!(card.title, "(no text)");
    assert_eq!(card.preview, "(no text)");
}
