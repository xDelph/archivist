use bytes::Bytes;
use http::StatusCode;

use crate::api::admin::process;
use crate::db::InMemoryRepository;
use crate::slack::backfill::{Channel, SlackApi, SlackError, SlackMessage, SlackUser};

const TOKEN: &str = "secret123";

/// No-op Slack client — returns empty data, used for auth-only tests.
struct NoOpSlack;

impl SlackApi for NoOpSlack {
    async fn conversations_list(
        &self,
        _cursor: Option<&str>,
    ) -> Result<(Vec<Channel>, Option<String>), SlackError> {
        Ok((vec![], None))
    }
    async fn conversations_history(
        &self,
        _channel_id: &str,
        _oldest: Option<&str>,
        _cursor: Option<&str>,
    ) -> Result<(Vec<SlackMessage>, Option<String>), SlackError> {
        Ok((vec![], None))
    }
    async fn conversations_replies(
        &self,
        _channel_id: &str,
        _ts: &str,
        _cursor: Option<&str>,
    ) -> Result<(Vec<SlackMessage>, Option<String>), SlackError> {
        Ok((vec![], None))
    }

    async fn users_list(
        &self,
        _cursor: Option<&str>,
    ) -> Result<(Vec<SlackUser>, Option<String>), SlackError> {
        Ok((vec![], None))
    }
}

#[tokio::test]
async fn test_missing_token_returns_401() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/backfill")
        .body(Bytes::new())
        .unwrap();
    let resp = process(
        TOKEN,
        "",
        req,
        &InMemoryRepository::default(),
        &NoOpSlack,
        None,
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_wrong_token_returns_401() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/backfill")
        .header("authorization", "Bearer wrong_token")
        .body(Bytes::new())
        .unwrap();
    let resp = process(
        TOKEN,
        "",
        req,
        &InMemoryRepository::default(),
        &NoOpSlack,
        None,
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_valid_token_triggers_backfill_and_returns_200() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/backfill")
        .header("authorization", "Bearer secret123")
        .body(Bytes::new())
        .unwrap();
    let resp = process(
        TOKEN,
        "",
        req,
        &InMemoryRepository::default(),
        &NoOpSlack,
        None,
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
