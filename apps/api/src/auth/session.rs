use super::current_unix_timestamp;
use axum::http::{HeaderMap, header::COOKIE};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub(crate) const SESSION_COOKIE_NAME: &str = "archivist_session";
const SESSION_TTL_SECONDS: i64 = 60 * 60 * 24 * 7;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SessionClaims {
    pub(crate) slack_user_id: String,
    pub(crate) team_id: String,
    pub(crate) email: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) avatar_url: Option<String>,
    pub(crate) exp: i64,
}

#[derive(Debug)]
pub(crate) enum SessionError {
    InvalidSession,
    ExpiredSession,
}

pub(crate) fn session_claims(
    slack_user_id: String,
    team_id: String,
    email: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
) -> SessionClaims {
    SessionClaims {
        slack_user_id,
        team_id,
        email,
        display_name,
        avatar_url,
        exp: current_unix_timestamp() + SESSION_TTL_SECONDS,
    }
}

pub(crate) fn session_cookie_header(
    session_secret: &str,
    claims: &SessionClaims,
) -> Result<String, SessionError> {
    let token = build_session_token(session_secret, claims)?;

    Ok(format!(
        "{SESSION_COOKIE_NAME}={token}; Max-Age={SESSION_TTL_SECONDS}; Path=/; HttpOnly; SameSite=Lax"
    ))
}

pub(crate) fn build_session_token(
    session_secret: &str,
    claims: &SessionClaims,
) -> Result<String, SessionError> {
    let payload = serde_json::to_vec(claims).map_err(|_| SessionError::InvalidSession)?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
    let signature = session_signature(session_secret, &payload_b64)?;

    Ok(format!("{payload_b64}.{signature}"))
}

pub(crate) fn validate_session_token(
    session_secret: &str,
    token: &str,
) -> Result<SessionClaims, SessionError> {
    let (payload_b64, signature) = token.split_once('.').ok_or(SessionError::InvalidSession)?;
    let expected_signature = session_signature(session_secret, payload_b64)?;
    if signature != expected_signature {
        return Err(SessionError::InvalidSession);
    }

    let decoded = URL_SAFE_NO_PAD
        .decode(payload_b64.as_bytes())
        .map_err(|_| SessionError::InvalidSession)?;
    let claims: SessionClaims =
        serde_json::from_slice(&decoded).map_err(|_| SessionError::InvalidSession)?;
    if claims.exp <= current_unix_timestamp() {
        return Err(SessionError::ExpiredSession);
    }

    Ok(claims)
}

pub(crate) fn session_cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (name, value) = cookie.trim().split_once('=')?;
                (name == SESSION_COOKIE_NAME).then_some(value)
            })
        })
}

fn session_signature(session_secret: &str, payload_b64: &str) -> Result<String, SessionError> {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(session_secret.as_bytes())
        .map_err(|_| SessionError::InvalidSession)?;
    mac.update(payload_b64.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}
