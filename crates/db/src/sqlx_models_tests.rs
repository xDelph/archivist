use super::{
    AnalyticsEventRow, ChannelRow, FileRow, MessageRow, ReactionRow, SavedItemRow,
    SearchDocumentRow, ThreadSummaryRow, UserRow,
};

#[test]
fn sqlx_rows_cover_core_entities() {
    let message = MessageRow {
        team_id: "T123".to_owned(),
        channel_id: "C123".to_owned(),
        ts: "1700000000.000001".to_owned(),
        thread_ts: None,
        user_id: Some("U123".to_owned()),
        text: "hello".to_owned(),
    };
    let reaction = ReactionRow {
        team_id: "T123".to_owned(),
        channel_id: "C123".to_owned(),
        message_ts: message.ts.clone(),
        user_id: "U123".to_owned(),
        name: "thumbsup".to_owned(),
    };
    let file = FileRow {
        id: "F123".to_owned(),
        team_id: "T123".to_owned(),
        channel_id: "C123".to_owned(),
        message_ts: message.ts.clone(),
        name: "brief.pdf".to_owned(),
        mimetype: Some("application/pdf".to_owned()),
        permalink: Some("https://files.example.com/brief.pdf".to_owned()),
        size_bytes: Some(42),
    };
    let channel = ChannelRow {
        team_id: "T123".to_owned(),
        id: "C123".to_owned(),
        kind: "public".to_owned(),
        name: Some("general".to_owned()),
        is_archived: false,
    };
    let user = UserRow {
        team_id: "T123".to_owned(),
        id: "U123".to_owned(),
        display_name: Some("Thomas".to_owned()),
        avatar_url: Some("https://example.com/avatar.png".to_owned()),
        is_active: true,
    };
    let search_document = SearchDocumentRow {
        team_id: "T123".to_owned(),
        channel_id: "C123".to_owned(),
        message_ts: message.ts.clone(),
        title: Some("General".to_owned()),
        body: "hello".to_owned(),
    };
    let analytics_event = AnalyticsEventRow {
        event_name: "thread_viewed".to_owned(),
        subject_id: Some("thread_1".to_owned()),
        payload_json: "{}".to_owned(),
    };
    let saved_item = SavedItemRow {
        team_id: "T123".to_owned(),
        slack_user_id: "U123".to_owned(),
        thread_id: "C123:1700000000.000001".to_owned(),
        channel_id: "C123".to_owned(),
        root_ts: message.ts.clone(),
        title: "General".to_owned(),
        preview: "hello".to_owned(),
        last_activity_ts: "1700000000.000002".to_owned(),
        saved_at: "2026-03-09T12:00:00Z".to_owned(),
    };
    let thread_summary = ThreadSummaryRow {
        team_id: "T123".to_owned(),
        channel_id: "C123".to_owned(),
        root_ts: message.ts.clone(),
        title: "General".to_owned(),
        preview: "hello".to_owned(),
        reply_count: 1,
        participant_count: 2,
        reaction_count: 1,
        file_count: 1,
        last_activity_ts: "1700000000.000002".to_owned(),
    };

    assert_eq!(reaction.name, "thumbsup");
    assert_eq!(file.name, "brief.pdf");
    assert_eq!(channel.name.as_deref(), Some("general"));
    assert!(user.is_active);
    assert_eq!(search_document.body, "hello");
    assert_eq!(saved_item.thread_id, "C123:1700000000.000001");
    assert_eq!(thread_summary.participant_count, 2);
    assert_eq!(analytics_event.event_name, "thread_viewed");
}
