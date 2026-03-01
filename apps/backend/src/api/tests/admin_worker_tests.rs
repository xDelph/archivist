use base64::Engine;
use bytes::Bytes;
use chrono::Utc;
use hmac::{Hmac, Mac};
use http::StatusCode;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::api::admin_worker::process;

const ADMIN_TOKEN: &str = "secret123";
const WORKER_TOKEN: &str = "worker_token_456";
const CURRENT_SIGNING_KEY: &str = "current_signing_key_123";
const NEXT_SIGNING_KEY: &str = "next_signing_key_456";

#[tokio::test]
async fn test_worker_missing_token_returns_401() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/sync/run")
        .body(Bytes::new())
        .unwrap();
    let resp = process(
        ADMIN_TOKEN,
        Some(WORKER_TOKEN),
        Some(CURRENT_SIGNING_KEY),
        Some(NEXT_SIGNING_KEY),
        req,
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_worker_admin_token_returns_202() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/sync/run")
        .header("authorization", "Bearer secret123")
        .body(Bytes::new())
        .unwrap();
    let resp = process(
        ADMIN_TOKEN,
        Some(WORKER_TOKEN),
        Some(CURRENT_SIGNING_KEY),
        Some(NEXT_SIGNING_KEY),
        req,
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_worker_token_returns_202() {
    let body = Bytes::new();
    let signature = sign_qstash_request(CURRENT_SIGNING_KEY, &body);
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/sync/run")
        .header("authorization", "Bearer worker_token_456")
        .header("upstash-signature", signature)
        .body(body)
        .unwrap();
    let resp = process(
        ADMIN_TOKEN,
        Some(WORKER_TOKEN),
        Some(CURRENT_SIGNING_KEY),
        Some(NEXT_SIGNING_KEY),
        req,
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_worker_token_with_padded_body_hash_signature_returns_202() {
    let body = Bytes::from_static(br#"{}"#);
    let signature = sign_qstash_request_with_padded_body_hash(CURRENT_SIGNING_KEY, &body);
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/sync/run")
        .header("authorization", "Bearer worker_token_456")
        .header("upstash-signature", signature)
        .body(body)
        .unwrap();
    let resp = process(
        ADMIN_TOKEN,
        Some(WORKER_TOKEN),
        Some(CURRENT_SIGNING_KEY),
        Some(NEXT_SIGNING_KEY),
        req,
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_worker_token_invalid_signature_returns_401() {
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/admin/sync/run")
        .header("authorization", "Bearer worker_token_456")
        .header("upstash-signature", "invalid.token.value")
        .body(Bytes::new())
        .unwrap();
    let resp = process(
        ADMIN_TOKEN,
        Some(WORKER_TOKEN),
        Some(CURRENT_SIGNING_KEY),
        Some(NEXT_SIGNING_KEY),
        req,
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

fn sign_qstash_request(signing_key: &str, body: &[u8]) -> String {
    let header = json!({
        "alg": "HS256",
        "typ": "JWT",
    });
    let now = Utc::now().timestamp();
    let body_hash = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(body));
    let claims = json!({
        "iss": "Upstash",
        "nbf": now - 5,
        "exp": now + 300,
        "body": body_hash,
    });
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&header).unwrap());
    let claims_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&claims).unwrap());
    let signing_input = format!("{header_b64}.{claims_b64}");

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(signing_key.as_bytes()).unwrap();
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);
    format!("{signing_input}.{signature_b64}")
}

fn sign_qstash_request_with_padded_body_hash(signing_key: &str, body: &[u8]) -> String {
    let header = json!({
        "alg": "HS256",
        "typ": "JWT",
    });
    let now = Utc::now().timestamp();
    let body_hash = base64::engine::general_purpose::URL_SAFE.encode(Sha256::digest(body));
    let claims = json!({
        "iss": "Upstash",
        "nbf": now - 5,
        "exp": now + 300,
        "body": body_hash,
    });
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&header).unwrap());
    let claims_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&claims).unwrap());
    let signing_input = format!("{header_b64}.{claims_b64}");

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(signing_key.as_bytes()).unwrap();
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);
    format!("{signing_input}.{signature_b64}")
}
