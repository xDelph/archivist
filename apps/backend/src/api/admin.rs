use std::{env, time::Duration};

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use serde_json::Value;
use vercel_runtime::{Error, Request, Response, ResponseBody};

use tracing::{error, warn};

const DEFAULT_QSTASH_TIMEOUT_SECONDS: u64 = 10;

/// Vercel entry-point — validates admin auth and triggers QStash worker execution.
pub async fn handler(req: Request) -> Result<Response<ResponseBody>, Error> {
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().unwrap_or("").to_owned();
    match tokio::spawn(async move { handle_request(req).await }).await {
        Ok(Ok(resp)) => Ok(resp),
        Ok(Err(err)) => {
            error!(method, path, query, error = %err, "admin trigger handler failed");
            internal_error_response()
        }
        Err(join_err) => {
            error!(
                method,
                path,
                query,
                is_panic = join_err.is_panic(),
                error = %join_err,
                "admin trigger handler task crashed"
            );
            internal_error_response()
        }
    }
}

async fn handle_request(req: Request) -> Result<Response<ResponseBody>, Error> {
    let admin_token = env::var("ADMIN_TOKEN").unwrap_or_default();
    let qstash_token = qstash_token();
    let worker_token = env::var("BACKFILL_WORKER_TOKEN").unwrap_or_default();

    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let requested_by = req
        .headers()
        .get("x-trigger-source")
        .or_else(|| req.headers().get("user-agent"))
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("api")
        .to_owned();

    let worker_url = resolve_worker_url(&req)?;
    let auth_resp = process(&admin_token, req).await?;
    if auth_resp.status() != StatusCode::ACCEPTED {
        let (parts, body) = auth_resp.into_parts();
        return Ok(Response::from_parts(parts, ResponseBody::from(body)));
    }

    let publish = publish_worker_job(&qstash_token, &worker_token, &worker_url).await?;
    let body = serde_json::json!({
        "ok": true,
        "requestedBy": requested_by,
        "workerUrl": worker_url,
        "qstash": publish,
    })
    .to_string();

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from(body)))?)
}

fn resolve_worker_url(req: &http::Request<Bytes>) -> Result<String, Error> {
    if let Ok(explicit) = env::var("BACKFILL_WORKER_URL")
        && !explicit.trim().is_empty()
    {
        let explicit = explicit.trim().to_owned();
        if !is_http_url(&explicit) {
            return Err(Error::from(format!(
                "invalid BACKFILL_WORKER_URL: {explicit}"
            )));
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
        .ok_or_else(|| Error::from("unable to resolve worker base URL"))?;
    if !is_http_url(&base) {
        return Err(Error::from(format!("invalid worker base URL: {base}")));
    }

    Ok(format!("{base}/api/admin/sync/run"))
}

async fn publish_worker_job(
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

    let qstash_base_url = qstash_url();
    if !is_http_url(worker_url) {
        return Err(Error::from(format!(
            "invalid worker URL scheme: {worker_url}"
        )));
    }

    let publish_url = format!("{qstash_base_url}/v2/publish/{worker_url}");
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

/// Core handler logic — auth check only (trigger is launched by [`handler`]).
pub(crate) async fn process(
    admin_token: &str,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    if req.method() != http::Method::POST {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Bytes::new())?);
    }

    let provided = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if admin_token.is_empty() || provided != format!("Bearer {}", admin_token) {
        warn!("unauthorized sync trigger attempt");
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Bytes::new())?);
    }

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Content-Type", "application/json")
        .body(Bytes::from(r#"{"ok":true}"#))?)
}
