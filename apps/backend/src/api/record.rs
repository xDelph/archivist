use std::collections::HashMap;
use std::env;

use bytes::Bytes;
use chrono::Utc;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tracing::error;
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::db::pool::create_pool;
use crate::db::{FileRow, PeriodRankedThread, Repository, ThreadSummary, ThreadWithWeeklyScore};
use crate::render::components::render_threads_content;
use crate::render::page::{render_page, render_thread_page, render_weekly_page};
use crate::render::thread::render_thread_fragment;

// ── DB connection pool ────────────────────────────────────────────────────────

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

// ── In-process TTL cache ──────────────────────────────────────────────────────
//
// Threads, users and channels change only during a backfill.  A 5-minute cache
// eliminates all DB round-trips for search keystrokes and filter changes — all
// of which are pure Rust operations on the cached Vec.
//
// RwLock: many parallel reads (search debounce), rare writes (cache miss).
// In test mode both variants are compiled away and the repo is called directly
// to prevent cross-test data contamination.

#[cfg(not(test))]
use std::sync::OnceLock;
#[cfg(not(test))]
use std::time::{Duration, Instant};
#[cfg(not(test))]
use tokio::sync::RwLock;

#[cfg(not(test))]
const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

#[cfg(not(test))]
struct AppCache {
    threads: Option<(Instant, Vec<ThreadSummary>)>,
    recent_threads: Option<(Instant, Vec<ThreadSummary>)>,
    users: Option<(Instant, Vec<(String, String)>)>,
    channels: Option<(Instant, Vec<(String, String)>)>,
}

#[cfg(not(test))]
static CACHE: OnceLock<RwLock<AppCache>> = OnceLock::new();

#[cfg(not(test))]
fn app_cache() -> &'static RwLock<AppCache> {
    CACHE.get_or_init(|| {
        RwLock::new(AppCache {
            threads: None,
            recent_threads: None,
            users: None,
            channels: None,
        })
    })
}

#[cfg(not(test))]
pub(crate) async fn cached_threads<R: Repository>(
    repo: &R,
) -> std::result::Result<Vec<ThreadSummary>, anyhow::Error> {
    {
        let c = app_cache().read().await;
        if let Some((ts, data)) = &c.threads
            && ts.elapsed() < CACHE_TTL
        {
            return Ok(data.clone());
        }
    }
    let data = repo.get_top_threads(200).await?;
    let mut cache = app_cache().write().await;
    if data.is_empty() {
        if let Some((_, previous)) = &cache.threads
            && !previous.is_empty()
        {
            return Ok(previous.clone());
        }
        return Ok(data);
    }
    cache.threads = Some((Instant::now(), data.clone()));
    Ok(data)
}

#[cfg(test)]
pub(crate) async fn cached_threads<R: Repository>(
    repo: &R,
) -> std::result::Result<Vec<ThreadSummary>, anyhow::Error> {
    repo.get_top_threads(200).await
}

#[cfg(not(test))]
pub(crate) async fn cached_recent_threads<R: Repository>(
    repo: &R,
) -> std::result::Result<Vec<ThreadSummary>, anyhow::Error> {
    {
        let c = app_cache().read().await;
        if let Some((ts, data)) = &c.recent_threads
            && ts.elapsed() < CACHE_TTL
        {
            return Ok(data.clone());
        }
    }
    let data = repo.get_recent_threads(2000).await?;
    let mut cache = app_cache().write().await;
    if data.is_empty() {
        if let Some((_, previous)) = &cache.recent_threads
            && !previous.is_empty()
        {
            return Ok(previous.clone());
        }
        return Ok(data);
    }
    cache.recent_threads = Some((Instant::now(), data.clone()));
    Ok(data)
}

#[cfg(test)]
pub(crate) async fn cached_recent_threads<R: Repository>(
    repo: &R,
) -> std::result::Result<Vec<ThreadSummary>, anyhow::Error> {
    repo.get_recent_threads(2000).await
}

#[cfg(not(test))]
pub(crate) async fn cached_users<R: Repository>(
    repo: &R,
) -> std::result::Result<Vec<(String, String)>, anyhow::Error> {
    {
        let c = app_cache().read().await;
        if let Some((ts, data)) = &c.users
            && ts.elapsed() < CACHE_TTL
        {
            return Ok(data.clone());
        }
    }
    let data = repo.get_all_users().await?;
    app_cache().write().await.users = Some((Instant::now(), data.clone()));
    Ok(data)
}

#[cfg(test)]
pub(crate) async fn cached_users<R: Repository>(
    repo: &R,
) -> std::result::Result<Vec<(String, String)>, anyhow::Error> {
    repo.get_all_users().await
}

#[cfg(not(test))]
pub(crate) async fn cached_channels<R: Repository>(
    repo: &R,
) -> std::result::Result<Vec<(String, String)>, anyhow::Error> {
    {
        let c = app_cache().read().await;
        if let Some((ts, data)) = &c.channels
            && ts.elapsed() < CACHE_TTL
        {
            return Ok(data.clone());
        }
    }
    let data = repo.get_all_channels().await?;
    app_cache().write().await.channels = Some((Instant::now(), data.clone()));
    Ok(data)
}

#[cfg(test)]
pub(crate) async fn cached_channels<R: Repository>(
    repo: &R,
) -> std::result::Result<Vec<(String, String)>, anyhow::Error> {
    repo.get_all_channels().await
}

// ── Vercel entry-point ────────────────────────────────────────────────────────

