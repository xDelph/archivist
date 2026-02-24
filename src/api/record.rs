use std::collections::HashMap;
use std::env;

use bytes::Bytes;
use chrono::Utc;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::db::pool::create_pool;
use crate::db::{FileRow, Repository};
use crate::render::components::render_threads_content;
use crate::render::page::render_page;
use crate::render::thread::render_thread_fragment;

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

    if path.ends_with("/thread") {
        return thread_fragment(repo, req.uri().query().unwrap_or("")).await;
    }

    if path.ends_with("/threads") {
        return threads_fragment(repo, req.uri().query().unwrap_or("")).await;
    }

    page_handler(repo).await
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn page_handler<R: Repository>(repo: &R) -> Result<Response<Bytes>, Error> {
    // Fetch 200 so the filter bar can populate user/channel options from a wider set
    let (threads, users_vec) = tokio::try_join!(repo.get_top_threads(200), repo.get_all_users(),)
        .map_err(|e| Error::from(e.to_string()))?;

    let users: HashMap<String, String> = users_vec.into_iter().collect();
    let workspace_url = env::var("SLACK_WORKSPACE_URL").ok();
    let markup = render_page(&threads, workspace_url.as_deref(), &users);
    html_ok(markup)
}

async fn threads_fragment<R: Repository>(repo: &R, query: &str) -> Result<Response<Bytes>, Error> {
    let params = parse_query(query);
    let sort = params
        .get("sort")
        .map(String::as_str)
        .unwrap_or("score")
        .to_owned();
    let period = params
        .get("period")
        .map(String::as_str)
        .unwrap_or("all")
        .to_owned();
    let user = params
        .get("user")
        .map(String::as_str)
        .unwrap_or("")
        .to_owned();
    let channel = params
        .get("channel")
        .map(String::as_str)
        .unwrap_or("")
        .to_owned();
    let search = params
        .get("search")
        .map(String::as_str)
        .unwrap_or("")
        .to_owned();

    let (mut threads, users_vec) =
        tokio::try_join!(repo.get_top_threads(200), repo.get_all_users(),)
            .map_err(|e| Error::from(e.to_string()))?;
    let users: HashMap<String, String> = users_vec.into_iter().collect();

    // Period filter — compare against thread_ts (Unix seconds), NOT created_at.
    // created_at reflects when we archived it (e.g. all on same backfill day), not when posted.
    if period != "all" {
        let days: i64 = if period == "7d" { 7 } else { 30 };
        let cutoff_secs = (Utc::now() - chrono::Duration::days(days)).timestamp() as f64;
        threads.retain(|t| t.thread_ts.parse::<f64>().unwrap_or(0.0) >= cutoff_secs);
    }

    // User / channel / search filters
    if !user.is_empty() {
        threads.retain(|t| t.display_name == user);
    }
    if !channel.is_empty() {
        threads.retain(|t| t.channel_name == channel);
    }
    if !search.is_empty() {
        let sl = search.to_lowercase();
        threads.retain(|t| t.text.to_lowercase().contains(&sl));
    }

    // Sort — date uses thread_ts (when posted in Slack), not created_at
    match sort.as_str() {
        "date" => threads.sort_by(|a, b| {
            let a_ts = a.thread_ts.parse::<f64>().unwrap_or(0.0);
            let b_ts = b.thread_ts.parse::<f64>().unwrap_or(0.0);
            b_ts.partial_cmp(&a_ts).unwrap_or(std::cmp::Ordering::Equal)
        }),
        "reactions" => threads.sort_by(|a, b| b.reaction_count.cmp(&a.reaction_count)),
        "replies" => threads.sort_by(|a, b| b.reply_count.cmp(&a.reply_count)),
        _ => {} // "score" — DB order is already correct
    }

    threads.truncate(50);

    html_ok(render_threads_content(&threads, &search, &users))
}

async fn thread_fragment<R: Repository>(repo: &R, query: &str) -> Result<Response<Bytes>, Error> {
    let params = parse_query(query);

    let channel_id = match params.get("channel_id") {
        Some(v) => v.clone(),
        None => return error_response(StatusCode::BAD_REQUEST, "missing channel_id"),
    };
    let ts = match params.get("ts") {
        Some(v) => v.clone(),
        None => return error_response(StatusCode::BAD_REQUEST, "missing ts"),
    };
    let search = params.get("search").cloned().unwrap_or_default();

    let (messages, users_vec, channels_vec) = tokio::try_join!(
        repo.get_thread_messages(&channel_id, &ts),
        repo.get_all_users(),
        repo.get_all_channels(),
    )
    .map_err(|e| Error::from(e.to_string()))?;

    let tss: Vec<String> = messages.iter().map(|m| m.ts.clone()).collect();
    let files = repo
        .get_files_for_messages(&channel_id, &tss)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let users: HashMap<String, String> = users_vec.into_iter().collect();
    let channels: HashMap<String, String> = channels_vec.into_iter().collect();
    let files_by_ts = group_files_by_ts(files);

    let markup = render_thread_fragment(&messages, &files_by_ts, &users, &channels, &search);
    html_ok(markup)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn group_files_by_ts(files: Vec<FileRow>) -> HashMap<String, Vec<FileRow>> {
    let mut map: HashMap<String, Vec<FileRow>> = HashMap::new();
    for f in files {
        map.entry(f.message_ts.clone()).or_default().push(f);
    }
    map
}

fn html_ok(markup: maud::Markup) -> Result<Response<Bytes>, Error> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Bytes::from(markup.into_string()))?)
}

fn error_response(status: StatusCode, msg: &str) -> Result<Response<Bytes>, Error> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Bytes::from(msg.to_owned()))?)
}

fn parse_query(query: &str) -> HashMap<String, String> {
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
