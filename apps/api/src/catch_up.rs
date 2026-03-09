use crate::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use domain::{Channel, ChannelKind, File, Message, Reaction};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

const DAY_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Deserialize)]
pub(crate) struct CatchUpQuery {
    window: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct CatchUpResponse {
    window: &'static str,
    channels: Vec<CatchUpChannelResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CatchUpChannelResponse {
    id: String,
    name: Option<String>,
    kind: &'static str,
    is_archived: bool,
    thread_count: usize,
    threads: Vec<CatchUpThreadResponse>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CatchUpThreadResponse {
    id: String,
    root_ts: String,
    title: String,
    preview: String,
    reply_count: usize,
    participant_count: usize,
    reaction_count: usize,
    file_count: usize,
    last_activity_ts: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CatchUpWindow {
    Day,
    Week,
}

impl CatchUpWindow {
    fn parse(value: Option<&str>) -> Option<Self> {
        match value.unwrap_or("24h") {
            "24h" => Some(Self::Day),
            "7d" => Some(Self::Week),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Day => "24h",
            Self::Week => "7d",
        }
    }

    const fn cutoff(self, now: i64) -> i64 {
        match self {
            Self::Day => now - DAY_SECONDS,
            Self::Week => now - (7 * DAY_SECONDS),
        }
    }
}

#[derive(Debug, Clone)]
struct ThreadAggregate {
    channel_id: String,
    root_ts: String,
    title: String,
    preview: String,
    reply_count: usize,
    participants: HashSet<String>,
    reaction_count: usize,
    file_count: usize,
    last_activity_ts: String,
    last_activity_seconds: i64,
}

pub(crate) async fn catch_up(
    State(state): State<AppState>,
    Query(query): Query<CatchUpQuery>,
) -> Result<Json<CatchUpResponse>, (StatusCode, Json<ErrorResponse>)> {
    let window = CatchUpWindow::parse(query.window.as_deref()).ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_window",
        }),
    ))?;
    let channels = build_catch_up(
        state.store.channels().await,
        state.store.messages().await,
        state.store.reactions().await,
        state.store.files().await,
        window,
        current_unix_timestamp(),
    );

    Ok(Json(CatchUpResponse {
        window: window.as_str(),
        channels,
    }))
}

