use std::collections::HashSet;

use reqwest::StatusCode;
use serde::Deserialize;
use thiserror::Error;
use tracing::{info, warn};

use crate::storage::R2Client;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SlackError {
    #[error("rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },
    #[error("slack API error: {0}")]
    Api(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
    pub is_member: bool,
}

/// A single message as returned by the Slack API.
/// `raw` holds the full JSON for upsert; `ts` and `thread_ts` are extracted
/// upfront so callers can decide whether to fetch thread replies.
#[derive(Debug, Clone)]
pub struct SlackMessage {
    pub ts: String,
    pub thread_ts: Option<String>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct SlackUser {
    pub user_id: String,
    pub team_id: String,
    pub display_name: String,
    pub avatar_url: String,
}

// ── Trait ─────────────────────────────────────────────────────────────────────

#[allow(async_fn_in_trait)]
pub trait SlackApi {
    async fn conversations_list(
        &self,
        cursor: Option<&str>,
    ) -> Result<(Vec<Channel>, Option<String>), SlackError>;

    async fn conversations_history(
        &self,
        channel_id: &str,
        oldest: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<(Vec<SlackMessage>, Option<String>), SlackError>;

    async fn conversations_replies(
        &self,
        channel_id: &str,
        ts: &str,
        cursor: Option<&str>,
    ) -> Result<(Vec<SlackMessage>, Option<String>), SlackError>;

    async fn users_list(
        &self,
        cursor: Option<&str>,
    ) -> Result<(Vec<SlackUser>, Option<String>), SlackError>;
}

// ── Client ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SlackClient {
    bot_token: String,
    http: reqwest::Client,
    base_url: String,
}

impl SlackClient {
    pub fn new(bot_token: impl Into<String>) -> Self {
        Self::with_base_url(bot_token, "https://slack.com/api")
    }

    pub fn with_base_url(bot_token: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
            http: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    async fn get(
        &self,
        method: &str,
        params: &[(&str, &str)],
    ) -> Result<serde_json::Value, SlackError> {
        let url = format!("{}/{}", self.base_url, method);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.bot_token)
            .query(params)
            .send()
            .await?;

        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);
            return Err(SlackError::RateLimited { retry_after });
        }

        let json: serde_json::Value = resp.json().await?;
        if json["ok"].as_bool() != Some(true) {
            let err = json["error"].as_str().unwrap_or("unknown_error").to_owned();
            return Err(SlackError::Api(err));
        }
        Ok(json)
    }
}

impl SlackApi for SlackClient {
    async fn conversations_list(
        &self,
        cursor: Option<&str>,
    ) -> Result<(Vec<Channel>, Option<String>), SlackError> {
        let mut params: Vec<(&str, &str)> = vec![
            ("exclude_archived", "false"),
            ("types", "public_channel"),
            ("limit", "200"),
        ];
        if let Some(c) = cursor {
            params.push(("cursor", c));
        }
        let json = self.get("conversations.list", &params).await?;
        let channels: Vec<Channel> = serde_json::from_value(json["channels"].clone())?;
        Ok((channels, extract_cursor(&json)))
    }

