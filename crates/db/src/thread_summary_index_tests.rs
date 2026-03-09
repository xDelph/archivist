use super::build_thread_summaries;
use domain::{File, Message};
use std::collections::{HashMap, HashSet};

#[test]
fn rebuild_thread_summaries_rolls_up_thread_activity() {
    let mut messages = HashMap::new();
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
    let reactions = HashSet::from([(
        "T123".to_owned(),
        "C123".to_owned(),
        "1700000000.000002".to_owned(),
        "U789".to_owned(),
        "eyes".to_owned(),
    )]);
    let files = HashMap::from([(
        ("T123".to_owned(), "F123".to_owned()),
        File {
            id: "F123".to_owned(),
            team_id: "T123".to_owned(),
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
        .get(&(
            "T123".to_owned(),
            "C123".to_owned(),
            "1700000000.000001".to_owned(),
        ))
        .expect("summary");
    assert_eq!(summary.title, "root summary");
    assert_eq!(summary.reply_count, 1);
    assert_eq!(summary.participant_count, 3);
    assert_eq!(summary.reaction_count, 1);
    assert_eq!(summary.file_count, 1);
    assert_eq!(summary.last_activity_ts, "1700000000.000002");
}
