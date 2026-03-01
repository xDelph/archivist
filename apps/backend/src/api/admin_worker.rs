use std::env;

use base64::Engine;
use bytes::Bytes;
use chrono::Utc;
use hmac::{Hmac, Mac};
use http::StatusCode;
use http_body_util::BodyExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tracing::{error, warn};
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::api::worker_engine::execute_worker_run;
use crate::db::pool::create_pool;
use crate::storage::R2Client;

static POOL: OnceCell<PgPool> = OnceCell::const_new();

async fn pool() -> Result<&'static PgPool, Error> {
    POOL.get_or_try_init(|| async {
        let url = env::var("DATABASE_URL").unwrap_or_default();
        create_pool(&url)
            .await
            .map_err(|e| Error::from(e.to_string()))
    })
    .await
}

pub async fn handler(req: Request) -> Result<Response<ResponseBody>, Error> {
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().unwrap_or("").to_owned();
    match tokio::spawn(async move { handle_request(req).await }).await {
        Ok(Ok(resp)) => Ok(resp),
        Ok(Err(err)) => {
            error!(method, path, query, error = %err, "admin worker handler failed");
            internal_error_response()
        }
        Err(join_err) => {
            error!(
                method,
                path,
                query,
                is_panic = join_err.is_panic(),
                error = %join_err,
                "admin worker handler task crashed"
            );
            internal_error_response()
        }
    }
}

async fn handle_request(req: Request) -> Result<Response<ResponseBody>, Error> {
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or_default();
    let worker_token = env::var("BACKFILL_WORKER_TOKEN").ok();
    let current_signing_key = env::var("UPSTASH_QSTASH_CURRENT_SIGNING_KEY").ok();
    let next_signing_key = env::var("UPSTASH_QSTASH_NEXT_SIGNING_KEY").ok();
    let slack_token = env::var("SLACK_USER_TOKEN")
        .or_else(|_| env::var("SLACK_BOT_TOKEN"))
        .unwrap_or_default();

    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let resp = process(
        &admin_token,
        worker_token.as_deref(),
        current_signing_key.as_deref(),
        next_signing_key.as_deref(),
        req,
    )
    .await?;
    if resp.status() != StatusCode::ACCEPTED {
        let (parts, body) = resp.into_parts();
        return Ok(Response::from_parts(parts, ResponseBody::from(body)));
    }

    let storage = R2Client::from_env().await.ok();
    let outcome = execute_worker_run(pool().await?, &slack_token, storage.as_ref())
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let body = serde_json::to_string(&serde_json::json!({
        "ok": true,
        "worker": outcome,
    }))
    .map_err(|e| Error::from(e.to_string()))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from(body)))?)
}

fn internal_error_response() -> Result<Response<ResponseBody>, Error> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from_static(
            br#"{"ok":false,"error":"internal server error"}"#,
        )))?)
}

pub(crate) async fn process(
    admin_token: &str,
    worker_token: Option<&str>,
    current_signing_key: Option<&str>,
    next_signing_key: Option<&str>,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    if req.method() != http::Method::POST && req.method() != http::Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Bytes::new())?);
    }

    let provided = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let admin_ok = !admin_token.is_empty() && provided == format!("Bearer {}", admin_token);
    let worker_ok = worker_token
        .is_some_and(|secret| !secret.is_empty() && provided == format!("Bearer {}", secret));
    if admin_ok {
        return Ok(Response::builder()
            .status(StatusCode::ACCEPTED)
            .header("Content-Type", "application/json")
            .body(Bytes::from(r#"{"ok":true}"#))?);
    }
    if !worker_ok {
        warn!("unauthorized backfill worker attempt");
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Bytes::new())?);
    }
    if let Err(reason) = verify_qstash_signature(&req, current_signing_key, next_signing_key) {
        warn!(reason, "invalid qstash signature");
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Bytes::new())?);
    }

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Content-Type", "application/json")
        .body(Bytes::from(r#"{"ok":true}"#))?)
}

#[derive(Debug, Deserialize)]
struct QStashClaims {
    exp: Option<i64>,
    nbf: Option<i64>,
    iss: Option<String>,
    body: Option<String>,
}

fn verify_qstash_signature(
    req: &http::Request<Bytes>,
    current_signing_key: Option<&str>,
    next_signing_key: Option<&str>,
) -> Result<(), &'static str> {
    let token = req
        .headers()
        .get("upstash-signature")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("missing upstash-signature header")?;
    let token = token.strip_prefix("Bearer ").unwrap_or(token);
    let body = req.body();

    let current_ok = current_signing_key
        .filter(|key| !key.is_empty())
        .is_some_and(|key| verify_signature_with_key(token, key, body).is_ok());
    if current_ok {
        return Ok(());
    }

    let next_ok = next_signing_key
        .filter(|key| !key.is_empty())
        .is_some_and(|key| verify_signature_with_key(token, key, body).is_ok());
    if next_ok {
        return Ok(());
    }

    Err("signature verification failed for current/next keys")
}

fn verify_signature_with_key(token: &str, key: &str, body: &[u8]) -> Result<(), &'static str> {
    let mut parts = token.split('.');
    let header_b64 = parts.next().ok_or("invalid jwt header")?;
    let claims_b64 = parts.next().ok_or("invalid jwt claims")?;
    let sig_b64 = parts.next().ok_or("invalid jwt signature")?;
    if parts.next().is_some() {
        return Err("invalid jwt format");
    }

    let signing_input = format!("{header_b64}.{claims_b64}");
    let provided_sig = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| "invalid jwt signature encoding")?;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).map_err(|_| "invalid signing key bytes")?;
    mac.update(signing_input.as_bytes());
    let expected_sig = mac.finalize().into_bytes();
    if provided_sig != expected_sig.as_slice() {
        return Err("jwt signature mismatch");
    }

    let claims_json = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(claims_b64)
        .map_err(|_| "invalid jwt claims encoding")?;
    let claims: QStashClaims =
        serde_json::from_slice(&claims_json).map_err(|_| "invalid jwt claims json")?;

    let now = Utc::now().timestamp();
    let exp = claims.exp.ok_or("missing exp claim")?;
    if now > exp {
        return Err("expired signature");
    }
    if let Some(nbf) = claims.nbf
        && now < nbf
    {
        return Err("signature not active yet");
    }
    if let Some(iss) = claims.iss
        && iss != "Upstash"
    {
        return Err("unexpected issuer");
    }

    let expected_body_hash =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(body).as_slice());
    let provided_body_hash = claims.body.ok_or("missing body claim")?;
    if provided_body_hash != expected_body_hash {
        return Err("body hash mismatch");
    }

    Ok(())
}
