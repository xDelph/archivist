use crate::slack::signature::{SignatureError, compute_signature, verify_signature_at};

const SECRET: &str = "test_signing_secret";
const BODY: &[u8] = b"payload=test_body";

/// Helper: returns a (timestamp, signature) pair that is valid at `now`.
fn valid_pair(now: i64) -> (String, String) {
    let ts = now.to_string();
    let sig = compute_signature(SECRET, now, BODY);
    (ts, sig)
}

#[test]
fn test_valid_signature_passes() {
    let now = 1_700_000_000_i64;
    let (ts, sig) = valid_pair(now);
    assert!(verify_signature_at(SECRET, &ts, BODY, &sig, now).is_ok());
}

#[test]
fn test_invalid_signature_rejected() {
    let now = 1_700_000_000_i64;
    let (ts, _) = valid_pair(now);
    let bad_sig = "v0=deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    assert_eq!(
        verify_signature_at(SECRET, &ts, BODY, bad_sig, now),
        Err(SignatureError::InvalidSignature)
    );
}

#[test]
fn test_tampered_body_rejected() {
    let now = 1_700_000_000_i64;
    let (ts, sig) = valid_pair(now);
    let tampered = b"payload=tampered";
    assert_eq!(
        verify_signature_at(SECRET, &ts, tampered, &sig, now),
        Err(SignatureError::InvalidSignature)
    );
}

#[test]
fn test_expired_timestamp_rejected() {
    let now = 1_700_000_000_i64;
    let old_ts = now - 301;
    let sig = compute_signature(SECRET, old_ts, BODY);
    assert_eq!(
        verify_signature_at(SECRET, &old_ts.to_string(), BODY, &sig, now),
        Err(SignatureError::Expired)
    );
}

#[test]
fn test_future_timestamp_rejected() {
    let now = 1_700_000_000_i64;
    let future_ts = now + 301;
    let sig = compute_signature(SECRET, future_ts, BODY);
    assert_eq!(
        verify_signature_at(SECRET, &future_ts.to_string(), BODY, &sig, now),
        Err(SignatureError::Expired)
    );
}

#[test]
fn test_boundary_timestamp_accepted() {
    let now = 1_700_000_000_i64;
    // Exactly at the boundary (300s) must pass
    let ts = now - 300;
    let sig = compute_signature(SECRET, ts, BODY);
    assert!(verify_signature_at(SECRET, &ts.to_string(), BODY, &sig, now).is_ok());
}

#[test]
fn test_missing_v0_prefix_rejected() {
    let now = 1_700_000_000_i64;
    let (ts, sig) = valid_pair(now);
    let no_prefix = sig.trim_start_matches("v0=");
    assert_eq!(
        verify_signature_at(SECRET, &ts, BODY, no_prefix, now),
        Err(SignatureError::InvalidFormat)
    );
}

#[test]
fn test_invalid_hex_rejected() {
    let now = 1_700_000_000_i64;
    let (ts, _) = valid_pair(now);
    assert_eq!(
        verify_signature_at(SECRET, &ts, BODY, "v0=zzzzzz", now),
        Err(SignatureError::InvalidFormat)
    );
}

#[test]
fn test_non_numeric_timestamp_rejected() {
    assert_eq!(
        verify_signature_at(SECRET, "not-a-number", BODY, "v0=aabb", 1_700_000_000),
        Err(SignatureError::InvalidFormat)
    );
}
