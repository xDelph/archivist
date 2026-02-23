use vercel_runtime::{Body, StatusCode};

use crate::api::admin::process;

const TOKEN: &str = "secret123";

#[tokio::test]
async fn test_missing_token_returns_401() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/backfill")
        .body(Body::Empty)
        .unwrap();
    let resp = process(TOKEN, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_wrong_token_returns_401() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/backfill")
        .header("authorization", "Bearer wrong_token")
        .body(Body::Empty)
        .unwrap();
    let resp = process(TOKEN, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_valid_token_returns_200() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/backfill")
        .header("authorization", "Bearer secret123")
        .body(Body::Empty)
        .unwrap();
    let resp = process(TOKEN, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
