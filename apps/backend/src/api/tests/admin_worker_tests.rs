use bytes::Bytes;
use http::StatusCode;

use crate::api::admin_worker::process;

const ADMIN_TOKEN: &str = "secret123";
const CRON_SECRET: &str = "cron_secret_456";

#[tokio::test]
async fn test_worker_missing_token_returns_401() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/backfill/run")
        .body(Bytes::new())
        .unwrap();
    let resp = process(ADMIN_TOKEN, Some(CRON_SECRET), req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_worker_admin_token_returns_202() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/backfill/run")
        .header("authorization", "Bearer secret123")
        .body(Bytes::new())
        .unwrap();
    let resp = process(ADMIN_TOKEN, Some(CRON_SECRET), req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_worker_cron_secret_returns_202() {
    let req = http::Request::builder()
        .method("GET")
        .uri("/api/admin/backfill/run")
        .header("authorization", "Bearer cron_secret_456")
        .body(Bytes::new())
        .unwrap();
    let resp = process(ADMIN_TOKEN, Some(CRON_SECRET), req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}
