use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::db::{InMemoryRepository, Repository};
use crate::slack::backfill::{
    Channel, SlackApi, SlackClient, SlackError, SlackMessage, SlackUser, run_backfill,
};

// ── SlackClient HTTP tests (wiremock) ─────────────────────────────────────────

#[tokio::test]
async fn test_history_returns_cursor_when_has_more() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/conversations.history"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "messages": [{"ts": "1700000000.000100", "text": "hello"}],
            "response_metadata": {"next_cursor": "page2cursor"}
        })))
        .mount(&server)
        .await;

    let client = SlackClient::with_base_url("xoxb-test", server.uri());
    let (messages, cursor) = client
        .conversations_history("C001", None, None)
        .await
        .unwrap();

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].ts, "1700000000.000100");
    assert_eq!(cursor, Some("page2cursor".to_owned()));
}

#[tokio::test]
async fn test_history_returns_none_cursor_when_exhausted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/conversations.history"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "messages": [{"ts": "1700000000.000100", "text": "hello"}],
            "response_metadata": {"next_cursor": ""}
        })))
        .mount(&server)
        .await;

    let client = SlackClient::with_base_url("xoxb-test", server.uri());
    let (messages, cursor) = client
        .conversations_history("C001", None, None)
        .await
        .unwrap();

    assert_eq!(messages.len(), 1);
    assert_eq!(cursor, None);
}

#[tokio::test]
async fn test_empty_history_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/conversations.history"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "messages": [],
            "response_metadata": {"next_cursor": ""}
        })))
        .mount(&server)
        .await;

    let client = SlackClient::with_base_url("xoxb-test", server.uri());
    let (messages, cursor) = client
        .conversations_history("C001", None, None)
        .await
        .unwrap();

    assert!(messages.is_empty());
    assert_eq!(cursor, None);
}

#[tokio::test]
async fn test_rate_limit_429_surfaces_as_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/conversations.history"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "30"))
        .mount(&server)
        .await;

    let client = SlackClient::with_base_url("xoxb-test", server.uri());
    let result = client.conversations_history("C001", None, None).await;

    assert!(matches!(
        result,
        Err(SlackError::RateLimited { retry_after: 30 })
    ));
}

#[tokio::test]
async fn test_conversations_list_returns_channels_and_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/conversations.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "channels": [
                {"id": "C001", "name": "general", "is_private": false},
                {"id": "C002", "name": "secret", "is_private": true}
            ],
            "response_metadata": {"next_cursor": "nextpage"}
        })))
        .mount(&server)
        .await;

    let client = SlackClient::with_base_url("xoxb-test", server.uri());
    let (channels, cursor) = client.conversations_list(None).await.unwrap();

    assert_eq!(channels.len(), 2);
    assert_eq!(channels[0].id, "C001");
    assert!(channels[1].is_private);
    assert_eq!(cursor, Some("nextpage".to_owned()));
}

#[tokio::test]
async fn test_replies_includes_thread_ts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/conversations.replies"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "messages": [
                {"ts": "1700000000.000100", "thread_ts": "1700000000.000100", "text": "parent"},
                {"ts": "1700000001.000200", "thread_ts": "1700000000.000100", "text": "reply"}
            ],
            "response_metadata": {"next_cursor": ""}
        })))
        .mount(&server)
        .await;

    let client = SlackClient::with_base_url("xoxb-test", server.uri());
    let (messages, _) = client
        .conversations_replies("C001", "1700000000.000100", None)
        .await
        .unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].thread_ts.as_deref(), Some("1700000000.000100"));
}

// ── run_backfill logic tests (in-memory mock, fully offline) ──────────────────

fn make_message(ts: &str, thread_ts: Option<&str>) -> SlackMessage {
    let mut raw = json!({"ts": ts, "text": "hi", "team": "T001", "user": "U001"});
    if let Some(tts) = thread_ts {
        raw["thread_ts"] = json!(tts);
    }
    SlackMessage {
        ts: ts.to_owned(),
        thread_ts: thread_ts.map(str::to_owned),
        raw,
    }
}

/// Minimal mock that serves preset channel/history/reply data.
type HistoryCalls = Arc<Mutex<Vec<(String, Option<String>)>>>;

struct MockSlackApi {
    channels: Vec<Channel>,
    history: HashMap<String, Vec<SlackMessage>>,
    replies: HashMap<(String, String), Vec<SlackMessage>>,
    /// Records (channel_id, oldest) for each conversations_history call.
    history_calls: HistoryCalls,
}

