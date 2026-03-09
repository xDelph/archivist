use super::{AppState, ErrorResponse, store_failed};
use axum::{Json, extract::State, http::StatusCode};
use db::StoreOutcome;
use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub(crate) struct BackfillChannelRequest {
    team_id: String,
    channel_id: String,
    cursor: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct BackfillChannelResponse {
    ok: bool,
    inserted: usize,
    duplicate: usize,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackHistoryResponse {
    ok: bool,
    messages: Option<Vec<SlackHistoryMessage>>,
    response_metadata: Option<SlackResponseMetadata>,
}

#[derive(Debug, Deserialize)]
struct SlackResponseMetadata {
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackHistoryMessage {
    ts: String,
    user: Option<String>,
    text: Option<String>,
    thread_ts: Option<String>,
    #[serde(default)]
    files: Vec<SlackHistoryFile>,
}

#[derive(Debug, Deserialize)]
struct SlackHistoryFile {
    id: String,
    name: String,
    mimetype: Option<String>,
    permalink: Option<String>,
    size: Option<u64>,
}

pub(crate) async fn backfill_channel(
    State(state): State<AppState>,
    Json(request): Json<BackfillChannelRequest>,
) -> Result<Json<BackfillChannelResponse>, (StatusCode, Json<ErrorResponse>)> {
    let slack_bot_token = state.slack_bot_token.as_deref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "missing_slack_bot_token",
        }),
    ))?;
    let history = fetch_channel_history(
        &state.slack_api_base_url,
        slack_bot_token,
        &request.channel_id,
        request.cursor.as_deref(),
    )
    .await?;
    let mut inserted = 0usize;
    let mut duplicate = 0usize;

    for message in history.messages.unwrap_or_default() {
        let outcome = state
            .store
            .record_process_event(&ProcessEventJob {
                event_id: format!("backfill:{}:{}", request.channel_id, message.ts),
                team_id: request.team_id.clone(),
                event_time: parse_event_time(&message.ts),
                received_at: current_unix_timestamp(),
                channel_id: request.channel_id.clone(),
                channel_kind: ChannelKind::from_channel_id(&request.channel_id),
                payload: EventPayload::Message {
                    user_id: message.user,
                    text: message.text,
                    ts: message.ts.clone(),
                    thread_ts: message
                        .thread_ts
                        .filter(|thread_ts| thread_ts != &message.ts),
                    files: message
                        .files
                        .into_iter()
                        .map(|file| SharedFile {
                            id: file.id,
                            name: file.name,
                            mimetype: file.mimetype,
                            permalink: file.permalink,
                            size: file.size,
                        })
                        .collect(),
                },
            })
            .await
            .map_err(store_failed)?;

        match outcome {
            StoreOutcome::Inserted => inserted += 1,
            StoreOutcome::Duplicate => duplicate += 1,
        }
    }

    Ok(Json(BackfillChannelResponse {
        ok: true,
        inserted,
        duplicate,
        next_cursor: history
            .response_metadata
            .and_then(|metadata| metadata.next_cursor)
            .filter(|cursor| !cursor.trim().is_empty()),
    }))
}

async fn fetch_channel_history(
    slack_api_base_url: &str,
    slack_bot_token: &str,
    channel_id: &str,
    cursor: Option<&str>,
) -> Result<SlackHistoryResponse, (StatusCode, Json<ErrorResponse>)> {
    let endpoint = format!(
        "{}/conversations.history",
        slack_api_base_url.trim_end_matches('/')
    );
    let response = reqwest::Client::new()
        .get(&endpoint)
        .bearer_auth(slack_bot_token)
        .query(&[
            ("channel", channel_id),
            ("cursor", cursor.unwrap_or_default()),
        ])
        .send()
        .await
        .map_err(|_| slack_history_failed())?;
    if !response.status().is_success() {
        return Err(slack_history_failed());
    }

    let history: SlackHistoryResponse =
        response.json().await.map_err(|_| slack_history_failed())?;
    if !history.ok {
        return Err(slack_history_failed());
    }

    Ok(history)
}

fn parse_event_time(ts: &str) -> i64 {
    ts.split('.')
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

fn slack_history_failed() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorResponse {
            error: "slack_history_failed",
        }),
    )
}

