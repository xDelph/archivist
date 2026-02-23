use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::slack::backfill::{SlackApi, SlackClient, SlackError};

// ── conversations_history ─────────────────────────────────────────────────────

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

// ── rate limiting ─────────────────────────────────────────────────────────────

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

// ── conversations_list ────────────────────────────────────────────────────────

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

// ── conversations_replies ─────────────────────────────────────────────────────

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
