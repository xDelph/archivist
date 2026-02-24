use std::env;
use std::time::Duration;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tracing::warn;
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::db::Repository;
use crate::db::pool::create_pool;

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
    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let (parts, body) = process(pool().await?, req).await?.into_parts();
    Ok(Response::from_parts(parts, ResponseBody::from(body)))
}

pub(crate) async fn process<R: Repository>(
    repo: &R,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");

    if path.ends_with("/context") {
        return slack_context(repo).await;
    }

    if path.ends_with("/threads") {
        let params = parse_query(query);
        if let (Some(channel_id), Some(ts)) = (params.get("channel_id"), params.get("ts")) {
            return thread_detail(repo, channel_id, ts).await;
        }
        return thread_list(repo).await;
    }

    if path.ends_with("/unfurl") {
        if let Some(url) = parse_query(query).get("url").cloned() {
            return unfurl(&url).await;
        }
        return json_error(StatusCode::BAD_REQUEST, "missing url parameter");
    }

    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Bytes::new())?)
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn thread_list<R: Repository>(repo: &R) -> Result<Response<Bytes>, Error> {
    let threads = repo
        .get_top_threads(50)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let items: Vec<serde_json::Value> = threads
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "channel_id":       t.channel_id,
                "channel_name":     t.channel_name,
                "thread_ts":        t.thread_ts,
                "text":             t.text,
                "created_at":       t.created_at.to_rfc3339(),
                "display_name":     t.display_name,
                "avatar_url":       t.avatar_url,
                "reaction_count":   t.reaction_count,
                "reply_count":      t.reply_count,
                "participant_count":t.participant_count,
                "score":            t.score,
            })
        })
        .collect();

    json_ok(&serde_json::json!({ "threads": items }))
}

async fn thread_detail<R: Repository>(
    repo: &R,
    channel_id: &str,
    thread_ts: &str,
) -> Result<Response<Bytes>, Error> {
    let messages = repo
        .get_thread_messages(channel_id, thread_ts)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let tss: Vec<String> = messages.iter().map(|m| m.ts.clone()).collect();
    let all_files = repo
        .get_files_for_messages(channel_id, &tss)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    // Group files by message ts
    let mut files_by_ts: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    for f in all_files {
        files_by_ts
            .entry(f.message_ts.clone())
            .or_default()
            .push(serde_json::json!({
                "file_id":  f.file_id,
                "name":     f.name,
                "mimetype": f.mimetype,
                "url":      f.storage_url,
            }));
    }

    let items: Vec<serde_json::Value> = messages
        .into_iter()
        .map(|m| {
            let files = files_by_ts.remove(&m.ts).unwrap_or_default();
            serde_json::json!({
                "ts":           m.ts,
                "text":         m.text,
                "display_name": m.display_name,
                "avatar_url":   m.avatar_url,
                "reactions":    m.reactions,
                "files":        files,
            })
        })
        .collect();

    json_ok(&serde_json::json!({ "messages": items }))
}

async fn slack_context<R: Repository>(repo: &R) -> Result<Response<Bytes>, Error> {
    let (users, channels) = tokio::try_join!(repo.get_all_users(), repo.get_all_channels(),)
        .map_err(|e| Error::from(e.to_string()))?;

    let users_obj: serde_json::Map<String, serde_json::Value> = users
        .into_iter()
        .map(|(id, name)| (id, serde_json::Value::String(name)))
        .collect();
    let channels_obj: serde_json::Map<String, serde_json::Value> = channels
        .into_iter()
        .map(|(id, name)| (id, serde_json::Value::String(name)))
        .collect();

    json_ok(&serde_json::json!({
        "users":    users_obj,
        "channels": channels_obj,
    }))
}

async fn unfurl(url: &str) -> Result<Response<Bytes>, Error> {
    // Basic validation — only http(s) URLs
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return json_error(StatusCode::BAD_REQUEST, "invalid url");
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .user_agent("Mozilla/5.0 (compatible; Archivist/1.0)")
        .build()
        .map_err(|e| Error::from(e.to_string()))?;

    let html = match client.get(url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                warn!(url, error = %e, "unfurl: failed to read body");
                return json_ok(&serde_json::json!({}));
            }
        },
        Err(e) => {
            warn!(url, error = %e, "unfurl: fetch failed");
            return json_ok(&serde_json::json!({}));
        }
    };

    let title = og_tag(&html, "og:title")
        .or_else(|| html_title(&html))
        .unwrap_or_default();
    let description = og_tag(&html, "og:description").unwrap_or_default();
    let image = og_tag(&html, "og:image").unwrap_or_default();

    json_ok(&serde_json::json!({
        "title":       title,
        "description": description,
        "image":       image,
    }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn og_tag(html: &str, property: &str) -> Option<String> {
    let needle = format!("property=\"{}\"", property);
    let pos = html.find(&needle)?;
    let after = &html[pos..];
    let content_pos = after.find("content=\"")?;
    let value_start = content_pos + "content=\"".len();
    let end = after[value_start..].find('"')?;
    let value = &after[value_start..value_start + end];
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn html_title(html: &str) -> Option<String> {
    let start = html.find("<title>")? + "<title>".len();
    let end = html[start..].find("</title>")?;
    let title = html[start..start + end].trim().to_owned();
    if title.is_empty() { None } else { Some(title) }
}

fn parse_query(query: &str) -> std::collections::HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let k = parts.next()?.to_owned();
            let v = percent_decode(parts.next().unwrap_or(""));
            if k.is_empty() { None } else { Some((k, v)) }
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(s.len());
    let mut iter = s.bytes();
    while let Some(b) = iter.next() {
        match b {
            b'%' => {
                let h1 = iter.next().and_then(|c| (c as char).to_digit(16));
                let h2 = iter.next().and_then(|c| (c as char).to_digit(16));
                if let (Some(h1), Some(h2)) = (h1, h2) {
                    bytes.push(((h1 << 4) | h2) as u8);
                }
            }
            b'+' => bytes.push(b' '),
            _ => bytes.push(b),
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn json_ok(value: &serde_json::Value) -> Result<Response<Bytes>, Error> {
    let body = serde_json::to_vec(value).map_err(|e| Error::from(e.to_string()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Bytes::from(body))?)
}

fn json_error(status: StatusCode, msg: &str) -> Result<Response<Bytes>, Error> {
    let body = serde_json::to_vec(&serde_json::json!({ "error": msg }))
        .map_err(|e| Error::from(e.to_string()))?;
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Bytes::from(body))?)
}
