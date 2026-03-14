use crate::ai::{
    GeneratedSummaryResult, OpenRouterConfig, compact_text, normalize_status, normalize_text,
    normalize_topic_tags,
};
use db::ThreadSummaryRow;
use domain::Message;
use serde::Deserialize;
use serde_json::json;
use std::time::{Duration, Instant};

const MAX_PROMPT_MESSAGES: usize = 40;
const MAX_MESSAGE_CHARS: usize = 400;
const OPENROUTER_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);

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
    why_it_mattered: Option<String>,
    status: Option<String>,
    topic_tags: Option<Vec<String>>,
}

pub(crate) async fn request_summary(
    config: &ReadyOpenRouterConfig,
    summary: &ThreadSummaryRow,
    messages: &[Message],
) -> Result<GeneratedSummaryResult, AiSummaryError> {
    let endpoint = chat_completions_url(&config.api_base_url);
    let prompt = build_prompt(summary, messages);
    let request_started_at = Instant::now();
    tracing::info!(
        channel_id = %summary.channel_id,
        root_ts = %summary.root_ts,
        model = %config.model,
        endpoint = %endpoint,
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
                    "content": "You summarize archived public Slack messages for later retrieval. Return concise factual JSON only. Keep references grounded in the provided messages. Write in the main language used by the messages. When mentioning a user, preserve Slack mention syntax like <@U123>."
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
    let generated =
        serde_json::from_str::<GeneratedSummaryPayload>(&repaired_content).map_err(|error| {
        tracing::warn!(?error, raw_content = %content, "failed to parse generated summary payload");
        AiSummaryError::InvalidPayload
    })?;
    let normalized_summary =
        normalize_text(&generated.summary).ok_or(AiSummaryError::MissingSummary)?;
    tracing::info!(
        channel_id = %summary.channel_id,
        root_ts = %summary.root_ts,
        status = generated.status.as_deref().unwrap_or("discussion"),
        topic_tag_count = generated.topic_tags.as_ref().map_or(0, Vec::len),
        elapsed_ms = request_started_at.elapsed().as_millis() as u64,
        "received OpenRouter thread summary response"
    );

    Ok(GeneratedSummaryResult {
        summary: normalized_summary,
        why_it_mattered: generated
            .why_it_mattered
            .and_then(|value| normalize_text(&value)),
        status: normalize_status(generated.status.as_deref()),
        topic_tags: normalize_topic_tags(generated.topic_tags.unwrap_or_default()),
    })
}

pub(crate) fn chat_completions_url(api_base_url: &str) -> String {
    let normalized = api_base_url.trim().trim_end_matches('/');
    if normalized.ends_with("/chat/completions") {
        return normalized.to_owned();
    }

    format!("{normalized}/chat/completions")
}

fn build_prompt(summary: &ThreadSummaryRow, messages: &[Message]) -> String {
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

    format!(
        "Summarize the following messages in the main language used by the messages in 2 to 3 sentences maximum.\n\
         \n\
         Write natural standalone sentences that read like notes or documentation.\n\
         Do not refer to the conversation itself (avoid phrases like \"this thread\", \"the discussion\", or \"someone asked\").\n\
         Start directly with the topic, tool, or outcome rather than the people.\n\
         \n\
         Return a JSON object with keys: summary, why_it_mattered, status, topic_tags.\n\
         Rules:\n\
         - summary: 2 to 3 concise factual sentences describing the topic, key points, and outcome\n\
         - merge related ideas instead of retelling the conversation chronologically\n\
         - prefer topic-first phrasing\n\
         - preserve user mentions in Slack format like <@U123> when referring to users\n\
         - do not invent facts, users, or outcomes not present in the messages\n\
         - why_it_mattered: one concise sentence describing the practical takeaway, workflow, recommendation, or decision; use null if unclear\n\
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
        compact_text(&summary.title, MAX_MESSAGE_CHARS),
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
}

#[cfg(test)]
mod parser_tests {
    use super::{
        GeneratedSummaryPayload, normalize_generated_payload_key, repair_generated_summary_payload,
    };

    #[test]
    fn repair_generated_summary_payload_normalizes_common_bad_keys() {
        let raw = r#"{
  "summary: ": "A concise summary.",
  "why_it_mattered": null,
  "status": "unresolved",
  "topic_tags": ["ollama", "configuration"]
}"#;

        let repaired = repair_generated_summary_payload(raw);
        let payload = serde_json::from_str::<GeneratedSummaryPayload>(&repaired)
            .expect("payload should parse");

        assert_eq!(payload.summary, "A concise summary.");
        assert_eq!(payload.status.as_deref(), Some("unresolved"));
        assert_eq!(
            payload.topic_tags.expect("topic tags"),
            vec!["ollama".to_owned(), "configuration".to_owned()]
        );
    }

    #[test]
    fn normalize_generated_payload_key_strips_trailing_colons() {
        assert_eq!(normalize_generated_payload_key("summary: "), "summary");
        assert_eq!(
            normalize_generated_payload_key("why-it-mattered:"),
            "why_it_mattered"
        );
    }
}
