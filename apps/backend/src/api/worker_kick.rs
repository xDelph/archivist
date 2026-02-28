use std::{env, time::Duration};

use http::HeaderMap;
use tracing::{info, warn};

const WORKER_PATH: &str = "/api/admin/backfill/run";

pub(crate) fn infer_base_url_from_headers(headers: &HeaderMap) -> Option<String> {
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|value| value.to_str().ok())
        .and_then(first_header_value)?;
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(first_header_value)
        .unwrap_or("https");
    Some(format!("{proto}://{host}"))
}

pub(crate) fn schedule_worker_kick(
    base_url_hint: Option<String>,
    bearer_token: Option<String>,
    reason: &'static str,
) {
    let Some(token) = bearer_token.filter(|value| !value.is_empty()) else {
        warn!(reason, "skipping worker kick: missing auth token");
        return;
    };
    let Some(base_url) = resolve_base_url(base_url_hint.as_deref()) else {
        warn!(reason, "skipping worker kick: missing base url");
        return;
    };

    let worker_url = format!("{base_url}{WORKER_PATH}");
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
        {
            Ok(client) => client,
            Err(err) => {
                warn!(reason, error = %err, "failed to build worker kick client");
                return;
            }
        };

        let trigger_source = format!("self-kick:{reason}");
        match client
            .post(&worker_url)
            .bearer_auth(token)
            .header("x-trigger-source", trigger_source)
            .send()
            .await
        {
            Ok(response) => {
                info!(
                    reason,
                    status = %response.status(),
                    worker_url = %worker_url,
                    "triggered async worker kick"
                );
            }
            Err(err) => {
                warn!(
                    reason,
                    error = %err,
                    worker_url = %worker_url,
                    "async worker kick failed"
                );
            }
        }
    });
}

fn resolve_base_url(hint: Option<&str>) -> Option<String> {
    if let Some(url) = hint.and_then(normalize_base_url) {
        return Some(url);
    }
    if let Some(url) = env::var("BACKEND_APP_URL")
        .ok()
        .as_deref()
        .and_then(normalize_base_url)
    {
        return Some(url);
    }
    if let Some(url) = env::var("APP_URL")
        .ok()
        .as_deref()
        .and_then(normalize_base_url)
    {
        return Some(url);
    }
    env::var("VERCEL_URL")
        .ok()
        .as_deref()
        .and_then(normalize_base_url)
}

fn normalize_base_url(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Some(trimmed.to_owned());
    }
    if trimmed.contains("://") {
        return None;
    }
    Some(format!("https://{trimmed}"))
}

fn first_header_value(value: &str) -> Option<&str> {
    value
        .split(',')
        .next()
        .map(str::trim)
        .filter(|v| !v.is_empty())
}
