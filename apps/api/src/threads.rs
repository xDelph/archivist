use crate::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use domain::{File, Message, Reaction};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ThreadDetailResponse {
    id: String,
    channel_id: String,
    root_ts: String,
    reply_count: usize,
    messages: Vec<ThreadMessageResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ThreadMessageResponse {
    ts: String,
    thread_ts: Option<String>,
    user_id: Option<String>,
    text: String,
    reactions: Vec<ThreadReactionResponse>,
    files: Vec<ThreadFileResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ThreadReactionResponse {
    user_id: String,
    name: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ThreadFileResponse {
    id: String,
    name: String,
    mimetype: Option<String>,
    permalink: Option<String>,
    size: Option<u64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

pub(crate) async fn thread_detail(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ThreadDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (channel_id, root_ts) = parse_thread_id(&id).ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_thread_id",
        }),
    ))?;

    let response = build_thread_detail(
        &id,
        channel_id,
        root_ts,
        state.store.messages().await,
        state.store.reactions().await,
        state.store.files().await,
    )
    .ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "thread_not_found",
        }),
    ))?;

    Ok(Json(response))
}

fn parse_thread_id(thread_id: &str) -> Option<(&str, &str)> {
    let (channel_id, root_ts) = thread_id.split_once(':')?;
    if channel_id.is_empty() || root_ts.is_empty() {
        return None;
    }
    Some((channel_id, root_ts))
}

fn build_thread_detail(
    id: &str,
    channel_id: &str,
    root_ts: &str,
    messages: Vec<Message>,
    reactions: Vec<Reaction>,
    files: Vec<File>,
) -> Option<ThreadDetailResponse> {
    let mut thread_messages = messages
        .into_iter()
        .filter(|message| {
            message.channel_id == channel_id
                && (message.ts == root_ts || message.thread_ts.as_deref() == Some(root_ts))
        })
        .collect::<Vec<_>>();
    thread_messages.sort_by(|left, right| left.ts.cmp(&right.ts));

    if !thread_messages.iter().any(|message| message.ts == root_ts) {
        return None;
    }

    let message_ids = thread_messages
        .iter()
        .map(|message| message.ts.clone())
        .collect::<HashSet<_>>();
    let mut reactions_by_message = HashMap::<String, Vec<ThreadReactionResponse>>::new();
    for reaction in reactions.into_iter().filter(|reaction| {
        reaction.channel_id == channel_id && message_ids.contains(&reaction.message_ts)
    }) {
        reactions_by_message
            .entry(reaction.message_ts)
            .or_default()
            .push(ThreadReactionResponse {
                user_id: reaction.user_id,
                name: reaction.name,
            });
    }
    for reactions in reactions_by_message.values_mut() {
        reactions
            .sort_by(|left, right| (&left.name, &left.user_id).cmp(&(&right.name, &right.user_id)));
    }

    let mut files_by_message = HashMap::<String, Vec<ThreadFileResponse>>::new();
    for file in files
        .into_iter()
        .filter(|file| file.channel_id == channel_id && message_ids.contains(&file.message_ts))
    {
        files_by_message
            .entry(file.message_ts)
            .or_default()
            .push(ThreadFileResponse {
                id: file.id,
                name: file.name,
                mimetype: file.mimetype,
                permalink: file.permalink,
                size: file.size,
            });
    }
    for files in files_by_message.values_mut() {
        files.sort_by(|left, right| left.id.cmp(&right.id));
    }

    let messages = thread_messages
        .into_iter()
        .map(|message| ThreadMessageResponse {
            reactions: reactions_by_message.remove(&message.ts).unwrap_or_default(),
            files: files_by_message.remove(&message.ts).unwrap_or_default(),
            ts: message.ts,
            thread_ts: message.thread_ts,
            user_id: message.user_id,
            text: message.text,
        })
        .collect::<Vec<_>>();
    let reply_count = messages.len().saturating_sub(1);

    Some(ThreadDetailResponse {
        id: id.to_owned(),
        channel_id: channel_id.to_owned(),
        root_ts: root_ts.to_owned(),
        reply_count,
        messages,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_thread_detail, parse_thread_id};
    use crate::{ApiConfig, build_router};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use db::JsonlEventStore;
    use domain::{ChannelKind, EventPayload, Message, ProcessEventJob, SharedFile};
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    #[test]
    fn parse_thread_id_requires_channel_and_timestamp() {
        assert_eq!(
            parse_thread_id("C123:1700000000.000001"),
            Some(("C123", "1700000000.000001"))
        );
        assert_eq!(parse_thread_id("C123"), None);
        assert_eq!(parse_thread_id(":1700000000.000001"), None);
    }

    #[test]
    fn build_thread_detail_requires_a_root_message() {
        let detail = build_thread_detail(
            "C123:1700000000.000001",
            "C123",
            "1700000000.000001",
            vec![Message {
                team_id: "T123".to_owned(),
                channel_id: "C123".to_owned(),
                ts: "1700000000.000002".to_owned(),
                thread_ts: Some("1700000000.000001".to_owned()),
                user_id: Some("U123".to_owned()),
                text: "reply only".to_owned(),
            }],
            vec![],
            vec![],
        );

        assert_eq!(detail, None);
    }

    #[tokio::test]
    async fn thread_detail_route_returns_messages_reactions_and_files() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&path).await.expect("store");
        let root_ts = "1700000000.000001";

        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_root".to_owned(),
                team_id: "T123".to_owned(),
                event_time: 1,
                received_at: 2,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some("root message".to_owned()),
                    ts: root_ts.to_owned(),
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
            .expect("root insert");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_reply".to_owned(),
                team_id: "T123".to_owned(),
                event_time: 3,
                received_at: 4,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U456".to_owned()),
                    text: Some("reply message".to_owned()),
                    ts: "1700000000.000002".to_owned(),
                    thread_ts: Some(root_ts.to_owned()),
                    files: vec![],
                },
            })
            .await
            .expect("reply insert");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_reaction".to_owned(),
                team_id: "T123".to_owned(),
                event_time: 5,
                received_at: 6,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ReactionAdded {
                    user_id: "U789".to_owned(),
                    reaction: "eyes".to_owned(),
                    item_ts: root_ts.to_owned(),
                },
            })
            .await
            .expect("reaction insert");

        let response = build_router(ApiConfig {
            host: "127.0.0.1".to_owned(),
            port: 4000,
            event_log_path: path.display().to_string(),
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_workspace_id: None,
            slack_token_url: None,
            session_secret: None,
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
                .uri("/api/threads/C123:1700000000.000001")
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

        assert_eq!(payload["reply_count"], 1);
        assert_eq!(payload["messages"][0]["text"], "root message");
        assert_eq!(payload["messages"][0]["reactions"][0]["name"], "eyes");
        assert_eq!(payload["messages"][0]["files"][0]["name"], "brief.pdf");
        assert_eq!(payload["messages"][1]["text"], "reply message");
    }

    #[tokio::test]
    async fn thread_detail_route_rejects_invalid_ids() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");

        let response = build_router(ApiConfig {
            host: "127.0.0.1".to_owned(),
            port: 4000,
            event_log_path: path.display().to_string(),
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_workspace_id: None,
            slack_token_url: None,
            session_secret: None,
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
                .uri("/api/threads/not-a-thread-id")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
