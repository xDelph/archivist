use base64::Engine;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub const UPSTASH_SIGNATURE_HEADER: &str = "upstash-signature";

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SignatureError {
    #[error("missing upstash-signature header")]
    MissingSignatureHeader,
    #[error("missing signing key")]
    MissingSigningKey,
    #[error("invalid qstash token")]
    InvalidToken,
    #[error("signature mismatch")]
    SignatureMismatch,
    #[error("signature is expired")]
    Expired,
    #[error("signature is not active yet")]
    NotYetActive,
    #[error("unexpected issuer")]
    UnexpectedIssuer,
    #[error("unexpected subject")]
    UnexpectedSubject,
    #[error("missing body claim")]
    MissingBodyClaim,
    #[error("body hash mismatch")]
    BodyMismatch,
}

#[derive(Debug, Deserialize)]
struct QStashClaims {
    exp: Option<i64>,
    nbf: Option<i64>,
    iss: Option<String>,
    sub: Option<String>,
    body: Option<String>,
}

pub fn verify_qstash_signature(
    signature: Option<&str>,
    body: &[u8],
    url: &str,
    current_signing_key: Option<&str>,
    next_signing_key: Option<&str>,
) -> Result<(), SignatureError> {
    let token = signature
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(SignatureError::MissingSignatureHeader)?;
    let token = token.strip_prefix("Bearer ").unwrap_or(token);

    let mut attempts = 0;
    let mut last_error = None;

    for signing_key in [current_signing_key, next_signing_key] {
        let Some(signing_key) = signing_key.map(str::trim).filter(|value| !value.is_empty()) else {
            continue;
        };

        attempts += 1;
        match verify_signature_with_key(token, signing_key, body, url) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }

    if attempts == 0 {
        return Err(SignatureError::MissingSigningKey);
    }

    Err(last_error.unwrap_or(SignatureError::SignatureMismatch))
}

fn verify_signature_with_key(
    token: &str,
    signing_key: &str,
    body: &[u8],
    url: &str,
) -> Result<(), SignatureError> {
    let mut parts = token.split('.');
    let header_b64 = parts.next().ok_or(SignatureError::InvalidToken)?;
    let claims_b64 = parts.next().ok_or(SignatureError::InvalidToken)?;
    let sig_b64 = parts.next().ok_or(SignatureError::InvalidToken)?;
    if parts.next().is_some() {
        return Err(SignatureError::InvalidToken);
    }

    let signing_input = format!("{header_b64}.{claims_b64}");
    let provided_signature = decode_base64url(sig_b64)?;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(signing_key.as_bytes())
        .map_err(|_| SignatureError::InvalidToken)?;
    mac.update(signing_input.as_bytes());
    let expected_signature = mac.finalize().into_bytes();
    if provided_signature != expected_signature.as_slice() {
        return Err(SignatureError::SignatureMismatch);
    }

    let claims_json = decode_base64url(claims_b64)?;
    let claims: QStashClaims =
        serde_json::from_slice(&claims_json).map_err(|_| SignatureError::InvalidToken)?;
    let now = current_unix_timestamp();
    let exp = claims.exp.ok_or(SignatureError::InvalidToken)?;
    if now > exp {
        return Err(SignatureError::Expired);
    }
    if let Some(nbf) = claims.nbf
        && now < nbf
    {
        return Err(SignatureError::NotYetActive);
    }
    if let Some(issuer) = claims.iss
        && issuer != "Upstash"
    {
        return Err(SignatureError::UnexpectedIssuer);
    }
    if claims.sub.as_deref() != Some(url) {
        return Err(SignatureError::UnexpectedSubject);
    }

    let provided_body_hash = claims.body.ok_or(SignatureError::MissingBodyClaim)?;
    let body_digest = Sha256::digest(body);
    let expected_body_hash = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(body_digest);
    let expected_padded_body_hash = base64::engine::general_purpose::URL_SAFE.encode(body_digest);
    if provided_body_hash != expected_body_hash && provided_body_hash != expected_padded_body_hash {
        return Err(SignatureError::BodyMismatch);
    }

    Ok(())
}

fn decode_base64url(value: &str) -> Result<Vec<u8>, SignatureError> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(value))
        .map_err(|_| SignatureError::InvalidToken)
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

#[cfg(test)]
#[path = "signature_tests.rs"]
mod tests;
