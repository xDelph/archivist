use std::collections::HashSet;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
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

    async fn users_info(&self, user_id: &str) -> Result<Option<SlackUser>, SlackError>;
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
        let timeout = StdDuration::from_secs(slack_api_timeout_seconds());
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            bot_token: bot_token.into(),
            http,
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

    async fn users_info(&self, user_id: &str) -> Result<Option<SlackUser>, SlackError> {
        let json = self.get("users.info", &[("user", user_id)]).await?;
        Ok(parse_user(&json["user"]))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn extract_cursor(json: &serde_json::Value) -> Option<String> {
    json["response_metadata"]["next_cursor"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn slack_api_timeout_seconds() -> u64 {
    std::env::var("SLACK_API_TIMEOUT_SECONDS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(20)
}

fn parse_users(value: &serde_json::Value) -> Result<Vec<SlackUser>, SlackError> {
    let arr = value
        .as_array()
        .ok_or_else(|| SlackError::Api("members field missing or not an array".to_owned()))?;
    Ok(arr.iter().filter_map(parse_user).collect())
}

fn parse_user(value: &serde_json::Value) -> Option<SlackUser> {
    if value["is_bot"].as_bool() == Some(true) || value["id"].as_str() == Some("USLACKBOT") {
        return None;
    }

    let user_id = value["id"].as_str()?.to_owned();
    let team_id = value["team_id"].as_str().unwrap_or("").to_owned();
    let profile = &value["profile"];
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

#[derive(Debug, Clone)]
pub struct BackfillSliceConfig {
    pub max_channels: usize,
    pub users_pages_per_slice: usize,
    pub users_sync_interval_minutes: i64,
}

impl Default for BackfillSliceConfig {
    fn default() -> Self {
        Self {
            max_channels: usize::MAX,
            users_pages_per_slice: 2,
            users_sync_interval_minutes: 720,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct BackfillSliceResult {
    pub channels_discovered: usize,
    pub channels_processed: usize,
    pub channels_skipped: usize,
    pub message_upserts: usize,
    pub weekly_score_upserts: usize,
    pub aggregation_jobs_enqueued: usize,
    pub users_pages: usize,
    pub users_cached: usize,
    pub users_sync_active: bool,
    pub next_channel_id: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct BackfillState {
    next_channel_id: Option<String>,
    users_cursor: Option<String>,
    users_synced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Clone)]
struct UserCacheSliceStats {
    pages: usize,
    users_cached: usize,
    next_cursor: Option<String>,
    missing_scope: bool,
}

#[derive(Debug, Default)]
struct UserLookupCache {
    refreshed: HashSet<String>,
    skipped: HashSet<String>,
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
    _slack_token: &str,
    _storage: Option<&R2Client>,
) -> anyhow::Result<()>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    let all_channels = discover_channels(client).await?;
    let mut user_lookup_cache = UserLookupCache::default();

    info!(total = all_channels.len(), "channels discovered");
    let mut stats = BackfillRunStats {
        channels_discovered: all_channels.len(),
        ..BackfillRunStats::default()
    };

    for ch in &all_channels {
        info!(channel_id = %ch.id, channel_name = %ch.name, "backfilling channel");
        match backfill_channel(repo, client, &ch.id, &mut user_lookup_cache).await? {
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
        users_refreshed = user_lookup_cache.refreshed.len(),
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

pub async fn run_backfill_slice<S: SlackApi>(
    pool: &PgPool,
    client: &S,
    _slack_token: &str,
    _storage: Option<&R2Client>,
    config: &BackfillSliceConfig,
) -> anyhow::Result<BackfillSliceResult> {
    let all_channels = discover_channels(client).await?;
    let channels_discovered = all_channels.len();
    cache_channels(pool, &all_channels).await?;

    let mut ordered_ids: Vec<String> = all_channels.iter().map(|ch| ch.id.clone()).collect();
    ordered_ids.sort();

    let mut state = load_backfill_state(pool).await?;
    let (channel_ids, next_channel_id) = select_channel_slice(
        &ordered_ids,
        state.next_channel_id.as_deref(),
        config.max_channels,
    );

    let mut result = BackfillSliceResult {
        channels_discovered,
        next_channel_id,
        ..BackfillSliceResult::default()
    };
    let mut user_lookup_cache = UserLookupCache::default();

    for channel_id in &channel_ids {
        match backfill_channel(pool, client, channel_id, &mut user_lookup_cache).await? {
            Some(channel_stats) => {
                result.channels_processed += 1;
                result.message_upserts += channel_stats.message_upserts;
                result.weekly_score_upserts += channel_stats.weekly_score_upserts;
                result.aggregation_jobs_enqueued += channel_stats.aggregation_jobs_enqueued;
            }
            None => {
                result.channels_skipped += 1;
            }
        }
    }

    let should_sync_users = should_sync_users(
        state.users_cursor.as_deref(),
        state.users_synced_at,
        config.users_sync_interval_minutes,
    );
    if should_sync_users && config.users_pages_per_slice > 0 {
        let user_stats = cache_users_slice(
            pool,
            client,
            state.users_cursor.as_deref(),
            config.users_pages_per_slice,
        )
        .await?;
        result.users_pages = user_stats.pages;
        result.users_cached = user_stats.users_cached;
        result.users_sync_active = user_stats.next_cursor.is_some();
        state.users_cursor = user_stats.next_cursor;
        if !result.users_sync_active || user_stats.missing_scope {
            state.users_synced_at = Some(Utc::now());
        }
    }

    state.next_channel_id = result.next_channel_id.clone();
    store_backfill_state(pool, &state).await?;
    Ok(result)
}

async fn discover_channels<S: SlackApi>(client: &S) -> Result<Vec<Channel>, SlackError> {
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
    Ok(all_channels)
}

fn select_channel_slice(
    ordered_channel_ids: &[String],
    next_channel_id: Option<&str>,
    max_channels: usize,
) -> (Vec<String>, Option<String>) {
    if ordered_channel_ids.is_empty() || max_channels == 0 {
        return (Vec::new(), None);
    }

    let start_idx = next_channel_id
        .and_then(|needle| ordered_channel_ids.iter().position(|id| id == needle))
        .unwrap_or(0);
    let to_take = max_channels.min(ordered_channel_ids.len());

    let mut selected = Vec::with_capacity(to_take);
    for offset in 0..to_take {
        let idx = (start_idx + offset) % ordered_channel_ids.len();
        selected.push(ordered_channel_ids[idx].clone());
    }

    let next_idx = (start_idx + to_take) % ordered_channel_ids.len();
    let next = Some(ordered_channel_ids[next_idx].clone());
    (selected, next)
}

async fn load_backfill_state(pool: &PgPool) -> anyhow::Result<BackfillState> {
    sqlx::query(
        r#"
        INSERT INTO backfill_state (id)
        VALUES (TRUE)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;

    let row = sqlx::query(
        r#"
        SELECT next_channel_id, users_cursor, users_synced_at
        FROM backfill_state
        WHERE id = TRUE
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(BackfillState {
        next_channel_id: row.get("next_channel_id"),
        users_cursor: row.get("users_cursor"),
        users_synced_at: row.get("users_synced_at"),
    })
}

async fn store_backfill_state(pool: &PgPool, state: &BackfillState) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO backfill_state (id, next_channel_id, users_cursor, users_synced_at, updated_at)
        VALUES (TRUE, $1, $2, $3, NOW())
        ON CONFLICT (id)
        DO UPDATE SET
            next_channel_id = EXCLUDED.next_channel_id,
            users_cursor = EXCLUDED.users_cursor,
            users_synced_at = EXCLUDED.users_synced_at,
            updated_at = NOW()
        "#,
    )
    .bind(state.next_channel_id.as_deref())
    .bind(state.users_cursor.as_deref())
    .bind(state.users_synced_at)
    .execute(pool)
    .await?;
    Ok(())
}

fn should_sync_users(
    users_cursor: Option<&str>,
    users_synced_at: Option<DateTime<Utc>>,
    users_sync_interval_minutes: i64,
) -> bool {
    if users_cursor.is_some() {
        return true;
    }
    let interval = users_sync_interval_minutes.max(1);
    match users_synced_at {
        Some(last_sync) => Utc::now() - last_sync >= Duration::minutes(interval),
        None => true,
    }
}

async fn backfill_channel<R, S>(
    repo: &R,
    client: &S,
    channel_id: &str,
    user_lookup_cache: &mut UserLookupCache,
) -> anyhow::Result<Option<ChannelBackfillStats>>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    let oldest = repo.get_last_archived_ts(channel_id).await?;
    let history_oldest = oldest.clone();
    info!(
        channel_id,
        oldest_ts = oldest.as_deref().unwrap_or("<none>"),
        history_oldest_ts = history_oldest.as_deref().unwrap_or("<none>"),
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
            .conversations_history(channel_id, history_oldest.as_deref(), cursor.as_deref())
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
            upsert_slack_message(repo, client, user_lookup_cache, channel_id, msg).await?;
            stats.message_upserts += 1;
            enqueue_message_file_backfill(repo, channel_id, msg).await?;
            touched_threads.insert(msg.thread_ts.clone().unwrap_or_else(|| msg.ts.clone()));
            if msg.thread_ts.as_deref() == Some(msg.ts.as_str()) {
                // Thread root in this batch — fetch all replies.
                info!(channel_id, thread_ts = %msg.ts, "fetching thread replies");
                let reply_stats = backfill_replies(
                    repo,
                    client,
                    channel_id,
                    &msg.ts,
                    &mut touched_threads,
                    user_lookup_cache,
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
            channel_id,
            &thread_ts,
            &mut touched_threads,
            user_lookup_cache,
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
    channel_id: &str,
    thread_ts: &str,
    touched_threads: &mut HashSet<String>,
    user_lookup_cache: &mut UserLookupCache,
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
            upsert_slack_message(repo, client, user_lookup_cache, channel_id, msg).await?;
            enqueue_message_file_backfill(repo, channel_id, msg).await?;
            touched_threads.insert(msg.thread_ts.clone().unwrap_or_else(|| msg.ts.clone()));
        }
        match next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    Ok(stats)
}

async fn upsert_slack_message<R, S>(
    repo: &R,
    client: &S,
    user_lookup_cache: &mut UserLookupCache,
    channel_id: &str,
    msg: &SlackMessage,
) -> anyhow::Result<()>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    hydrate_user_profile(repo, client, msg.raw["user"].as_str(), user_lookup_cache).await?;

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

async fn hydrate_user_profile<R, S>(
    repo: &R,
    client: &S,
    user_id: Option<&str>,
    cache: &mut UserLookupCache,
) -> anyhow::Result<()>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    let Some(user_id) = user_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    if cache.refreshed.contains(user_id) || cache.skipped.contains(user_id) {
        return Ok(());
    }

    match client.users_info(user_id).await {
        Ok(Some(user)) => {
            repo.upsert_user(&crate::db::UserRecord {
                user_id: user.user_id.clone(),
                team_id: user.team_id,
                display_name: user.display_name,
                avatar_url: user.avatar_url,
            })
            .await?;
            cache.refreshed.insert(user.user_id);
        }
        Ok(None) => {
            cache.skipped.insert(user_id.to_owned());
        }
        Err(SlackError::Api(err))
            if err == "users_not_found" || err == "user_not_found" || err == "missing_scope" =>
        {
            if err == "missing_scope" {
                warn!(
                    user_id,
                    "users.info requires users:read scope — skipping user profile hydration"
                );
            }
            cache.skipped.insert(user_id.to_owned());
        }
        Err(err) => {
            warn!(user_id, error = %err, "users.info failed; skipping user profile hydration");
            cache.skipped.insert(user_id.to_owned());
        }
    }

    Ok(())
}

async fn enqueue_message_file_backfill<R: crate::db::Repository>(
    repo: &R,
    channel_id: &str,
    msg: &SlackMessage,
) -> anyhow::Result<()> {
    let team_id = msg.raw["team"].as_str().unwrap_or("");
    let files_raw = msg.raw["files"].clone();
    let has_files = files_raw.as_array().is_some_and(|files| !files.is_empty());
    if !has_files {
        return Ok(());
    }

    repo.enqueue_file_backfill_job(team_id, channel_id, &msg.ts, &files_raw)
        .await?;
    Ok(())
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

pub async fn sync_all_users<R, S>(repo: &R, client: &S) -> anyhow::Result<usize>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    info!("starting users cache");
    let stats = cache_users_slice(repo, client, None, usize::MAX).await?;
    info!(total = stats.users_cached, "users cached");
    Ok(stats.users_cached)
}

async fn cache_users_slice<R, S>(
    repo: &R,
    client: &S,
    initial_cursor: Option<&str>,
    max_pages: usize,
) -> anyhow::Result<UserCacheSliceStats>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    use crate::db::UserRecord;

    if max_pages == 0 {
        return Ok(UserCacheSliceStats::default());
    }

    let mut stats = UserCacheSliceStats::default();
    let mut cursor = initial_cursor.map(str::to_owned);
    for _ in 0..max_pages {
        info!(
            page = stats.pages + 1,
            has_cursor = cursor.is_some(),
            "fetching users.list page"
        );
        let (users, next) = match client.users_list(cursor.as_deref()).await {
            Ok(r) => r,
            Err(SlackError::Api(ref e)) if e == "missing_scope" => {
                warn!(
                    "users.list requires users:read scope — skipping user cache (add scope and re-run backfill)"
                );
                stats.missing_scope = true;
                stats.next_cursor = None;
                return Ok(stats);
            }
            Err(e) => return Err(e.into()),
        };

        let users_in_page = users.len();
        stats.pages += 1;
        stats.users_cached += users_in_page;
        for u in users {
            repo.upsert_user(&UserRecord {
                user_id: u.user_id,
                team_id: u.team_id,
                display_name: u.display_name,
                avatar_url: u.avatar_url,
            })
            .await?;
        }

        let has_next = next.is_some();
        match next {
            Some(next_cursor) => cursor = Some(next_cursor),
            None => {
                cursor = None;
            }
        }
        info!(
            page = stats.pages,
            users_in_page,
            users_cached_total = stats.users_cached,
            has_next,
            "users.list page fetched"
        );
        if !has_next {
            break;
        }
    }

    stats.next_cursor = cursor;
    Ok(stats)
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

#[cfg(test)]
mod slice_tests {
    use super::select_channel_slice;

    #[test]
    fn select_channel_slice_rotates_from_saved_cursor() {
        let channels = vec![
            "C001".to_owned(),
            "C002".to_owned(),
            "C003".to_owned(),
            "C004".to_owned(),
        ];

        let (slice, next) = select_channel_slice(&channels, Some("C003"), 2);
        assert_eq!(slice, vec!["C003".to_owned(), "C004".to_owned()]);
        assert_eq!(next.as_deref(), Some("C001"));
    }

    #[test]
    fn select_channel_slice_handles_missing_cursor() {
        let channels = vec!["C001".to_owned(), "C002".to_owned(), "C003".to_owned()];
        let (slice, next) = select_channel_slice(&channels, Some("does-not-exist"), 2);
        assert_eq!(slice, vec!["C001".to_owned(), "C002".to_owned()]);
        assert_eq!(next.as_deref(), Some("C003"));
    }
}
