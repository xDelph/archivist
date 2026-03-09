use crate::AppState;
use axum::{Json, extract::State};
use domain::{Channel, ChannelKind, File, Message, Reaction};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ChannelSummaryResponse {
    id: String,
    name: Option<String>,
    kind: &'static str,
    is_archived: bool,
    message_count: usize,
    reaction_count: usize,
    file_count: usize,
    last_message_ts: Option<String>,
}

#[derive(Debug)]
struct ChannelSummaryBuilder {
    id: String,
    name: Option<String>,
    kind: ChannelKind,
    is_archived: bool,
    message_count: usize,
    reaction_count: usize,
    file_count: usize,
    last_message_ts: Option<String>,
}

impl Default for ChannelSummaryBuilder {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: None,
            kind: ChannelKind::Unknown,
            is_archived: false,
            message_count: 0,
            reaction_count: 0,
            file_count: 0,
            last_message_ts: None,
        }
    }
}

pub(crate) async fn channels(State(state): State<AppState>) -> Json<Vec<ChannelSummaryResponse>> {
    Json(
        build_channel_summaries(
            state.store.channels().await,
            state.store.messages().await,
            state.store.reactions().await,
            state.store.files().await,
        )
        .into_iter()
        .map(|summary| ChannelSummaryResponse {
            id: summary.id,
            name: summary.name,
            kind: summary.kind.as_str(),
            is_archived: summary.is_archived,
            message_count: summary.message_count,
            reaction_count: summary.reaction_count,
            file_count: summary.file_count,
            last_message_ts: summary.last_message_ts,
        })
        .collect(),
    )
}

fn build_channel_summaries(
    channels: Vec<Channel>,
    messages: Vec<Message>,
    reactions: Vec<Reaction>,
    files: Vec<File>,
) -> Vec<ChannelSummaryBuilder> {
    let mut summaries = HashMap::<String, ChannelSummaryBuilder>::new();

    for channel in channels {
        let summary =
            summaries
                .entry(channel.id.clone())
                .or_insert_with(|| ChannelSummaryBuilder {
                    id: channel.id.clone(),
                    kind: channel.kind,
                    ..ChannelSummaryBuilder::default()
                });
        summary.name = channel.name;
        summary.kind = channel.kind;
        summary.is_archived = channel.is_archived;
    }

    for message in messages {
        let summary = summaries
            .entry(message.channel_id.clone())
            .or_insert_with(|| inferred_channel_summary(&message.channel_id));
        summary.message_count += 1;
        if summary
            .last_message_ts
            .as_deref()
            .is_none_or(|current| current < message.ts.as_str())
        {
            summary.last_message_ts = Some(message.ts);
        }
    }

    for reaction in reactions {
        let summary = summaries
            .entry(reaction.channel_id.clone())
            .or_insert_with(|| inferred_channel_summary(&reaction.channel_id));
        summary.reaction_count += 1;
    }

    for file in files {
        let summary = summaries
            .entry(file.channel_id.clone())
            .or_insert_with(|| inferred_channel_summary(&file.channel_id));
        summary.file_count += 1;
        if summary
            .last_message_ts
            .as_deref()
            .is_none_or(|current| current < file.message_ts.as_str())
        {
            summary.last_message_ts = Some(file.message_ts);
        }
    }

    let mut summaries = summaries.into_values().collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        (
            right.last_message_ts.as_deref(),
            left.is_archived,
            left.name.as_deref(),
            left.id.as_str(),
        )
            .cmp(&(
                left.last_message_ts.as_deref(),
                right.is_archived,
                right.name.as_deref(),
                right.id.as_str(),
            ))
    });
    summaries
}

fn inferred_channel_summary(channel_id: &str) -> ChannelSummaryBuilder {
    ChannelSummaryBuilder {
        id: channel_id.to_owned(),
        kind: ChannelKind::from_channel_id(channel_id),
        ..ChannelSummaryBuilder::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::{ApiConfig, auth::build_session_token, auth::current_unix_timestamp, build_router};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use db::JsonlEventStore;
    use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn channels_route_requires_authenticated_session() {
        let tempdir = tempdir().expect("tempdir");
        let response = build_router(ApiConfig {
            host: "127.0.0.1".to_owned(),
            port: 4000,
            event_log_path: tempdir.path().join("events.jsonl").display().to_string(),
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_workspace_id: None,
            slack_token_url: None,
            session_secret: Some("session_secret".to_owned()),
            auth_store_path: tempdir
                .path()
                .join("auth-identities.json")
                .display()
                .to_string(),
            synced_users_path: tempdir
                .path()
                .join("synced-users.json")
                .display()
                .to_string(),
        })
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/channels")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn channels_route_reports_activity_and_channel_metadata() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&path).await.expect("store");

        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_message".to_owned(),
                team_id: "T123".to_owned(),
                event_time: 1,
                received_at: 2,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some("hello".to_owned()),
                    ts: "1700000000.000010".to_owned(),
                    thread_ts: None,
                    files: vec![SharedFile {
                        id: "F123".to_owned(),
                        name: "brief.pdf".to_owned(),
                        mimetype: Some("application/pdf".to_owned()),
                        permalink: None,
                        size: Some(42),
                    }],
                },
            })
            .await
            .expect("message insert");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_reaction".to_owned(),
                team_id: "T123".to_owned(),
                event_time: 3,
                received_at: 4,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ReactionAdded {
                    user_id: "U456".to_owned(),
                    reaction: "thumbsup".to_owned(),
                    item_ts: "1700000000.000010".to_owned(),
                },
            })
            .await
            .expect("reaction insert");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_channel".to_owned(),
                team_id: "T123".to_owned(),
                event_time: 5,
                received_at: 6,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ChannelUpdated {
                    name: Some("announcements".to_owned()),
                    is_archived: Some(false),
                },
            })
            .await
            .expect("channel insert");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_message_2".to_owned(),
                team_id: "T123".to_owned(),
                event_time: 7,
                received_at: 8,
                channel_id: "C999".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U789".to_owned()),
                    text: Some("second channel".to_owned()),
                    ts: "1700000000.000009".to_owned(),
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("second message insert");
        let session_token = build_session_token(
            "session_secret",
            &crate::auth::SessionClaims {
                slack_user_id: "U123".to_owned(),
                team_id: "T123".to_owned(),
                email: None,
                display_name: Some("Thomas".to_owned()),
                avatar_url: None,
                exp: current_unix_timestamp() + 60,
            },
        )
        .expect("session token");

        let response = build_router(ApiConfig {
            host: "127.0.0.1".to_owned(),
            port: 4000,
            event_log_path: path.display().to_string(),
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_workspace_id: None,
            slack_token_url: None,
            session_secret: Some("session_secret".to_owned()),
            auth_store_path: tempdir
                .path()
                .join("auth-identities.json")
                .display()
                .to_string(),
            synced_users_path: tempdir
                .path()
                .join("synced-users.json")
                .display()
                .to_string(),
        })
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/api/channels")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");

        assert_eq!(payload.as_array().map(Vec::len), Some(2));
        assert_eq!(payload[0]["id"], "C123");
        assert_eq!(payload[0]["name"], "announcements");
        assert_eq!(payload[0]["kind"], "public");
        assert_eq!(payload[0]["message_count"], 1);
        assert_eq!(payload[0]["reaction_count"], 1);
        assert_eq!(payload[0]["file_count"], 1);
        assert_eq!(payload[1]["id"], "C999");
        assert_eq!(payload[1]["name"], serde_json::Value::Null);
        assert_eq!(payload[1]["kind"], "public");
        assert_eq!(payload[1]["message_count"], 1);
    }
}
