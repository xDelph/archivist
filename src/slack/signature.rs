use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Maximum age (seconds) of a request before it is rejected as a replay.
const MAX_AGE_SECS: i64 = 300;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SignatureError {
    #[error("timestamp is too old or too new (replay protection)")]
    Expired,
    #[error("signature format is invalid")]
    InvalidFormat,
    #[error("signature does not match")]
    InvalidSignature,
}

/// Verifies a Slack request signature.
///
/// # Arguments
/// * `signing_secret` — value of `SLACK_SIGNING_SECRET`
/// * `timestamp`      — value of `X-Slack-Request-Timestamp` header
/// * `raw_body`       — raw request body bytes (before any parsing)
/// * `signature`      — value of `X-Slack-Signature` header (`v0=<hex>`)
pub fn verify_signature(
    signing_secret: &str,
    timestamp: &str,
    raw_body: &[u8],
    signature: &str,
) -> Result<(), SignatureError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    verify_signature_at(signing_secret, timestamp, raw_body, signature, now)
}

/// Same as [`verify_signature`] but accepts an explicit `now_secs` for testing.
pub(crate) fn verify_signature_at(
    signing_secret: &str,
    timestamp: &str,
    raw_body: &[u8],
    signature: &str,
    now_secs: i64,
) -> Result<(), SignatureError> {
    // 1. Parse timestamp
    let ts: i64 = timestamp
        .parse()
        .map_err(|_| SignatureError::InvalidFormat)?;

    // 2. Anti-replay: reject if |now - ts| > 300s
    if (now_secs - ts).abs() > MAX_AGE_SECS {
        return Err(SignatureError::Expired);
    }

    // 3. Strip "v0=" prefix
    let hex_sig = signature
        .strip_prefix("v0=")
        .ok_or(SignatureError::InvalidFormat)?;

    // 4. Decode hex signature bytes
    let sig_bytes = hex::decode(hex_sig).map_err(|_| SignatureError::InvalidFormat)?;

    // 5. Compute expected HMAC-SHA256 over "v0:{timestamp}:{raw_body}"
    let mut mac =
        HmacSha256::new_from_slice(signing_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(format!("v0:{}:", timestamp).as_bytes());
    mac.update(raw_body);

    // 6. Constant-time comparison
    mac.verify_slice(&sig_bytes)
        .map_err(|_| SignatureError::InvalidSignature)
}

/// Computes a valid Slack signature for the given inputs. Used in tests.
#[cfg(test)]
pub(crate) fn compute_signature(signing_secret: &str, timestamp: i64, raw_body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(signing_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(format!("v0:{}:", timestamp).as_bytes());
    mac.update(raw_body);
    format!("v0={}", hex::encode(mac.finalize().into_bytes()))
}