fn build_catch_up(
    channels: Vec<Channel>,
    messages: Vec<Message>,
    reactions: Vec<Reaction>,
    files: Vec<File>,
    window: CatchUpWindow,
    now: i64,
) -> Vec<CatchUpChannelResponse> {
    let cutoff = window.cutoff(now);
    let channel_metadata = channels
        .into_iter()
        .map(|channel| (channel.id.clone(), channel))
        .collect::<HashMap<_, _>>();
    let message_lookup = messages
        .iter()
        .map(|message| (message.ts.clone(), message))
        .collect::<HashMap<_, _>>();
    let mut threads = HashMap::<(String, String), ThreadAggregate>::new();

    for message in &messages {
        let activity_seconds = parse_ts_seconds(&message.ts);
        if activity_seconds < cutoff {
            continue;
        }

        let root_ts = message
            .thread_ts
            .clone()
            .unwrap_or_else(|| message.ts.clone());
        let entry = threads
            .entry((message.channel_id.clone(), root_ts.clone()))
            .or_insert_with(|| ThreadAggregate {
                channel_id: message.channel_id.clone(),
                root_ts: root_ts.clone(),
                title: summarize_text(&message.text),
                preview: summarize_text(&message.text),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: message.ts.clone(),
                last_activity_seconds: activity_seconds,
            });

        if message.ts == root_ts {
            entry.title = summarize_text(&message.text);
            entry.preview = summarize_text(&message.text);
        } else {
            entry.reply_count += 1;
        }
        if let Some(user_id) = &message.user_id {
            entry.participants.insert(user_id.clone());
        }
        update_last_activity(entry, &message.ts, activity_seconds);
    }

    for reaction in reactions {
        let Some(message) = message_lookup.get(&reaction.message_ts) else {
            continue;
        };
        let activity_seconds = parse_ts_seconds(&reaction.message_ts);
        if activity_seconds < cutoff {
            continue;
        }

        let root_ts = message
            .thread_ts
            .clone()
            .unwrap_or_else(|| message.ts.clone());
        let entry = threads
            .entry((reaction.channel_id.clone(), root_ts.clone()))
            .or_insert_with(|| ThreadAggregate {
                channel_id: reaction.channel_id.clone(),
                root_ts,
                title: summarize_text(&message.text),
                preview: summarize_text(&message.text),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: reaction.message_ts.clone(),
                last_activity_seconds: activity_seconds,
            });
        entry.reaction_count += 1;
        entry.participants.insert(reaction.user_id);
        update_last_activity(entry, &reaction.message_ts, activity_seconds);
    }

    for file in files {
        let Some(message) = message_lookup.get(&file.message_ts) else {
            continue;
        };
        let activity_seconds = parse_ts_seconds(&file.message_ts);
        if activity_seconds < cutoff {
            continue;
        }

        let root_ts = message
            .thread_ts
            .clone()
            .unwrap_or_else(|| message.ts.clone());
        let entry = threads
            .entry((file.channel_id.clone(), root_ts.clone()))
            .or_insert_with(|| ThreadAggregate {
                channel_id: file.channel_id.clone(),
                root_ts,
                title: summarize_text(&message.text),
                preview: summarize_text(&message.text),
                reply_count: 0,
                participants: HashSet::new(),
                reaction_count: 0,
                file_count: 0,
                last_activity_ts: file.message_ts.clone(),
                last_activity_seconds: activity_seconds,
            });
        entry.file_count += 1;
        update_last_activity(entry, &file.message_ts, activity_seconds);
    }

    let mut grouped = HashMap::<String, Vec<ThreadAggregate>>::new();
    for thread in threads.into_values() {
        grouped
            .entry(thread.channel_id.clone())
            .or_default()
            .push(thread);
    }

    let mut channels = grouped
        .into_iter()
        .map(|(channel_id, mut threads)| {
            threads.sort_by(|left, right| {
                (
                    right.last_activity_seconds,
                    right.reply_count,
                    right.reaction_count,
                    right.file_count,
                    right.root_ts.as_str(),
                )
                    .cmp(&(
                        left.last_activity_seconds,
                        left.reply_count,
                        left.reaction_count,
                        left.file_count,
                        left.root_ts.as_str(),
                    ))
            });
            let metadata = channel_metadata.get(&channel_id);

            (
                threads
                    .first()
                    .map(|thread| thread.last_activity_seconds)
                    .unwrap_or_default(),
                CatchUpChannelResponse {
                    id: channel_id.clone(),
                    name: metadata.and_then(|channel| channel.name.clone()),
                    kind: metadata
                        .map(|channel| channel.kind.as_str())
                        .unwrap_or_else(|| ChannelKind::from_channel_id(&channel_id).as_str()),
                    is_archived: metadata.is_some_and(|channel| channel.is_archived),
                    thread_count: threads.len(),
                    threads: threads
                        .into_iter()
                        .map(|thread| CatchUpThreadResponse {
                            id: format!("{}:{}", thread.channel_id, thread.root_ts),
                            root_ts: thread.root_ts,
                            title: thread.title,
                            preview: thread.preview,
                            reply_count: thread.reply_count,
                            participant_count: thread.participants.len(),
                            reaction_count: thread.reaction_count,
                            file_count: thread.file_count,
                            last_activity_ts: thread.last_activity_ts,
                        })
                        .collect(),
                },
            )
        })
        .collect::<Vec<_>>();
    channels.sort_by(|left, right| {
        (
            right.0,
            left.1.is_archived,
            left.1.name.as_deref(),
            left.1.id.as_str(),
        )
            .cmp(&(
                left.0,
                right.1.is_archived,
                right.1.name.as_deref(),
                right.1.id.as_str(),
            ))
    });

    channels.into_iter().map(|(_, channel)| channel).collect()
}

fn update_last_activity(thread: &mut ThreadAggregate, ts: &str, seconds: i64) {
    if seconds >= thread.last_activity_seconds {
        thread.last_activity_seconds = seconds;
        thread.last_activity_ts = ts.to_owned();
    }
}

fn summarize_text(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "(no text)".to_owned();
    }
    normalized.chars().take(80).collect()
}

