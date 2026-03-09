use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct MessageRow {
    pub team_id: String,
    pub channel_id: String,
    pub ts: String,
    pub thread_ts: Option<String>,
    pub user_id: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ReactionRow {
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub user_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct FileRow {
    pub id: String,
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub name: String,
    pub mimetype: Option<String>,
    pub permalink: Option<String>,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ChannelRow {
    pub team_id: String,
    pub id: String,
    pub kind: String,
    pub name: Option<String>,
    pub is_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct UserRow {
    pub team_id: String,
    pub id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct SearchDocumentRow {
    pub team_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub title: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ThreadSummaryRow {
    pub team_id: String,
    pub channel_id: String,
    pub root_ts: String,
    pub title: String,
    pub preview: String,
    pub reply_count: i64,
    pub participant_count: i64,
    pub reaction_count: i64,
    pub file_count: i64,
    pub last_activity_ts: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct AnalyticsEventRow {
    pub event_name: String,
    pub subject_id: Option<String>,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct SavedItemRow {
    pub team_id: String,
    pub slack_user_id: String,
    pub thread_id: String,
    pub channel_id: String,
    pub root_ts: String,
    pub title: String,
    pub preview: String,
    pub last_activity_ts: String,
    pub saved_at: String,
}

#[cfg(test)]
mod tests {
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
}