pub async fn handler(req: Request) -> Result<Response<ResponseBody>, Error> {
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().unwrap_or("").to_owned();
    match tokio::spawn(async move { handle_request(req).await }).await {
        Ok(Ok(resp)) => Ok(resp),
        Ok(Err(err)) => {
            error!(
                method,
                path,
                query,
                error = %err,
                "record handler failed"
            );
            internal_error_response()
        }
        Err(join_err) => {
            error!(
                method,
                path,
                query,
                is_panic = join_err.is_panic(),
                error = %join_err,
                "record handler task crashed"
            );
            internal_error_response()
        }
    }
}

async fn handle_request(req: Request) -> Result<Response<ResponseBody>, Error> {
    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let pool = pool().await?;

    if req.uri().path().starts_with("/api/auth") {
        let (parts, body) = crate::api::auth::process(pool, req).await?.into_parts();
        return Ok(Response::from_parts(parts, ResponseBody::from(body)));
    }

    let mut viewer = None;
    if path_requires_auth(req.uri().path()) {
        match crate::api::auth::authorize_request(pool, &req).await {
            Ok(ctx) => {
                viewer = Some(ctx);
            }
            Err(resp) => {
                let (parts, body) = resp.into_parts();
                return Ok(Response::from_parts(parts, ResponseBody::from(body)));
            }
        }
    }

    let (parts, body) = process(pool, req, viewer.as_ref()).await?.into_parts();
    Ok(Response::from_parts(parts, ResponseBody::from(body)))
}

fn path_requires_auth(path: &str) -> bool {
    path.starts_with("/api/record") || path.starts_with("/record")
}

fn internal_error_response() -> Result<Response<ResponseBody>, Error> {
    let body = Bytes::from_static(br#"{"ok":false,"error":"internal server error"}"#);
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(body))?)
}

pub(crate) async fn process<R: Repository>(
    repo: &R,
    req: http::Request<Bytes>,
    viewer: Option<&crate::api::auth::AuthContext>,
) -> Result<Response<Bytes>, Error> {
    let raw_path = req.uri().path();
    let path = if raw_path == "/" {
        raw_path
    } else {
        raw_path.trim_end_matches('/')
    };
    let query = req.uri().query().unwrap_or("");

    if path.starts_with("/api/record") {
        return crate::api::record_json::process(repo, path, query, viewer).await;
    }

    if path.ends_with("/thread") {
        let is_htmx = req.headers().contains_key("hx-request");
        return thread_fragment(repo, query, is_htmx).await;
    }

    if path.ends_with("/threads") {
        return threads_fragment(repo, query).await;
    }

    if path.ends_with("/weekly") {
        return weekly_page_handler(repo, query).await;
    }

    page_handler(repo).await
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn page_handler<R: Repository>(repo: &R) -> Result<Response<Bytes>, Error> {
    let (threads, users_vec) = tokio::try_join!(cached_threads(repo), cached_users(repo))
        .map_err(|e| Error::from(e.to_string()))?;

    let users: HashMap<String, String> = users_vec.into_iter().collect();
    let workspace_url = env::var("SLACK_WORKSPACE_URL").ok();
    let markup = render_page(&threads, workspace_url.as_deref(), &users);
    html_ok(markup)
}

async fn weekly_page_handler<R: Repository>(
    repo: &R,
    query: &str,
) -> Result<Response<Bytes>, Error> {
    let params = parse_query(query);
    let tab = match params.get("tab").map(String::as_str) {
        Some("week") => "week",
        Some("month") => "month",
        _ => "top",
    };

    let users_vec = cached_users(repo)
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    let users: HashMap<String, String> = users_vec.into_iter().collect();

    let (top_threads, ranked_threads): (Vec<ThreadWithWeeklyScore>, Vec<PeriodRankedThread>) =
        match tab {
            "week" => (
                vec![],
                repo.get_weekly_ranked_threads(200)
                    .await
                    .map_err(|e| Error::from(e.to_string()))?,
            ),
            "month" => (
                vec![],
                repo.get_monthly_ranked_threads(200)
                    .await
                    .map_err(|e| Error::from(e.to_string()))?,
            ),
            _ => (
                repo.get_top_threads_with_weekly(200)
                    .await
                    .map_err(|e| Error::from(e.to_string()))?,
                vec![],
            ),
        };

    let workspace_url = env::var("SLACK_WORKSPACE_URL").ok();
    html_ok(render_weekly_page(
        tab,
        &top_threads,
        &ranked_threads,
        workspace_url.as_deref(),
        &users,
    ))
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

    let (mut threads, users_vec) = tokio::try_join!(cached_threads(repo), cached_users(repo))
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

async fn thread_fragment<R: Repository>(
    repo: &R,
    query: &str,
    is_htmx: bool,
) -> Result<Response<Bytes>, Error> {
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

    // messages is per-thread (can't cache generically); users/channels are cached
    let (messages, users_vec, channels_vec) = tokio::try_join!(
        repo.get_thread_messages(&channel_id, &ts),
        cached_users(repo),
        cached_channels(repo),
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

    let markup = if is_htmx {
        render_thread_fragment(&messages, &files_by_ts, &users, &channels, &search)
    } else {
        let workspace_url = env::var("SLACK_WORKSPACE_URL").ok();
        render_thread_page(
            &messages,
            &files_by_ts,
            &users,
            &channels,
            &search,
            workspace_url.as_deref(),
        )
    };
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
