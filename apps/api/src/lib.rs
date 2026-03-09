use axum::{Json, Router, extract::State, routing::get};
use db::{JsonlEventStore, RepositoryMode, StoreError};
use domain::{Channel, ChannelKind, File, Message, Reaction, WorkspaceMode};
use search::SearchBackend;
use serde::Serialize;
use std::collections::HashMap;
use tower_http::trace::TraceLayer;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 4000;
const DEFAULT_EVENT_LOG_PATH: &str = "logs/process-events.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub event_log_path: String,
}

impl ApiConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("ARCHIVIST_API_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_owned()),
            port: read_port("ARCHIVIST_API_PORT", DEFAULT_PORT),
            event_log_path: std::env::var("ARCHIVIST_EVENT_LOG_PATH")
                .unwrap_or_else(|_| DEFAULT_EVENT_LOG_PATH.to_owned()),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Clone)]
struct AppState {
    store: JsonlEventStore,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HealthResponse {
    service: &'static str,
    version: &'static str,
    workspace_mode: &'static str,
    repository_mode: &'static str,
    search_backend: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ChannelSummaryResponse {
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

pub async fn build_router(config: ApiConfig) -> Result<Router, StoreError> {
    let store = JsonlEventStore::open(&config.event_log_path).await?;

    Ok(Router::new()
        .route("/health", get(health))
        .route("/api/channels", get(channels))
        .with_state(AppState { store })
        .layer(TraceLayer::new_for_http()))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "api",
        version: env!("CARGO_PKG_VERSION"),
        workspace_mode: WorkspaceMode::SingleWorkspace.as_str(),
        repository_mode: RepositoryMode::LocalJsonlMock.as_str(),
        search_backend: SearchBackend::PostgresTsvectorPlaceholder.as_str(),
    })
}

async fn channels(State(state): State<AppState>) -> Json<Vec<ChannelSummaryResponse>> {
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

fn read_port(key: &str, fallback: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::{ApiConfig, build_router};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use db::JsonlEventStore;
    use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    #[test]
    fn config_uses_defaults() {
        let config = ApiConfig::from_env();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 4000);
        assert_eq!(config.event_log_path, "logs/process-events.jsonl");
        assert_eq!(config.bind_address(), "127.0.0.1:4000");
    }

    #[tokio::test]
    async fn health_route_reports_workspace_capabilities() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let response = build_router(ApiConfig {
            host: "127.0.0.1".to_owned(),
            port: 4000,
            event_log_path: path.display().to_string(),
        })
        .await
        .expect("router")
        .oneshot(
            Request::builder()
                .uri("/health")
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

        assert_eq!(payload["service"], "api");
        assert_eq!(payload["workspace_mode"], "single_workspace");
        assert_eq!(payload["repository_mode"], "local_jsonl_mock");
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

        let response = build_router(ApiConfig {
            host: "127.0.0.1".to_owned(),
            port: 4000,
            event_log_path: path.display().to_string(),
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