impl MockSlackApi {
    fn new(channels: Vec<Channel>) -> Self {
        Self {
            channels,
            history: HashMap::new(),
            replies: HashMap::new(),
            history_calls: Arc::new(Mutex::new(vec![])),
        }
    }
    fn with_history(mut self, channel_id: &str, messages: Vec<SlackMessage>) -> Self {
        self.history.insert(channel_id.to_owned(), messages);
        self
    }
    fn with_replies(mut self, channel_id: &str, ts: &str, messages: Vec<SlackMessage>) -> Self {
        self.replies
            .insert((channel_id.to_owned(), ts.to_owned()), messages);
        self
    }
}

impl SlackApi for MockSlackApi {
    async fn conversations_list(
        &self,
        _cursor: Option<&str>,
    ) -> Result<(Vec<Channel>, Option<String>), SlackError> {
        Ok((self.channels.clone(), None))
    }

    async fn conversations_history(
        &self,
        channel_id: &str,
        oldest: Option<&str>,
        _cursor: Option<&str>,
    ) -> Result<(Vec<SlackMessage>, Option<String>), SlackError> {
        self.history_calls
            .lock()
            .unwrap()
            .push((channel_id.to_owned(), oldest.map(str::to_owned)));
        let msgs = self.history.get(channel_id).cloned().unwrap_or_default();
        Ok((msgs, None))
    }

    async fn conversations_replies(
        &self,
        channel_id: &str,
        ts: &str,
        _cursor: Option<&str>,
    ) -> Result<(Vec<SlackMessage>, Option<String>), SlackError> {
        let key = (channel_id.to_owned(), ts.to_owned());
        let msgs = self.replies.get(&key).cloned().unwrap_or_default();
        Ok((msgs, None))
    }

    async fn users_list(
        &self,
        _cursor: Option<&str>,
    ) -> Result<(Vec<SlackUser>, Option<String>), SlackError> {
        Ok((vec![], None))
    }
}

#[tokio::test]
async fn test_backfill_upserts_messages_for_all_channels() {
    let mock = MockSlackApi::new(vec![
        Channel {
            id: "C001".into(),
            name: "general".into(),
            is_private: false,
            is_member: true,
        },
        Channel {
            id: "C002".into(),
            name: "random".into(),
            is_private: false,
            is_member: true,
        },
    ])
    .with_history("C001", vec![make_message("1700000001.000100", None)])
    .with_history(
        "C002",
        vec![
            make_message("1700000002.000100", None),
            make_message("1700000002.000200", None),
        ],
    );

    let repo = InMemoryRepository::default();
    run_backfill(&repo, &mock, "", None).await.unwrap();

    assert_eq!(repo.messages.lock().unwrap().len(), 3);
    assert_eq!(repo.weekly_upserts.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn test_backfill_fetches_replies_for_thread_parents() {
    let thread_ts = "1700000000.000100";
    let mock = MockSlackApi::new(vec![Channel {
        id: "C001".into(),
        name: "general".into(),
        is_private: false,
        is_member: true,
    }])
    // thread parent: thread_ts == ts
    .with_history("C001", vec![make_message(thread_ts, Some(thread_ts))])
    // two replies (parent + child)
    .with_replies(
        "C001",
        thread_ts,
        vec![
            make_message(thread_ts, Some(thread_ts)),
            make_message("1700000001.000200", Some(thread_ts)),
        ],
    );

    let repo = InMemoryRepository::default();
    run_backfill(&repo, &mock, "", None).await.unwrap();

    // parent (from history + replies, idempotent) + reply = 2 distinct (channel, ts) keys
    assert_eq!(repo.messages.lock().unwrap().len(), 2);
    // one weekly upsert per touched thread root
    assert_eq!(repo.weekly_upserts.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_backfill_applies_overlap_to_last_archived_ts() {
    let mock = MockSlackApi::new(vec![Channel {
        id: "C001".into(),
        name: "general".into(),
        is_private: false,
        is_member: true,
    }])
    .with_history("C001", vec![]);

    let repo = InMemoryRepository::default();
    // Pre-populate so get_last_archived_ts returns a ts
    let _: uuid::Uuid = repo
        .upsert_message(&crate::db::MessageRecord {
            team_id: "T001".into(),
            channel_id: "C001".into(),
            ts: "1700000005.000100".into(),
            thread_ts: None,
            user_id: None,
            text: "existing".into(),
            subtype: None,
            edited_ts: None,
            deleted: false,
            raw_json: serde_json::Value::Null,
        })
        .await
        .unwrap();

    let calls = mock.history_calls.clone();
    #[allow(unused_variables)]
    run_backfill(&repo, &mock, "", None).await.unwrap();

    let recorded = calls.lock().unwrap();
    let oldest = recorded[0]
        .1
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap();
    assert!(oldest < 1700000005.000100_f64);
}
