use super::build_thread_summaries;
use domain::{File, Message};
use std::collections::{HashMap, HashSet};

#[test]
fn rebuild_thread_summaries_rolls_up_thread_activity() {
    let mut messages = HashMap::new();
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
    let reactions = HashSet::from([(
        "C123".to_owned(),
        "1700000000.000002".to_owned(),
        "U789".to_owned(),
        "eyes".to_owned(),
    )]);
    let files = HashMap::from([(
        (
            "C123".to_owned(),
            "1700000000.000002".to_owned(),
            "F123".to_owned(),
        ),
        File {
            id: "F123".to_owned(),
            channel_id: "C123".to_owned(),
            message_ts: "1700000000.000002".to_owned(),
            name: "brief.pdf".to_owned(),
            mimetype: Some("application/pdf".to_owned()),
            permalink: None,
            size: Some(42),
        },
    )]);
    let thread_summaries = build_thread_summaries(&messages, &reactions, &files);

    let summary = thread_summaries
        .get(&("C123".to_owned(), "1700000000.000001".to_owned()))
        .expect("summary");
    assert_eq!(summary.reply_count, 1);
    assert_eq!(summary.participant_count, 3);
    assert_eq!(summary.reaction_count, 1);
    assert_eq!(summary.file_count, 1);
    assert_eq!(summary.last_activity_ts, "1700000000.000002");
}

#[test]
fn rebuild_thread_summaries_keeps_channel_threads_separated() {
    let mut messages = HashMap::new();
    messages.insert(
        ("C123".to_owned(), "1700000000.000001".to_owned()),
        Message {
            channel_id: "C123".to_owned(),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            user_id: Some("U123".to_owned()),
            text: "channel one root".to_owned(),
        },
    );
    messages.insert(
        ("C999".to_owned(), "1700000000.000001".to_owned()),
        Message {
            channel_id: "C999".to_owned(),
            ts: "1700000000.000001".to_owned(),
            thread_ts: None,
            user_id: Some("U999".to_owned()),
            text: "channel two root".to_owned(),
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

    let thread_summaries = build_thread_summaries(&messages, &HashSet::new(), &HashMap::new());

    assert_eq!(thread_summaries.len(), 2);
    assert_eq!(
        thread_summaries
            .get(&("C123".to_owned(), "1700000000.000001".to_owned()))
            .map(|summary| summary.reply_count),
        Some(1)
    );
    assert_eq!(
        thread_summaries
            .get(&("C999".to_owned(), "1700000000.000001".to_owned()))
            .map(|summary| summary.reply_count),
        Some(0)
    );
}

#[test]
fn rebuild_thread_summaries_treats_thread_ts_equal_to_ts_as_root() {
    let messages = HashMap::from([
        (
            ("C123".to_owned(), "1700000000.000001".to_owned()),
            Message {
                channel_id: "C123".to_owned(),
                ts: "1700000000.000001".to_owned(),
                thread_ts: Some("1700000000.000001".to_owned()),
                user_id: Some("U123".to_owned()),
                text: "root summary".to_owned(),
            },
        ),
        (
            ("C123".to_owned(), "1700000000.000002".to_owned()),
            Message {
                channel_id: "C123".to_owned(),
                ts: "1700000000.000002".to_owned(),
                thread_ts: Some("1700000000.000001".to_owned()),
                user_id: Some("U456".to_owned()),
                text: "reply details".to_owned(),
            },
        ),
    ]);

    let thread_summaries = build_thread_summaries(&messages, &HashSet::new(), &HashMap::new());

    assert_eq!(
        thread_summaries
            .get(&("C123".to_owned(), "1700000000.000001".to_owned()))
            .map(|summary| summary.reply_count),
        Some(1)
    );
}

#[test]
fn rebuild_thread_summaries_keeps_microsecond_precision_for_last_activity() {
    let messages = HashMap::from([
        (
            ("C123".to_owned(), "1700000000.100001".to_owned()),
            Message {
                channel_id: "C123".to_owned(),
                ts: "1700000000.100001".to_owned(),
                thread_ts: None,
                user_id: Some("U123".to_owned()),
                text: "root summary".to_owned(),
            },
        ),
        (
            ("C123".to_owned(), "1700000000.100009".to_owned()),
            Message {
                channel_id: "C123".to_owned(),
                ts: "1700000000.100009".to_owned(),
                thread_ts: Some("1700000000.100001".to_owned()),
                user_id: Some("U456".to_owned()),
                text: "later reply in same second".to_owned(),
            },
        ),
    ]);

    let thread_summaries = build_thread_summaries(&messages, &HashSet::new(), &HashMap::new());

    assert_eq!(
        thread_summaries
            .get(&("C123".to_owned(), "1700000000.100001".to_owned()))
            .map(|summary| summary.last_activity_ts.as_str()),
        Some("1700000000.100009")
    );
}
