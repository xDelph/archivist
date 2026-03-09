use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

const MAX_AGE_SECS: i64 = 300;

pub const SLACK_SIGNATURE_HEADER: &str = "x-slack-signature";
pub const SLACK_TIMESTAMP_HEADER: &str = "x-slack-request-timestamp";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SignatureError {
    #[error("timestamp is too old or too new")]
    Expired,
    #[error("signature format is invalid")]
    InvalidFormat,
    #[error("signature does not match")]
    InvalidSignature,
}

pub fn verify_signature(
    signing_secret: &str,
    timestamp: &str,
    raw_body: &[u8],
    signature: &str,
) -> Result<(), SignatureError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64;

    verify_signature_at(signing_secret, timestamp, raw_body, signature, now)
}

fn verify_signature_at(
    signing_secret: &str,
    timestamp: &str,
    raw_body: &[u8],
    signature: &str,
    now_secs: i64,
) -> Result<(), SignatureError> {
    let timestamp = timestamp
        .parse::<i64>()
        .map_err(|_| SignatureError::InvalidFormat)?;

    if (now_secs - timestamp).abs() > MAX_AGE_SECS {
        return Err(SignatureError::Expired);
    }

    let supplied_signature = signature
        .strip_prefix("v0=")
        .ok_or(SignatureError::InvalidFormat)?;
    let signature_bytes =
        hex::decode(supplied_signature).map_err(|_| SignatureError::InvalidFormat)?;

    let mut mac =
        HmacSha256::new_from_slice(signing_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(format!("v0:{timestamp}:").as_bytes());
    mac.update(raw_body);
    mac.verify_slice(&signature_bytes)
        .map_err(|_| SignatureError::InvalidSignature)
}

#[cfg(test)]
#[path = "signature_tests.rs"]
mod tests;
