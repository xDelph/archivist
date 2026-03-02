use std::{env, time::Duration};

use base64::Engine;
use bytes::Bytes;
use chrono::Utc;
use hmac::{Hmac, Mac};
use http::StatusCode;
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tracing::{error, info, warn};
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::api::worker_engine::{
    execute_aggregation_phase, execute_files_phase, execute_messages_phase, release_worker_lock,
    try_acquire_worker_lock,
};
use crate::db::pool::create_pool;
use crate::storage::R2Client;

const DEFAULT_QSTASH_TIMEOUT_SECONDS: u64 = 10;
static POOL: OnceCell<PgPool> = OnceCell::const_new();

#[derive(Debug, Clone, Copy)]
enum WorkerPhase {
    Messages,
    Files,
    Aggregate,
}

impl WorkerPhase {
    fn from_path(path: &str) -> Self {
        if path.ends_with("/api/admin/sync/files") || path.ends_with("/api/sync/files") {
            return Self::Files;
        }
        if path.ends_with("/api/admin/sync/aggregate") || path.ends_with("/api/sync/aggregate") {
            return Self::Aggregate;
        }
        Self::Messages
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Messages => "messages",
            Self::Files => "files",
            Self::Aggregate => "aggregate",
        }
    }
}

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
    let requested_by = req
        .headers()
        .get("x-trigger-source")
        .or_else(|| req.headers().get("user-agent"))
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("worker")
        .to_owned();
    let phase = WorkerPhase::from_path(req.uri().path());

    let auth = process(
        &admin_token,
        worker_token.as_deref(),
        current_signing_key.as_deref(),
        next_signing_key.as_deref(),
        req.clone(),
    )
    .await?;
    if auth.status() != StatusCode::ACCEPTED {
        let (parts, body) = auth.into_parts();
        return Ok(Response::from_parts(parts, ResponseBody::from(body)));
    }

    let pool = pool().await?;
    let storage = R2Client::from_env().await.ok();

    let Some(mut worker_lock_conn) = try_acquire_worker_lock(pool).await? else {
        let body = serde_json::json!({
            "ok": true,
            "phase": phase.as_str(),
            "requestedBy": requested_by,
            "status": "busy",
        })
        .to_string();
        return Ok(Response::builder()
            .status(StatusCode::ACCEPTED)
            .header("Content-Type", "application/json")
            .body(ResponseBody::from(Bytes::from(body)))?);
    };

    let phase_result = async {
        let (worker_result, next_phase) = match phase {
            WorkerPhase::Messages => {
                let outcome = execute_messages_phase(pool, &slack_token, storage.as_ref()).await?;
                let worker_result =
                    serde_json::to_value(&outcome).map_err(|e| Error::from(e.to_string()))?;

                if outcome.status == "succeeded" {
                    let next_url = resolve_next_worker_url(&req, phase)?;
                    let publish = publish_next_phase_job(
                        &qstash_token(),
                        worker_token.as_deref().unwrap_or(""),
                        &next_url,
                    )
                    .await?;
                    let next_phase = Some(serde_json::json!({
                        "phase": "files",
                        "url": next_url,
                        "qstash": publish,
                    }));
                    (worker_result, next_phase)
                } else {
                    (worker_result, None)
                }
            }
            WorkerPhase::Files => {
                let outcome = execute_files_phase(pool, &slack_token, storage.as_ref()).await?;
                let worker_result =
                    serde_json::to_value(&outcome).map_err(|e| Error::from(e.to_string()))?;

                let next_url = resolve_next_worker_url(&req, phase)?;
                let publish = publish_next_phase_job(
                    &qstash_token(),
                    worker_token.as_deref().unwrap_or(""),
                    &next_url,
                )
                .await?;
                let next_phase = Some(serde_json::json!({
                    "phase": "aggregate",
                    "url": next_url,
                    "qstash": publish,
                }));
                (worker_result, next_phase)
            }
            WorkerPhase::Aggregate => {
                let outcome = execute_aggregation_phase(pool).await?;
                let worker_result =
                    serde_json::to_value(&outcome).map_err(|e| Error::from(e.to_string()))?;
                (worker_result, None)
            }
        };

        let body = serde_json::json!({
            "ok": true,
            "phase": phase.as_str(),
            "requestedBy": requested_by,
            "worker": worker_result,
            "nextPhase": next_phase,
        })
        .to_string();

        Ok::<String, Error>(body)
    }
    .await;

    if let Err(err) = release_worker_lock(&mut worker_lock_conn).await {
        warn!(error = %err, "failed to release worker advisory lock");
    }

    let body = phase_result?;
    info!(phase = phase.as_str(), "admin worker phase completed");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from(body)))?)
}

