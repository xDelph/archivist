use crate::ai::{
    GeneratedSummaryResult, OpenRouterConfig, compact_text, normalize_markdown_text,
    normalize_status, normalize_text, normalize_topic_tags,
};
use crate::ai_openrouter_language::{
    LanguageHint, build_language_requirement, detect_thread_language, is_output_language_mismatch,
};
use db::ThreadSummaryRow;
use domain::Message;
use serde::Deserialize;
use serde_json::json;
use std::time::{Duration, Instant};

const MAX_PROMPT_MESSAGES: usize = 40;
const MAX_MESSAGE_CHARS: usize = 400;
const OPENROUTER_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_SUMMARY_LANGUAGE_ATTEMPTS: usize = 2;

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterMessage,
}

#[derive(Debug, Deserialize)]
struct OpenRouterMessage {
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GeneratedSummaryPayload {
    summary: String,
    full_summary: Option<String>,
    why_it_mattered: Option<String>,
    status: Option<String>,
    topic_tags: Option<Vec<String>>,
}

pub(crate) async fn request_summary(
    config: &ReadyOpenRouterConfig,
    summary: &ThreadSummaryRow,
    messages: &[Message],
) -> Result<GeneratedSummaryResult, AiSummaryError> {
    let language_hint = detect_thread_language(summary, messages);
    for attempt in 0..MAX_SUMMARY_LANGUAGE_ATTEMPTS {
        let generated =
            request_summary_payload(config, summary, messages, language_hint, attempt).await?;
        let normalized_summary =
            normalize_text(&generated.summary).ok_or(AiSummaryError::MissingSummary)?;
        let full_summary = generated
            .full_summary
            .as_deref()
            .and_then(normalize_markdown_text)
            .unwrap_or_else(|| normalized_summary.clone());
        let why_it_mattered = generated
            .why_it_mattered
            .and_then(|value| normalize_text(&value));

        if is_output_language_mismatch(
            language_hint,
            &normalized_summary,
            why_it_mattered.as_deref(),
            Some(full_summary.as_str()),
        ) {
            tracing::warn!(
                channel_id = %summary.channel_id,
                root_ts = %summary.root_ts,
                expected_language = language_hint.map_or("unknown", |hint| hint.label),
                attempt = attempt + 1,
                "generated summary language did not match the detected thread language"
            );
            if attempt + 1 < MAX_SUMMARY_LANGUAGE_ATTEMPTS {
                continue;
            }
            return Err(AiSummaryError::LanguageMismatch);
        }

        tracing::info!(
            channel_id = %summary.channel_id,
            root_ts = %summary.root_ts,
            status = generated.status.as_deref().unwrap_or("discussion"),
            topic_tag_count = generated.topic_tags.as_ref().map_or(0, Vec::len),
            detected_language = language_hint.map_or("unknown", |hint| hint.label),
            "received OpenRouter thread summary response"
        );

        return Ok(GeneratedSummaryResult {
            summary: normalized_summary,
            full_summary,
            why_it_mattered,
            status: normalize_status(generated.status.as_deref()),
            topic_tags: normalize_topic_tags(generated.topic_tags.unwrap_or_default()),
        });
    }

    Err(AiSummaryError::LanguageMismatch)
}

async fn request_summary_payload(
    config: &ReadyOpenRouterConfig,
    summary: &ThreadSummaryRow,
    messages: &[Message],
    language_hint: Option<&'static LanguageHint>,
    attempt: usize,
) -> Result<GeneratedSummaryPayload, AiSummaryError> {
    let endpoint = chat_completions_url(&config.api_base_url);
    let prompt = build_prompt(summary, messages, language_hint, attempt > 0);
    let request_started_at = Instant::now();
    tracing::info!(
        channel_id = %summary.channel_id,
        root_ts = %summary.root_ts,
        model = %config.model,
        endpoint = %endpoint,
        detected_language = language_hint.map_or("unknown", |hint| hint.label),
        attempt = attempt + 1,
        prompt_chars = prompt.len(),
        timeout_seconds = OPENROUTER_REQUEST_TIMEOUT.as_secs(),
        "sending OpenRouter thread summary request"
    );
    let response = reqwest::Client::builder()
        .timeout(OPENROUTER_REQUEST_TIMEOUT)
        .build()
        .expect("OpenRouter HTTP client should build")
        .post(endpoint)
        .bearer_auth(&config.api_key)
        .json(&json!({
            "model": config.model,
            "temperature": 0.2,
            "response_format": { "type": "json_object" },
            "messages": [
                {
                    "role": "system",
                    "content": "You summarize archived public Slack messages for later retrieval. Return concise factual JSON only. Keep references grounded in the provided messages. Write in the main language used by the messages. Write like neutral editorial or documentation copy, not like a chat recap. Do not mention Slack, threads, channels, messages, replies, or users unless a specific identity is materially necessary to understand the outcome. Avoid openings like \"this thread\", \"the discussion\", \"someone asked\", \"a user said\", or their equivalents in any language. When mentioning a specific user because it is materially necessary, preserve Slack mention syntax like <@U123>."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(
                ?error,
                channel_id = %summary.channel_id,
                root_ts = %summary.root_ts,
                elapsed_ms = request_started_at.elapsed().as_millis() as u64,
                timed_out = error.is_timeout(),
                "openrouter request failed"
            );
            AiSummaryError::Request
        })?;
    let response = response.error_for_status().map_err(|error| {
        tracing::warn!(
            ?error,
            channel_id = %summary.channel_id,
            root_ts = %summary.root_ts,
            elapsed_ms = request_started_at.elapsed().as_millis() as u64,
            "openrouter returned an unsuccessful status"
        );
        AiSummaryError::RequestStatus
    })?;
    let payload = response
        .json::<OpenRouterResponse>()
        .await
        .map_err(|error| {
            tracing::warn!(
                ?error,
                channel_id = %summary.channel_id,
                root_ts = %summary.root_ts,
                elapsed_ms = request_started_at.elapsed().as_millis() as u64,
                "failed to decode openrouter response body"
            );
            AiSummaryError::Response
        })?;
    let content = extract_content(payload.choices.first()).ok_or(AiSummaryError::MissingContent)?;
    let repaired_content = repair_generated_summary_payload(&content);
    serde_json::from_str::<GeneratedSummaryPayload>(&repaired_content).map_err(|error| {
        tracing::warn!(?error, raw_content = %content, "failed to parse generated summary payload");
        AiSummaryError::InvalidPayload
    })
}

pub(crate) fn chat_completions_url(api_base_url: &str) -> String {
    let normalized = api_base_url.trim().trim_end_matches('/');
    if normalized.ends_with("/chat/completions") {
        return normalized.to_owned();
    }

    format!("{normalized}/chat/completions")
}

fn build_prompt(
    summary: &ThreadSummaryRow,
    messages: &[Message],
    language_hint: Option<&'static LanguageHint>,
    retrying_after_language_mismatch: bool,
) -> String {
    let transcript = messages
        .iter()
        .take(MAX_PROMPT_MESSAGES)
        .map(|message| {
            format!(
                "- ts={} user={} text={}",
                message.ts,
                message.user_id.as_deref().unwrap_or("unknown"),
                compact_text(&message.text, MAX_MESSAGE_CHARS)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let root_text = messages
        .iter()
        .find(|message| message.ts == summary.root_ts)
        .map(|message| compact_text(&message.text, MAX_MESSAGE_CHARS))
        .unwrap_or_else(|| "(no text)".to_owned());
    let language_requirement =
        build_language_requirement(language_hint, retrying_after_language_mismatch);

    format!(
        "Summarize the following messages with a short preview summary and a fuller markdown recap.\n\
         \n\
         Write natural standalone sentences that read like neutral notes, release notes, or blog text.\n\
         Do not refer to the source conversation or medium itself.\n\
         Avoid phrases like \"this thread\", \"the discussion\", \"someone asked\", \"a user said\", \"in Slack\", \"in the channel\", or equivalents in any language.\n\
         Start directly with the topic, tool, decision, or outcome rather than the people or the conversation.\n\
         Keep the text generic and self-contained, as if it could be published outside the chat context.\n\
         {language_requirement}\
         \n\
         Return a JSON object with keys: summary, full_summary, why_it_mattered, status, topic_tags.\n\
         Rules:\n\
         - summary: 1 to 2 concise factual sentences optimized for a card preview; keep it tighter than the full recap and aim for roughly 220 characters when possible\n\
         - full_summary: a markdown summary of the thread covering the important context, key points, decisions, and next steps without a hard length cap; keep it factual and reasonably concise\n\
         - merge related ideas instead of retelling the conversation chronologically\n\
         - prefer topic-first phrasing and outcome-first wording\n\
         - do not mention people unless their identity materially matters to the outcome; when it does, preserve user mentions in Slack format like <@U123>\n\
         - do not invent facts, users, or outcomes not present in the messages\n\
         - summary, full_summary, and why_it_mattered must match the dominant thread language exactly; do not translate them to English unless the thread is in English\n\
         - why_it_mattered: one concise sentence describing the practical takeaway, workflow, recommendation, or decision in the same neutral editorial style; use null if unclear\n\
         - status: one of answered, unresolved, announcement, debate, resource, discussion\n\
         - topic_tags: 2 to 5 short lowercase tags without # describing the main tools, technologies, or concepts someone might search for\n\
         - stay grounded in the provided messages only\n\
         \n\
         Thread metadata:\n\
         channel_id={}\n\
         root_ts={}\n\
         title={}\n\
         reply_count={}\n\
         participant_count={}\n\
         reaction_count={}\n\
         file_count={}\n\
         \n\
        Messages:\n{}",
        summary.channel_id,
        summary.root_ts,
        root_text,
        summary.reply_count,
        summary.participant_count,
        summary.reaction_count,
        summary.file_count,
        transcript
    )
}

fn extract_content(choice: Option<&OpenRouterChoice>) -> Option<String> {
    let content = &choice?.message.content;
    if let Some(text) = content.as_str() {
        return Some(text.to_owned());
    }
    let parts = content.as_array()?;
    let text = parts
        .iter()
        .filter_map(
            |part| match part.get("type").and_then(|value| value.as_str()) {
                Some("text") => part.get("text").and_then(|value| value.as_str()),
                _ => None,
            },
        )
        .collect::<Vec<_>>()
        .join("");
    (!text.is_empty()).then_some(text)
}

fn repair_generated_summary_payload(content: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(content) else {
        return content.to_owned();
    };
    let Some(object) = value.as_object_mut() else {
        return content.to_owned();
    };

    let mut repaired = serde_json::Map::new();
    for (key, value) in object.iter() {
        let normalized_key = normalize_generated_payload_key(key);
        if repaired.contains_key(&normalized_key) {
            continue;
        }
        repaired.insert(normalized_key, value.clone());
    }

    serde_json::Value::Object(repaired).to_string()
}

fn normalize_generated_payload_key(key: &str) -> String {
    key.trim()
        .trim_matches('"')
        .trim()
        .trim_end_matches(':')
        .trim()
        .replace([' ', '-'], "_")
        .to_ascii_lowercase()
}

impl OpenRouterConfig {
    pub(crate) fn ready(&self) -> Option<ReadyOpenRouterConfig> {
        let api_key = self
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())?
            .to_owned();
        let model = self
            .model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())?
            .to_owned();

        Some(ReadyOpenRouterConfig {
            api_base_url: self.api_base_url.trim().to_owned(),
            api_key,
            model,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ReadyOpenRouterConfig {
    pub(crate) api_base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
}

#[derive(Debug)]
pub(crate) enum AiSummaryError {
    Request,
    RequestStatus,
    Response,
    MissingContent,
    InvalidPayload,
    MissingSummary,
    LanguageMismatch,
}

#[cfg(test)]
#[path = "ai_openrouter_tests.rs"]
mod tests;
