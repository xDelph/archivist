use reqwest::StatusCode;
use serde::Deserialize;
use thiserror::Error;

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
            ("types", "public_channel,private_channel"),
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
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn extract_cursor(json: &serde_json::Value) -> Option<String> {
    json["response_metadata"]["next_cursor"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

// ── Backfill orchestration ────────────────────────────────────────────────────

/// Backfills all channels the bot can see.
///
/// For each channel:
/// 1. Query the DB for the latest archived `ts` (resume point).
/// 2. Fetch `conversations.history` from that point, upsert every message.
/// 3. For every thread-parent message (`thread_ts == ts`), fetch
///    `conversations.replies` and upsert all replies.
pub async fn run_backfill<R, S>(repo: &R, client: &S) -> anyhow::Result<()>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    let mut cursor: Option<String> = None;
    loop {
        let (channels, next) = client.conversations_list(cursor.as_deref()).await?;
        for ch in channels {
            backfill_channel(repo, client, &ch.id).await?;
        }
        match next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    Ok(())
}

async fn backfill_channel<R, S>(repo: &R, client: &S, channel_id: &str) -> anyhow::Result<()>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    let oldest = repo.get_last_archived_ts(channel_id).await?;
    let mut cursor: Option<String> = None;
    loop {
        let (messages, next) = match client
            .conversations_history(channel_id, oldest.as_deref(), cursor.as_deref())
            .await
        {
            Ok(r) => r,
            Err(SlackError::Api(ref e)) if e == "not_in_channel" => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        for msg in &messages {
            upsert_slack_message(repo, channel_id, msg).await?;
            if msg.thread_ts.as_deref() == Some(msg.ts.as_str()) {
                backfill_replies(repo, client, channel_id, &msg.ts).await?;
            }
        }
        match next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    Ok(())
}

async fn backfill_replies<R, S>(
    repo: &R,
    client: &S,
    channel_id: &str,
    thread_ts: &str,
) -> anyhow::Result<()>
where
    R: crate::db::Repository,
    S: SlackApi,
{
    let mut cursor: Option<String> = None;
    loop {
        let (messages, next) = client
            .conversations_replies(channel_id, thread_ts, cursor.as_deref())
            .await?;
        for msg in &messages {
            upsert_slack_message(repo, channel_id, msg).await?;
        }
        match next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    Ok(())
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