#[cfg(test)]
mod tests {
    use crate::{WorkerConfig, build_router};
    use axum::{
        Json, Router,
        body::{Body, Bytes, to_bytes},
        http::{Request, StatusCode},
        routing::get,
    };
    use db::JsonlEventStore;
    use serde_json::json;
    use tempfile::tempdir;
    use tokio::{net::TcpListener, task::JoinHandle};
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn backfill_channel_rejects_missing_bot_token() {
        let tempdir = tempdir().expect("tempdir");
        let log_path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&log_path).await.expect("store");
        let router = build_router(
            store,
            WorkerConfig {
                host: "127.0.0.1".to_owned(),
                port: 4002,
                event_log_path: log_path.display().to_string(),
                worker_base_url: "http://127.0.0.1:4002".to_owned(),
                slack_api_base_url: "https://slack.com/api".to_owned(),
                slack_bot_token: None,
                r2_account_id: None,
                r2_access_key_id: None,
                r2_secret_access_key: None,
                r2_bucket: None,
                r2_public_url: None,
                r2_endpoint_url: None,
                current_signing_key: None,
                next_signing_key: None,
            },
        )
        .expect("router");

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/jobs/backfill_channel")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "team_id": "T123", "channel_id": "C123" }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn backfill_channel_fetches_history_and_upserts_messages() {
        let tempdir = tempdir().expect("tempdir");
        let log_path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&log_path).await.expect("store");
        let (slack_api_base_url, handle) = spawn_history_server(Json(json!({
            "ok": true,
            "messages": [
                {
                    "ts": "1700000000.000001",
                    "user": "U123",
                    "text": "hello from backfill",
                    "files": [
                        {
                            "id": "F123",
                            "name": "brief.pdf",
                            "mimetype": "application/pdf",
                            "permalink": "https://files.example.com/brief.pdf",
                            "size": 42
                        }
                    ]
                },
                {
                    "ts": "1700000000.000002",
                    "user": "U456",
                    "text": "reply from backfill",
                    "thread_ts": "1700000000.000001"
                }
            ],
            "response_metadata": {
                "next_cursor": "cursor_2"
            }
        })))
        .await;
        let router = build_router(
            store.clone(),
            WorkerConfig {
                host: "127.0.0.1".to_owned(),
                port: 4002,
                event_log_path: log_path.display().to_string(),
                worker_base_url: "http://127.0.0.1:4002".to_owned(),
                slack_api_base_url,
                slack_bot_token: Some("xoxb-test".to_owned()),
                r2_account_id: None,
                r2_access_key_id: None,
                r2_secret_access_key: None,
                r2_bucket: None,
                r2_public_url: None,
                r2_endpoint_url: None,
                current_signing_key: None,
                next_signing_key: None,
            },
        )
        .expect("router");
        let body = json!({ "team_id": "T123", "channel_id": "C123" }).to_string();

        let first = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/jobs/backfill_channel")
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .expect("request"),
            )
            .await
            .expect("first response");
        let second = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/jobs/backfill_channel")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("second response");

        handle.abort();

        assert_eq!(first.status(), StatusCode::OK);
        let first_body = to_bytes(first.into_body(), usize::MAX)
            .await
            .expect("first body");
        let first_payload: serde_json::Value = serde_json::from_slice(&first_body).expect("json");
        assert_eq!(first_payload["inserted"], 2);
        assert_eq!(first_payload["duplicate"], 0);
        assert_eq!(first_payload["next_cursor"], "cursor_2");

        let second_body = to_bytes(second.into_body(), usize::MAX)
            .await
            .expect("second body");
        let second_payload: serde_json::Value = serde_json::from_slice(&second_body).expect("json");
        assert_eq!(second_payload["inserted"], 0);
        assert_eq!(second_payload["duplicate"], 2);

        let messages = store.messages().await;
        let files = store.files().await;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "hello from backfill");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "brief.pdf");
    }

    #[tokio::test]
    async fn backfill_channel_returns_bad_gateway_when_slack_history_fails() {
        let tempdir = tempdir().expect("tempdir");
        let log_path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&log_path).await.expect("store");
        let (slack_api_base_url, handle) = spawn_history_server(Json(json!({ "ok": false }))).await;
        let router = build_router(
            store,
            WorkerConfig {
                host: "127.0.0.1".to_owned(),
                port: 4002,
                event_log_path: log_path.display().to_string(),
                worker_base_url: "http://127.0.0.1:4002".to_owned(),
                slack_api_base_url,
                slack_bot_token: Some("xoxb-test".to_owned()),
                r2_account_id: None,
                r2_access_key_id: None,
                r2_secret_access_key: None,
                r2_bucket: None,
                r2_public_url: None,
                r2_endpoint_url: None,
                current_signing_key: None,
                next_signing_key: None,
            },
        )
        .expect("router");

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/jobs/backfill_channel")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "team_id": "T123", "channel_id": "C123" }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        handle.abort();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    async fn spawn_history_server(response: Json<serde_json::Value>) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let address = listener.local_addr().expect("local addr");
        let app = Router::new().route(
            "/conversations.history",
            get(move |_body: Bytes| {
                let response = response.clone();
                async move { response }
            }),
        );
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("server");
        });

        (format!("http://{address}"), handle)
    }
}