    async fn conversations_history(
        &self,
        channel_id: &str,
        oldest: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<(Vec<SlackMessage>, Option<String>), SlackError> {
        let mut params: Vec<(&str, &str)> = vec![("channel", channel_id), ("limit", "200")];
        if let Some(o) = oldest {
            params.push(("oldest", o));
        }
        if let Some(c) = cursor {
            params.push(("cursor", c));
        }
        let json = self.get("conversations.history", &params).await?;
        let messages = parse_messages(&json["messages"])?;
        Ok((messages, extract_cursor(&json)))
    }

    async fn conversations_replies(
        &self,
        channel_id: &str,
        ts: &str,
        cursor: Option<&str>,
    ) -> Result<(Vec<SlackMessage>, Option<String>), SlackError> {
        let mut params: Vec<(&str, &str)> =
            vec![("channel", channel_id), ("ts", ts), ("limit", "200")];
        if let Some(c) = cursor {
            params.push(("cursor", c));
        }
        let json = self.get("conversations.replies", &params).await?;
        let messages = parse_messages(&json["messages"])?;
        Ok((messages, extract_cursor(&json)))
    }

    async fn users_list(
        &self,
        cursor: Option<&str>,
    ) -> Result<(Vec<SlackUser>, Option<String>), SlackError> {
        let mut params: Vec<(&str, &str)> = vec![("limit", "200")];
        if let Some(c) = cursor {
            params.push(("cursor", c));
        }
        let json = self.get("users.list", &params).await?;
        let users = parse_users(&json["members"])?;
        Ok((users, extract_cursor(&json)))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn extract_cursor(json: &serde_json::Value) -> Option<String> {
    json["response_metadata"]["next_cursor"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn parse_users(value: &serde_json::Value) -> Result<Vec<SlackUser>, SlackError> {
    let arr = value
        .as_array()
        .ok_or_else(|| SlackError::Api("members field missing or not an array".to_owned()))?;
    Ok(arr
        .iter()
        .filter(|u| u["is_bot"].as_bool() != Some(true) && u["id"].as_str() != Some("USLACKBOT"))
        .filter_map(|u| {
            let user_id = u["id"].as_str()?.to_owned();
            let team_id = u["team_id"].as_str().unwrap_or("").to_owned();
            let profile = &u["profile"];
            let display_name = profile["display_name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or_else(|| profile["real_name"].as_str())
                .unwrap_or(&user_id)
                .to_owned();
            let avatar_url = profile["image_72"].as_str().unwrap_or("").to_owned();
            Some(SlackUser {
                user_id,
                team_id,
                display_name,
                avatar_url,
            })
        })
        .collect())
}

// ── Backfill orchestration ────────────────────────────────────────────────────

#[derive(Debug, Default, Clone, Copy)]
struct BackfillRunStats {
    channels_discovered: usize,
    channels_processed: usize,
    channels_skipped: usize,
    history_batches: usize,
    history_messages: usize,
    reply_batches: usize,
    reply_messages: usize,
    stale_threads: usize,
    touched_threads: usize,
    weekly_score_upserts: usize,
    aggregation_jobs_enqueued: usize,
    message_upserts: usize,
}

impl BackfillRunStats {
    fn include_channel(&mut self, stats: ChannelBackfillStats) {
        self.history_batches += stats.history_batches;
        self.history_messages += stats.history_messages;
        self.reply_batches += stats.reply_batches;
        self.reply_messages += stats.reply_messages;
        self.stale_threads += stats.stale_threads;
        self.touched_threads += stats.touched_threads;
        self.weekly_score_upserts += stats.weekly_score_upserts;
        self.aggregation_jobs_enqueued += stats.aggregation_jobs_enqueued;
        self.message_upserts += stats.message_upserts;
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ChannelBackfillStats {
    history_batches: usize,
    history_messages: usize,
    reply_batches: usize,
    reply_messages: usize,
    stale_threads: usize,
    touched_threads: usize,
    weekly_score_upserts: usize,
    aggregation_jobs_enqueued: usize,
    message_upserts: usize,
}

impl ChannelBackfillStats {
    fn include_reply(&mut self, stats: ReplyBackfillStats) {
        self.reply_batches += stats.batches;
        self.reply_messages += stats.messages;
        self.message_upserts += stats.messages;
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ReplyBackfillStats {
    batches: usize,
    messages: usize,
}

/// Backfills all channels the bot can see.
///
/// For each channel:
/// 1. Query the DB for the latest archived `ts` (resume point).
/// 2. Fetch `conversations.history` from that point, upsert every message.
/// 3. For every thread-parent message (`thread_ts == ts`), fetch
///    `conversations.replies` and upsert all replies.
pub async fn run_backfill<R, S>(
    repo: &R,
    client: &S,
    slack_token: &str,
    storage: Option<&R2Client>,
) -> anyhow::Result<()>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    let mut cursor: Option<String> = None;
    let mut all_channels: Vec<Channel> = Vec::new();
    loop {
        let (channels, next) = client.conversations_list(cursor.as_deref()).await?;
        all_channels.extend(channels);
        match next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }

    info!(total = all_channels.len(), "channels discovered");
    let mut stats = BackfillRunStats {
        channels_discovered: all_channels.len(),
        ..BackfillRunStats::default()
    };

    for ch in &all_channels {
        info!(channel_id = %ch.id, channel_name = %ch.name, "backfilling channel");
        match backfill_channel(repo, client, slack_token, storage, &ch.id).await? {
            Some(channel_stats) => {
                stats.channels_processed += 1;
                stats.include_channel(channel_stats);
            }
            None => {
                stats.channels_skipped += 1;
            }
        }
    }

    let cached_channels = cache_channels(repo, &all_channels).await?;
    let cached_users = cache_users(repo, client).await?;

    info!(
        channels_discovered = stats.channels_discovered,
        channels_processed = stats.channels_processed,
        channels_skipped = stats.channels_skipped,
        history_batches = stats.history_batches,
        history_messages = stats.history_messages,
        reply_batches = stats.reply_batches,
        reply_messages = stats.reply_messages,
        message_upserts = stats.message_upserts,
        stale_threads = stats.stale_threads,
        touched_threads = stats.touched_threads,
        weekly_score_upserts = stats.weekly_score_upserts,
        aggregation_jobs_enqueued = stats.aggregation_jobs_enqueued,
        channels_cached = cached_channels,
        users_cached = cached_users,
        "backfill complete"
    );
    if stats.message_upserts == 0 {
        warn!(
            channels_discovered = stats.channels_discovered,
            channels_processed = stats.channels_processed,
            "backfill completed with zero message upserts"
        );
    }
    Ok(())
}

async fn backfill_channel<R, S>(
    repo: &R,
    client: &S,
    slack_token: &str,
    storage: Option<&R2Client>,
    channel_id: &str,
) -> anyhow::Result<Option<ChannelBackfillStats>>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    let oldest = repo.get_last_archived_ts(channel_id).await?;
    info!(
        channel_id,
        oldest_ts = oldest.as_deref().unwrap_or("<none>"),
        "starting channel backfill window"
    );
    let mut stats = ChannelBackfillStats::default();
    let mut cursor: Option<String> = None;
    // Threads whose root was archived in a previous run — replies appear in
    // history but the root won't be re-fetched via conversations.history.
    let mut stale_threads: HashSet<String> = HashSet::new();
    let mut touched_threads: HashSet<String> = HashSet::new();
    loop {
        let (messages, next) = match client
            .conversations_history(channel_id, oldest.as_deref(), cursor.as_deref())
            .await
        {
            Ok(r) => r,
            Err(SlackError::Api(ref e)) if e == "not_in_channel" => {
                warn!(channel_id, "skipping private channel: bot not a member");
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };
        stats.history_batches += 1;
        stats.history_messages += messages.len();
        info!(channel_id, count = messages.len(), "fetched message batch");
        for msg in &messages {
            upsert_slack_message(repo, channel_id, msg).await?;
            stats.message_upserts += 1;
            download_and_archive_files(repo, storage, slack_token, channel_id, msg).await?;
            touched_threads.insert(msg.thread_ts.clone().unwrap_or_else(|| msg.ts.clone()));
            if msg.thread_ts.as_deref() == Some(msg.ts.as_str()) {
                // Thread root in this batch — fetch all replies.
                info!(channel_id, thread_ts = %msg.ts, "fetching thread replies");
                let reply_stats = backfill_replies(
                    repo,
                    client,
                    slack_token,
                    storage,
                    channel_id,
                    &msg.ts,
                    &mut touched_threads,
                )
                .await?;
                stats.include_reply(reply_stats);
                stale_threads.remove(&msg.ts);
            } else if let Some(tts) = &msg.thread_ts {
                // Reply whose root was archived in a previous run.
                stale_threads.insert(tts.clone());
            }
        }
        match next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    // Re-fetch threads whose root predates this backfill window.
    stats.stale_threads = stale_threads.len();
    for thread_ts in stale_threads {
        info!(channel_id, %thread_ts, "backfilling stale thread");
        let reply_stats = backfill_replies(
            repo,
            client,
            slack_token,
            storage,
            channel_id,
            &thread_ts,
            &mut touched_threads,
        )
        .await?;
        stats.include_reply(reply_stats);
    }

    stats.touched_threads = touched_threads.len();
    for thread_ts in touched_threads {
        repo.upsert_thread_weekly_score(channel_id, &thread_ts)
            .await?;
        stats.weekly_score_upserts += 1;
        repo.enqueue_thread_aggregation(channel_id, &thread_ts, "backfill")
            .await?;
        stats.aggregation_jobs_enqueued += 1;
    }

    info!(
        channel_id,
        history_batches = stats.history_batches,
        history_messages = stats.history_messages,
        reply_batches = stats.reply_batches,
        reply_messages = stats.reply_messages,
        message_upserts = stats.message_upserts,
        stale_threads = stats.stale_threads,
        touched_threads = stats.touched_threads,
        weekly_score_upserts = stats.weekly_score_upserts,
        aggregation_jobs_enqueued = stats.aggregation_jobs_enqueued,
        "channel backfill complete"
    );

    Ok(Some(stats))
}

async fn backfill_replies<R, S>(
    repo: &R,
    client: &S,
    slack_token: &str,
    storage: Option<&R2Client>,
    channel_id: &str,
    thread_ts: &str,
    touched_threads: &mut HashSet<String>,
) -> anyhow::Result<ReplyBackfillStats>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    let mut stats = ReplyBackfillStats::default();
    let mut cursor: Option<String> = None;
    loop {
        let (messages, next) = client
            .conversations_replies(channel_id, thread_ts, cursor.as_deref())
            .await?;
        stats.batches += 1;
        stats.messages += messages.len();
        info!(
            channel_id,
            thread_ts,
            count = messages.len(),
            "fetched thread replies batch"
        );
        for msg in &messages {
            upsert_slack_message(repo, channel_id, msg).await?;
            download_and_archive_files(repo, storage, slack_token, channel_id, msg).await?;
            touched_threads.insert(msg.thread_ts.clone().unwrap_or_else(|| msg.ts.clone()));
        }
        match next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    Ok(stats)
}

async fn upsert_slack_message<R: crate::db::Repository>(
    repo: &R,
    channel_id: &str,
    msg: &SlackMessage,
) -> anyhow::Result<()> {
    use crate::db::MessageRecord;
    repo.upsert_message(&MessageRecord {
        team_id: msg.raw["team"].as_str().unwrap_or("").to_owned(),
        channel_id: channel_id.to_owned(),
        ts: msg.ts.clone(),
        thread_ts: msg.thread_ts.clone(),
        user_id: msg.raw["user"].as_str().map(str::to_owned),
        text: msg.raw["text"].as_str().unwrap_or("").to_owned(),
        subtype: msg.raw["subtype"].as_str().map(str::to_owned),
        edited_ts: msg.raw["edited"]["ts"].as_str().map(str::to_owned),
        deleted: false,
        raw_json: msg.raw.clone(),
    })
    .await?;
    Ok(())
}

async fn download_and_archive_files<R: crate::db::Repository>(
    repo: &R,
    storage: Option<&R2Client>,
    slack_token: &str,
    channel_id: &str,
    msg: &SlackMessage,
) -> anyhow::Result<()> {
    let team_id = msg.raw["team"].as_str().unwrap_or("");
    let files_raw = msg.raw["files"].clone();
    archive_files(
        repo,
        storage,
        slack_token,
        channel_id,
        &msg.ts,
        team_id,
        &files_raw,
    )
    .await
}

pub async fn archive_files<R: crate::db::Repository>(
    repo: &R,
    storage: Option<&R2Client>,
    slack_token: &str,
    channel_id: &str,
    message_ts: &str,
    team_id: &str,
    files_value: &serde_json::Value,
) -> anyhow::Result<()> {
    let storage = match storage {
        Some(s) => s,
        None => return Ok(()),
    };
    let files = match files_value.as_array() {
        Some(f) if !f.is_empty() => f.clone(),
        _ => return Ok(()),
    };
    let http = reqwest::Client::new();

    for file in &files {
        let file_id = match file["id"].as_str() {
            Some(id) => id,
            None => continue,
        };
        if repo.file_exists(file_id).await? {
            continue;
        }
        let url_private = match file["url_private"].as_str() {
            Some(u) => u,
            None => continue,
        };
        let name = file["name"].as_str().unwrap_or("file");
        let mimetype = file["mimetype"]
            .as_str()
            .unwrap_or("application/octet-stream");
        let size_bytes = file["size"].as_i64().unwrap_or(0);

        let resp = http
            .get(url_private)
            .bearer_auth(slack_token)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await;

        let response = match resp {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                warn!(file_id, status = %r.status(), "failed to download file from Slack");
                continue;
            }
            Err(e) => {
                warn!(file_id, error = %e, "error downloading file from Slack");
                continue;
            }
        };

        // Slack redirects to an HTML login page when the token lacks files:read
        // scope or the file has expired. reqwest follows the redirect and returns
        // 200 OK with HTML — detect and skip this case.
        let resp_content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        if resp_content_type.starts_with("text/html") && !mimetype.starts_with("text/") {
            warn!(
                file_id,
                name,
                expected_mime = mimetype,
                "Slack returned HTML instead of file — token may lack files:read scope or file has expired; skipping"
            );
            continue;
        }

        let data = response.bytes().await?;

        let storage_key = format!("{team_id}/{channel_id}/{file_id}/{name}");
        let storage_url = match storage.upload(&storage_key, data, mimetype).await {
            Ok(u) => u,
            Err(e) => {
                warn!(file_id, error = %e, "failed to upload file to R2");
                continue;
            }
        };

        repo.insert_file(&crate::db::FileRecord {
            file_id: file_id.to_owned(),
            team_id: team_id.to_owned(),
            channel_id: channel_id.to_owned(),
            message_ts: message_ts.to_owned(),
            name: name.to_owned(),
            mimetype: mimetype.to_owned(),
            size_bytes,
            storage_key,
            storage_url,
        })
        .await?;

        info!(file_id, name, "archived file to R2");
    }
    Ok(())
}

async fn cache_channels<R: crate::db::Repository>(
    repo: &R,
    channels: &[Channel],
) -> anyhow::Result<usize> {
    use crate::db::ChannelRecord;
    for ch in channels {
        repo.upsert_channel(&ChannelRecord {
            channel_id: ch.id.clone(),
            team_id: String::new(),
            name: ch.name.clone(),
        })
        .await?;
    }
    info!(count = channels.len(), "channels cached");
    Ok(channels.len())
}

async fn cache_users<R, S>(repo: &R, client: &S) -> anyhow::Result<usize>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    use crate::db::UserRecord;
    let mut cursor: Option<String> = None;
    let mut total = 0usize;
    loop {
        let (users, next) = match client.users_list(cursor.as_deref()).await {
            Ok(r) => r,
            Err(SlackError::Api(ref e)) if e == "missing_scope" => {
                warn!(
                    "users.list requires users:read scope — skipping user cache (add scope and re-run backfill)"
                );
                return Ok(0);
            }
            Err(e) => return Err(e.into()),
        };
        total += users.len();
        for u in users {
            repo.upsert_user(&UserRecord {
                user_id: u.user_id,
                team_id: u.team_id,
                display_name: u.display_name,
                avatar_url: u.avatar_url,
            })
            .await?;
        }
        match next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    info!(total, "users cached");
    Ok(total)
}

fn parse_messages(value: &serde_json::Value) -> Result<Vec<SlackMessage>, SlackError> {
    let arr = value
        .as_array()
        .ok_or_else(|| SlackError::Api("messages field missing or not an array".to_owned()))?;
    arr.iter()
        .map(|m| {
            let ts = m["ts"]
                .as_str()
                .ok_or_else(|| SlackError::Api("message missing ts".to_owned()))?
                .to_owned();
            let thread_ts = m["thread_ts"].as_str().map(str::to_owned);
            Ok(SlackMessage {
                ts,
                thread_ts,
                raw: m.clone(),
            })
        })
        .collect()
}