fn resolve_next_worker_url(
    req: &http::Request<Bytes>,
    phase: WorkerPhase,
) -> Result<String, Error> {
    let (override_env, next_path) = match phase {
        WorkerPhase::Messages => ("BACKFILL_FILES_WORKER_URL", "/api/admin/sync/files"),
        WorkerPhase::Files => ("BACKFILL_AGGREGATE_WORKER_URL", "/api/admin/sync/aggregate"),
        WorkerPhase::Aggregate => {
            return Err(Error::from("aggregate phase has no next worker URL"));
        }
    };

    if let Ok(explicit) = env::var(override_env)
        && !explicit.trim().is_empty()
    {
        let explicit = explicit.trim().to_owned();
        if !is_http_url(&explicit) {
            return Err(Error::from(format!("invalid {override_env}: {explicit}")));
        }
        return Ok(explicit);
    }

    let base = env::var("BACKEND_APP_URL")
        .ok()
        .or_else(|| env::var("APP_URL").ok())
        .filter(|value| !value.trim().is_empty())
        .map(|value| normalize_url(&value))
        .or_else(|| {
            let host = req
                .headers()
                .get("x-forwarded-host")
                .or_else(|| req.headers().get("host"))
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())?;
            let scheme = req
                .headers()
                .get("x-forwarded-proto")
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())
                .unwrap_or("https");
            Some(format!("{scheme}://{host}"))
        })
        .ok_or_else(|| Error::from("unable to resolve next worker base URL"))?;
    if !is_http_url(&base) {
        return Err(Error::from(format!("invalid worker base URL: {base}")));
    }
    Ok(format!("{base}{next_path}"))
}

async fn publish_next_phase_job(
    qstash_token: &str,
    worker_token: &str,
    worker_url: &str,
) -> Result<Value, Error> {
    if qstash_token.is_empty() {
        return Err(Error::from("missing UPSTASH_QSTASH_TOKEN"));
    }
    if worker_token.is_empty() {
        return Err(Error::from("missing BACKFILL_WORKER_TOKEN"));
    }
    if !is_http_url(worker_url) {
        return Err(Error::from(format!(
            "invalid worker URL scheme: {worker_url}"
        )));
    }

    let publish_url = format!("{}/v2/publish/{}", qstash_url(), worker_url);
    let timeout = Duration::from_secs(qstash_timeout_seconds());
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| Error::from(e.to_string()))?;

    let response = client
        .post(&publish_url)
        .bearer_auth(qstash_token)
        .header("Content-Type", "application/json")
        .header("Upstash-Method", "POST")
        .header(
            "Upstash-Forward-Authorization",
            format!("Bearer {worker_token}"),
        )
        .body("{}")
        .send()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    if !status.is_success() {
        return Err(Error::from(format!(
            "qstash publish failed: status={} worker_url={} publish_url={} body={}",
            status.as_u16(),
            worker_url,
            publish_url,
            body
        )));
    }
    serde_json::from_str(&body).map_err(|e| Error::from(e.to_string()))
}

fn qstash_timeout_seconds() -> u64 {
    env::var("QSTASH_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_QSTASH_TIMEOUT_SECONDS)
}

fn qstash_token() -> String {
    env::var("UPSTASH_QSTASH_TOKEN").unwrap_or_default()
}

fn qstash_url() -> String {
    normalize_url(
        &env::var("UPSTASH_QSTASH_URL").unwrap_or_else(|_| "https://qstash.upstash.io".to_owned()),
    )
}

fn normalize_url(value: &str) -> String {
    let trimmed = value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_owned();
    }
    format!("https://{trimmed}")
}

fn is_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
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
    let provided_sig = decode_base64url(sig_b64).map_err(|_| "invalid jwt signature encoding")?;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).map_err(|_| "invalid signing key bytes")?;
    mac.update(signing_input.as_bytes());
    let expected_sig = mac.finalize().into_bytes();
    if provided_sig != expected_sig.as_slice() {
        return Err("jwt signature mismatch");
    }

    let claims_json = decode_base64url(claims_b64).map_err(|_| "invalid jwt claims encoding")?;
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

    let body_digest = Sha256::digest(body);
    let expected_body_hash_no_pad =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(body_digest.as_slice());
    let expected_body_hash_padded =
        base64::engine::general_purpose::URL_SAFE.encode(body_digest.as_slice());
    let provided_body_hash = claims.body.ok_or("missing body claim")?;
    if provided_body_hash != expected_body_hash_no_pad
        && provided_body_hash != expected_body_hash_padded
    {
        return Err("body hash mismatch");
    }

    Ok(())
}

fn decode_base64url(value: &str) -> Result<Vec<u8>, base64::DecodeError> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(value))
}
