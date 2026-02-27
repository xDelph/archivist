use std::collections::{HashMap, HashSet};
use std::env;

use bytes::Bytes;
use chrono::{Datelike, Utc};
use http::StatusCode;
use serde::Serialize;
use serde_json::Value;
use vercel_runtime::{Error, Response};

use crate::db::{FileRow, PeriodRankedThread, Repository, ThreadMessage, ThreadSummary};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiAuthor {
    name: String,
    initials: String,
    avatar_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiThread {
    id: String,
    channel_id: String,
    ts: String,
    author: ApiAuthor,
    channel: String,
    message: String,
    date: String,
    replies: i64,
    reactions: i64,
    participants: i64,
    score: i64,
    has_files: bool,
    url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiChannelStat {
    name: String,
    count: i64,
    percentage: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiActivityPoint {
    date: String,
    messages: i64,
    threads: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiOverviewStats {
    total_messages: i64,
    total_threads: i64,
    total_files: i64,
    total_users: i64,
    messages_change: f64,
    threads_change: f64,
    files_change: f64,
    users_change: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiThreadsResponse {
    tab: String,
    workspace_url: Option<String>,
    threads: Vec<ApiThread>,
    users: Vec<String>,
    channel_stats: Vec<ApiChannelStat>,
    activity_data: Vec<ApiActivityPoint>,
    overview_stats: ApiOverviewStats,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiThreadFile {
    name: String,
    mimetype: String,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiThreadMessage {
    id: String,
    ts: String,
    author: ApiAuthor,
    message: String,
    timestamp: String,
    timestamp_iso: String,
    reactions: i64,
    files: Vec<ApiThreadFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiThreadResponse {
    channel_id: String,
    ts: String,
    messages: Vec<ApiThreadMessage>,
}

struct RootFileSummary {
    file_counts_by_thread: HashMap<(String, String), i64>,
    total_files: i64,
}

#[derive(Default)]
struct OverviewChanges {
    messages_change: f64,
    threads_change: f64,
    files_change: f64,
    users_change: f64,
}

#[derive(Default)]
struct MetricAccumulator {
    messages: i64,
    threads: i64,
    files: i64,
    users: HashSet<String>,
}

impl MetricAccumulator {
    fn add(&mut self, thread: &ThreadSummary, file_count: i64) {
        self.threads += 1;
        self.messages += thread.reply_count + 1;
        self.files += file_count;
        self.users.insert(thread.display_name.clone());
    }
}

pub(crate) async fn process<R: Repository>(
    repo: &R,
    path: &str,
    query: &str,
) -> Result<Response<Bytes>, Error> {
    if path.ends_with("/thread") {
        return thread_json(repo, query).await;
    }
    threads_json(repo, query).await
}

async fn threads_json<R: Repository>(repo: &R, query: &str) -> Result<Response<Bytes>, Error> {
    let params = parse_query(query);
    let tab = match params.get("tab").map(String::as_str) {
        Some("week") => "week",
        Some("month") => "month",
        _ => "top",
    };
    let sort = params.get("sort").map(String::as_str).unwrap_or("score");
    let period = params.get("period").map(String::as_str).unwrap_or("all");
    let user = params.get("user").map(String::as_str).unwrap_or("").trim();
    let channel = params
        .get("channel")
        .map(String::as_str)
        .unwrap_or("")
        .trim_start_matches('#')
        .trim();
    let search = params.get("search").map(String::as_str).unwrap_or("");
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(200)
        .min(200);

    let base_threads = load_threads_for_tab(repo, tab).await?;

    let mut filtered_for_users = base_threads.clone();
    apply_channel_and_search_filters(&mut filtered_for_users, channel, search);
    apply_period_filter(&mut filtered_for_users, period);
    let users = build_user_options(&filtered_for_users);

    let mut candidate_threads = base_threads;
    apply_channel_and_search_filters(&mut candidate_threads, channel, search);
    apply_user_filter(&mut candidate_threads, user);
    apply_period_filter(&mut candidate_threads, period);
    apply_sort(&mut candidate_threads, sort);
    candidate_threads.truncate(limit);

    let root_files = fetch_root_file_summary(repo, &candidate_threads).await?;
    let overview_changes =
        compute_overview_changes(repo, tab, period, user, channel, search).await?;
    let channel_stats = build_channel_stats(&candidate_threads);
    let activity_data = build_activity_data(&candidate_threads);
    let overview_stats = build_overview_stats(
        &candidate_threads,
        root_files.total_files,
        &overview_changes,
    );

    let threads = candidate_threads
        .iter()
        .map(|thread| ApiThread {
            id: format!("{}:{}", thread.channel_id, thread.thread_ts),
            channel_id: thread.channel_id.clone(),
            ts: thread.thread_ts.clone(),
            author: ApiAuthor {
                name: thread.display_name.clone(),
                initials: initials(&thread.display_name),
                avatar_url: thread.avatar_url.clone(),
            },
            channel: format!("#{}", thread.channel_name),
            message: thread.text.clone(),
            date: format_day_date(&thread.thread_ts),
            replies: thread.reply_count,
            reactions: thread.reaction_count,
            participants: thread.participant_count,
            score: thread.score,
            has_files: root_files
                .file_counts_by_thread
                .contains_key(&(thread.channel_id.clone(), thread.thread_ts.clone())),
            url: extract_first_url(&thread.text),
        })
        .collect();

    let resp = ApiThreadsResponse {
        tab: tab.to_owned(),
        workspace_url: env::var("SLACK_WORKSPACE_URL").ok(),
        threads,
        users,
        channel_stats,
        activity_data,
        overview_stats,
    };
    json_ok(&resp)
}

async fn thread_json<R: Repository>(repo: &R, query: &str) -> Result<Response<Bytes>, Error> {
    let params = parse_query(query);
    let channel_id = match params.get("channel_id") {
        Some(v) => v.clone(),
        None => return error_response(StatusCode::BAD_REQUEST, "missing channel_id"),
    };
    let ts = match params.get("ts") {
        Some(v) => v.clone(),
        None => return error_response(StatusCode::BAD_REQUEST, "missing ts"),
    };

    let messages = repo
        .get_thread_messages(&channel_id, &ts)
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    let tss: Vec<String> = messages.iter().map(|m| m.ts.clone()).collect();
    let files = repo
        .get_files_for_messages(&channel_id, &tss)
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    let mut files_by_ts = group_files_by_ts(files);

    let messages = messages
        .into_iter()
        .map(|message| map_thread_message(message, &mut files_by_ts))
        .collect();

    let resp = ApiThreadResponse {
        channel_id,
        ts,
        messages,
    };
    json_ok(&resp)
}

fn map_thread_message(
    message: ThreadMessage,
    files_by_ts: &mut HashMap<String, Vec<FileRow>>,
) -> ApiThreadMessage {
    let ThreadMessage {
        ts,
        text,
        display_name,
        avatar_url,
        reactions,
    } = message;

    let files = files_by_ts
        .remove(&ts)
        .unwrap_or_default()
        .into_iter()
        .map(|file| ApiThreadFile {
            name: file.name,
            mimetype: file.mimetype,
            url: file.storage_url,
        })
        .collect();

    ApiThreadMessage {
        id: ts.clone(),
        author: ApiAuthor {
            name: display_name.clone(),
            initials: initials(&display_name),
            avatar_url,
        },
        ts: ts.clone(),
        message: text,
        timestamp: format_time_24h(&ts),
        timestamp_iso: format_time_iso(&ts),
        reactions: reaction_total(&reactions),
        files,
    }
}

async fn load_threads_for_tab<R: Repository>(
    repo: &R,
    tab: &str,
) -> Result<Vec<ThreadSummary>, Error> {
    if tab == "week" {
        let rows = repo
            .get_weekly_ranked_threads(200)
            .await
            .map_err(|e| Error::from(e.to_string()))?;
        return Ok(normalize_ranked_threads(rows));
    }
    if tab == "month" {
        let rows = repo
            .get_monthly_ranked_threads(200)
            .await
            .map_err(|e| Error::from(e.to_string()))?;
        return Ok(normalize_ranked_threads(rows));
    }

    crate::api::record::cached_threads(repo)
        .await
        .map_err(|e| Error::from(e.to_string()))
}

fn normalize_ranked_threads(rows: Vec<PeriodRankedThread>) -> Vec<ThreadSummary> {
    rows.into_iter()
        .map(|row| {
            let mut thread = row.thread;
            thread.score = row.rank_score;
            thread
        })
        .collect()
}

fn apply_period_filter(threads: &mut Vec<ThreadSummary>, period: &str) {
    if period == "all" {
        return;
    }
    let days = if period == "7d" { 7 } else { 30 };
    let cutoff_secs = (Utc::now() - chrono::Duration::days(days)).timestamp() as f64;
    threads.retain(|thread| thread.thread_ts.parse::<f64>().unwrap_or(0.0) >= cutoff_secs);
}

fn apply_channel_and_search_filters(threads: &mut Vec<ThreadSummary>, channel: &str, search: &str) {
    if !channel.is_empty() {
        threads.retain(|thread| thread.channel_name == channel);
    }
    if !search.is_empty() {
        let query = search.to_lowercase();
        threads.retain(|thread| thread.text.to_lowercase().contains(&query));
    }
}

fn apply_user_filter(threads: &mut Vec<ThreadSummary>, user: &str) {
    if !user.is_empty() {
        threads.retain(|thread| thread.display_name == user);
    }
}

fn apply_sort(threads: &mut Vec<ThreadSummary>, sort: &str) {
    match sort {
        "date" => threads.sort_by(|a, b| {
            let a_ts = a.thread_ts.parse::<f64>().unwrap_or(0.0);
            let b_ts = b.thread_ts.parse::<f64>().unwrap_or(0.0);
            b_ts.partial_cmp(&a_ts).unwrap_or(std::cmp::Ordering::Equal)
        }),
        "reactions" => threads.sort_by(|a, b| b.reaction_count.cmp(&a.reaction_count)),
        "replies" => threads.sort_by(|a, b| b.reply_count.cmp(&a.reply_count)),
        _ => {}
    }
}

fn build_user_options(threads: &[ThreadSummary]) -> Vec<String> {
    let mut users: Vec<String> = threads
        .iter()
        .map(|thread| thread.display_name.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    users.sort_by_key(|name| name.to_lowercase());
    users
}

async fn fetch_root_file_summary<R: Repository>(
    repo: &R,
    threads: &[ThreadSummary],
) -> Result<RootFileSummary, Error> {
    let mut roots_by_channel: HashMap<String, Vec<String>> = HashMap::new();
    for thread in threads {
        roots_by_channel
            .entry(thread.channel_id.clone())
            .or_default()
            .push(thread.thread_ts.clone());
    }

    let mut file_counts_by_thread: HashMap<(String, String), i64> = HashMap::new();
    let mut total_files: i64 = 0;

    for (channel_id, tss) in roots_by_channel {
        let files = repo
            .get_files_for_messages(&channel_id, &tss)
            .await
            .map_err(|e| Error::from(e.to_string()))?;

        for file in files {
            total_files += 1;
            *file_counts_by_thread
                .entry((channel_id.clone(), file.message_ts))
                .or_insert(0) += 1;
        }
    }

    Ok(RootFileSummary {
        file_counts_by_thread,
        total_files,
    })
}

fn build_channel_stats(threads: &[ThreadSummary]) -> Vec<ApiChannelStat> {
    let mut counts: HashMap<String, i64> = HashMap::new();
    for thread in threads {
        *counts
            .entry(format!("#{}", thread.channel_name))
            .or_insert(0) += 1;
    }

    let total = threads.len() as i64;
    let mut stats: Vec<ApiChannelStat> = counts
        .into_iter()
        .map(|(name, count)| ApiChannelStat {
            name,
            count,
            percentage: if total > 0 { (count * 100) / total } else { 0 },
        })
        .collect();
    stats.sort_by(|a, b| b.count.cmp(&a.count));
    stats
}

fn build_activity_data(threads: &[ThreadSummary]) -> Vec<ApiActivityPoint> {
    let labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut threads_per_day = [0_i64; 7];
    let mut messages_per_day = [0_i64; 7];

    for thread in threads {
        let weekday = timestamp_to_datetime(&thread.thread_ts)
            .weekday()
            .num_days_from_monday() as usize;
        threads_per_day[weekday] += 1;
        messages_per_day[weekday] += thread.reply_count + 1;
    }

    labels
        .iter()
        .enumerate()
        .map(|(idx, label)| ApiActivityPoint {
            date: (*label).to_owned(),
            messages: messages_per_day[idx],
            threads: threads_per_day[idx],
        })
        .collect()
}

fn build_overview_stats(
    threads: &[ThreadSummary],
    total_files: i64,
    changes: &OverviewChanges,
) -> ApiOverviewStats {
    let total_threads = threads.len() as i64;
    let total_messages: i64 = threads.iter().map(|thread| thread.reply_count + 1).sum();
    let total_users = threads
        .iter()
        .map(|thread| thread.display_name.clone())
        .collect::<HashSet<_>>()
        .len() as i64;

    ApiOverviewStats {
        total_messages,
        total_threads,
        total_files,
        total_users,
        messages_change: changes.messages_change,
        threads_change: changes.threads_change,
        files_change: changes.files_change,
        users_change: changes.users_change,
    }
}

async fn compute_overview_changes<R: Repository>(
    repo: &R,
    tab: &str,
    period: &str,
    user: &str,
    channel: &str,
    search: &str,
) -> Result<OverviewChanges, Error> {
    let Some(window_days) = change_window_days(tab, period) else {
        return Ok(OverviewChanges::default());
    };

    let mut baseline_threads = crate::api::record::cached_threads(repo)
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    apply_channel_and_search_filters(&mut baseline_threads, channel, search);
    apply_user_filter(&mut baseline_threads, user);

    let file_summary = fetch_root_file_summary(repo, &baseline_threads).await?;
    let now = Utc::now().timestamp() as f64;
    let current_start = now - (window_days as f64 * 86_400.0);
    let previous_start = now - ((window_days * 2) as f64 * 86_400.0);

    let mut current = MetricAccumulator::default();
    let mut previous = MetricAccumulator::default();

    for thread in &baseline_threads {
        let thread_ts = thread.thread_ts.parse::<f64>().unwrap_or(0.0);
        let file_count = file_summary
            .file_counts_by_thread
            .get(&(thread.channel_id.clone(), thread.thread_ts.clone()))
            .copied()
            .unwrap_or(0);

        if thread_ts >= current_start {
            current.add(thread, file_count);
        } else if thread_ts >= previous_start {
            previous.add(thread, file_count);
        }
    }

    let current_users = current.users.len() as i64;
    let previous_users = previous.users.len() as i64;

    Ok(OverviewChanges {
        messages_change: percentage_change(current.messages, previous.messages),
        threads_change: percentage_change(current.threads, previous.threads),
        files_change: percentage_change(current.files, previous.files),
        users_change: percentage_change(current_users, previous_users),
    })
}

fn change_window_days(tab: &str, period: &str) -> Option<i64> {
    if period == "7d" {
        return Some(7);
    }
    if period == "30d" {
        return Some(30);
    }

    match tab {
        "week" => Some(7),
        "month" => Some(30),
        "top" => Some(30),
        _ => None,
    }
}

fn percentage_change(current: i64, previous: i64) -> f64 {
    if previous == 0 {
        return if current == 0 { 0.0 } else { 100.0 };
    }
    let delta = ((current - previous) as f64 / previous as f64) * 100.0;
    (delta * 10.0).round() / 10.0
}

fn extract_first_url(text: &str) -> Option<String> {
    text.split_whitespace().find_map(|token| {
        if token.starts_with("http://") || token.starts_with("https://") {
            return Some(
                token
                    .trim_end_matches(is_trailing_url_punctuation)
                    .to_owned(),
            );
        }
        None
    })
}

fn is_trailing_url_punctuation(c: char) -> bool {
    matches!(c, ')' | ']' | '}' | ',' | '.' | ';' | '!' | '?')
}

fn format_day_date(ts: &str) -> String {
    timestamp_to_datetime(ts).format("%d %b %Y").to_string()
}

fn format_time_24h(ts: &str) -> String {
    timestamp_to_datetime(ts).format("%d %b %Y %H:%M").to_string()
}

fn format_time_iso(ts: &str) -> String {
    timestamp_to_datetime(ts).to_rfc3339()
}

fn timestamp_to_datetime(ts: &str) -> chrono::DateTime<Utc> {
    let secs = ts
        .split('.')
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    chrono::DateTime::from_timestamp(secs, 0).unwrap_or(chrono::DateTime::UNIX_EPOCH)
}

fn reaction_total(reactions: &Value) -> i64 {
    reactions
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("count").and_then(Value::as_i64))
                .sum()
        })
        .unwrap_or(0)
}

fn initials(name: &str) -> String {
    let initials: String = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect();
    if initials.is_empty() {
        "?".to_owned()
    } else {
        initials.to_uppercase()
    }
}

fn group_files_by_ts(files: Vec<FileRow>) -> HashMap<String, Vec<FileRow>> {
    let mut map: HashMap<String, Vec<FileRow>> = HashMap::new();
    for file in files {
        map.entry(file.message_ts.clone()).or_default().push(file);
    }
    map
}

fn json_ok<T: Serialize>(value: &T) -> Result<Response<Bytes>, Error> {
    let json = serde_json::to_vec(value).map_err(|e| Error::from(e.to_string()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Access-Control-Allow-Origin", "*")
        .body(Bytes::from(json))?)
}

fn error_response(status: StatusCode, msg: &str) -> Result<Response<Bytes>, Error> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("Access-Control-Allow-Origin", "*")
        .body(Bytes::from(msg.to_owned()))?)
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.to_owned();
            let value = percent_decode(parts.next().unwrap_or(""));
            if key.is_empty() {
                None
            } else {
                Some((key, value))
            }
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
