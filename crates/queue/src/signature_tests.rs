use super::{SignatureError, verify_qstash_signature};
use base64::Engine;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::{Digest, Sha256};

const CURRENT_SIGNING_KEY: &str = "current_signing_key";
const NEXT_SIGNING_KEY: &str = "next_signing_key";
const URL: &str = "https://archivist.example.com/jobs/process_event";

#[test]
fn current_signing_key_is_accepted() {
    let body = br#"{"ok":true}"#;
    let signature = sign_qstash_request(CURRENT_SIGNING_KEY, body, URL, false, false);

    let result = verify_qstash_signature(
        Some(&signature),
        body,
        URL,
        Some(CURRENT_SIGNING_KEY),
        Some(NEXT_SIGNING_KEY),
    );

    assert_eq!(result, Ok(()));
}

#[test]
fn next_signing_key_is_accepted() {
    let body = br#"{"ok":true}"#;
    let signature = sign_qstash_request(NEXT_SIGNING_KEY, body, URL, false, false);

    let result = verify_qstash_signature(
        Some(&signature),
        body,
        URL,
        Some(CURRENT_SIGNING_KEY),
        Some(NEXT_SIGNING_KEY),
    );

    assert_eq!(result, Ok(()));
}

#[test]
fn padded_body_hash_is_accepted() {
    let body = br#"{"ok":true}"#;
    let signature = sign_qstash_request(CURRENT_SIGNING_KEY, body, URL, true, false);

    let result =
        verify_qstash_signature(Some(&signature), body, URL, Some(CURRENT_SIGNING_KEY), None);

    assert_eq!(result, Ok(()));
}

#[test]
fn bearer_prefix_is_accepted() {
    let body = br#"{"ok":true}"#;
    let signature = sign_qstash_request(CURRENT_SIGNING_KEY, body, URL, false, false);

    let bearer_signature = format!("Bearer {signature}");
    let result = verify_qstash_signature(
        Some(&bearer_signature),
        body,
        URL,
        Some(CURRENT_SIGNING_KEY),
        None,
    );

    assert_eq!(result, Ok(()));
}

#[test]
fn wrong_subject_is_rejected() {
    let body = br#"{"ok":true}"#;
    let signature = sign_qstash_request(
        CURRENT_SIGNING_KEY,
        body,
        "https://archivist.example.com/jobs/heartbeat",
        false,
        false,
    );

    let result =
        verify_qstash_signature(Some(&signature), body, URL, Some(CURRENT_SIGNING_KEY), None);

    assert_eq!(result, Err(SignatureError::UnexpectedSubject));
}

#[test]
fn expired_signature_is_rejected() {
    let body = br#"{"ok":true}"#;
    let signature = sign_qstash_request(CURRENT_SIGNING_KEY, body, URL, false, true);

    let result =
        verify_qstash_signature(Some(&signature), body, URL, Some(CURRENT_SIGNING_KEY), None);

    assert_eq!(result, Err(SignatureError::Expired));
}

fn sign_qstash_request(
    signing_key: &str,
    body: &[u8],
    url: &str,
    padded_body_hash: bool,
    expired: bool,
) -> String {
    let now = current_unix_timestamp();
    let header = json!({
        "alg": "HS256",
        "typ": "JWT",
    });
    let body_hash = if padded_body_hash {
        base64::engine::general_purpose::URL_SAFE.encode(Sha256::digest(body))
    } else {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(body))
    };
    let claims = json!({
        "iss": "Upstash",
        "sub": url,
        "nbf": now - 5,
        "exp": if expired { now - 1 } else { now + 300 },
        "body": body_hash,
    });
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&header).expect("header json"));
    let claims_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&claims).expect("claims json"));
    let signing_input = format!("{header_b64}.{claims_b64}");

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(signing_key.as_bytes()).expect("signing key");
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);

    format!("{signing_input}.{signature_b64}")
}

fn current_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}