fn parse_ts_seconds(value: &str) -> i64 {
    value
        .split('.')
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or_default()
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::{CatchUpWindow, DAY_SECONDS, build_catch_up};
    use crate::{
        ApiConfig,
        auth::{SessionClaims, build_session_token, current_unix_timestamp},
        build_router,
    };
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use db::JsonlEventStore;
    use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    #[test]
    fn build_catch_up_filters_to_the_requested_window() {
        let now = 1_700_000_000;
        let channels = vec![];
        let messages = vec![
            domain::Message {
                team_id: "T123".to_owned(),
                channel_id: "C123".to_owned(),
                ts: format!("{}.000001", now - 60),
                thread_ts: None,
                user_id: Some("U123".to_owned()),
                text: "recent".to_owned(),
            },
            domain::Message {
                team_id: "T123".to_owned(),
                channel_id: "C123".to_owned(),
                ts: format!("{}.000001", now - (8 * DAY_SECONDS)),
                thread_ts: None,
                user_id: Some("U123".to_owned()),
                text: "too old".to_owned(),
            },
        ];

        let catch_up = build_catch_up(channels, messages, vec![], vec![], CatchUpWindow::Week, now);

        assert_eq!(catch_up.len(), 1);
        assert_eq!(catch_up[0].threads.len(), 1);
        assert_eq!(catch_up[0].threads[0].preview, "recent");
    }

    #[tokio::test]
    async fn catch_up_route_requires_authenticated_session() {
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
                .uri("/api/catch-up")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn catch_up_route_groups_threads_by_channel_and_sorts_by_activity() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&path).await.expect("store");
        let now = current_unix_timestamp();
        let root_recent_ts = format!("{}.000001", now - 120);
        let reply_recent_ts = format!("{}.000002", now - 60);
        let root_other_ts = format!("{}.000001", now - 600);
        let old_root_ts = format!("{}.000001", now - (8 * DAY_SECONDS));

        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_channel".to_owned(),
                team_id: "T123".to_owned(),
                event_time: now - 1_000,
                received_at: now - 1_000,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ChannelUpdated {
                    name: Some("general".to_owned()),
                    is_archived: Some(false),
                },
            })
            .await
            .expect("channel update");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_root_recent".to_owned(),
                team_id: "T123".to_owned(),
                event_time: now - 120,
                received_at: now - 120,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U123".to_owned()),
                    text: Some("Recent root".to_owned()),
                    ts: root_recent_ts.clone(),
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("root recent");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_reply_recent".to_owned(),
                team_id: "T123".to_owned(),
                event_time: now - 60,
                received_at: now - 60,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U456".to_owned()),
                    text: Some("Latest reply".to_owned()),
                    ts: reply_recent_ts.clone(),
                    thread_ts: Some(root_recent_ts.clone()),
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
            .expect("reply recent");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_reaction_recent".to_owned(),
                team_id: "T123".to_owned(),
                event_time: now - 59,
                received_at: now - 59,
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::ReactionAdded {
                    user_id: "U789".to_owned(),
                    reaction: "eyes".to_owned(),
                    item_ts: reply_recent_ts.clone(),
                },
            })
            .await
            .expect("reaction recent");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_root_other".to_owned(),
                team_id: "T123".to_owned(),
                event_time: now - 600,
                received_at: now - 600,
                channel_id: "C999".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U111".to_owned()),
                    text: Some("Other channel root".to_owned()),
                    ts: root_other_ts.clone(),
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("other root");
        store
            .record_process_event(&ProcessEventJob {
                event_id: "evt_old_root".to_owned(),
                team_id: "T123".to_owned(),
                event_time: now - (8 * DAY_SECONDS),
                received_at: now - (8 * DAY_SECONDS),
                channel_id: "C123".to_owned(),
                channel_kind: ChannelKind::Public,
                payload: EventPayload::Message {
                    user_id: Some("U222".to_owned()),
                    text: Some("Old root".to_owned()),
                    ts: old_root_ts,
                    thread_ts: None,
                    files: vec![],
                },
            })
            .await
            .expect("old root");

        let session_token = build_session_token(
            "session_secret",
            &SessionClaims {
                slack_user_id: "U123".to_owned(),
                team_id: "T123".to_owned(),
                email: None,
                display_name: Some("Thomas".to_owned()),
                avatar_url: None,
                exp: now + 60,
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
                .uri("/api/catch-up?window=24h")
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

        assert_eq!(payload["window"], "24h");
        assert_eq!(payload["channels"].as_array().map(Vec::len), Some(2));
        assert_eq!(payload["channels"][0]["id"], "C123");
        assert_eq!(payload["channels"][0]["thread_count"], 1);
        assert_eq!(payload["channels"][0]["threads"][0]["reply_count"], 1);
        assert_eq!(payload["channels"][0]["threads"][0]["participant_count"], 3);
        assert_eq!(payload["channels"][0]["threads"][0]["reaction_count"], 1);
        assert_eq!(payload["channels"][0]["threads"][0]["file_count"], 1);
        assert_eq!(
            payload["channels"][0]["threads"][0]["preview"],
            "Recent root"
        );
        assert_eq!(payload["channels"][1]["id"], "C999");
    }

    #[tokio::test]
    async fn catch_up_route_rejects_invalid_windows() {
        let tempdir = tempdir().expect("tempdir");
        let session_token = build_session_token(
            "session_secret",
            &SessionClaims {
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
                .uri("/api/catch-up?window=30d")
                .header("cookie", format!("archivist_session={session_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
