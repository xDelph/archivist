use crate::{user_privacy::ANONYMOUS_DISPLAY_NAME, user_store::SyncedUserRecord};
use domain::Channel;
use html_escape::decode_html_entities as decode_entities;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

const MENTION_PATTERN: &str = r"<(?:(?:@(?<user>[A-Z0-9]+)(?:\|(?<user_label>[^>]+))?)|(?:#(?<channel>[A-Z0-9]+)(?:\|(?<channel_label>[^>]+))?)|(?:!(?<special>here|channel|everyone))|(?:!subteam\^[^|>]+(?:\|(?<subteam>[^>]+))?))>";

pub(crate) fn build_channel_name_map(channels: &[Channel]) -> HashMap<String, Option<String>> {
    channels
        .iter()
        .map(|channel| (channel.id.clone(), channel.name.clone()))
        .collect()
}

pub(crate) fn collect_user_mention_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> HashSet<String> {
    values
        .into_iter()
        .flat_map(|value| mention_captures(value).filter_map(|capture| capture.user))
        .collect()
}

pub(crate) fn collect_channel_mention_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> HashSet<String> {
    values
        .into_iter()
        .flat_map(|value| mention_captures(value).filter_map(|capture| capture.channel))
        .collect()
}

pub(crate) fn render_slack_text(
    value: &str,
    users: &HashMap<String, SyncedUserRecord>,
    channels: &HashMap<String, Option<String>>,
) -> String {
    let decoded = decode_html_entities(value);
    mention_pattern()
        .replace_all(&decoded, |captures: &regex::Captures<'_>| {
            if let Some(user_id) = captures.name("user").map(|value| value.as_str()) {
                if users.get(user_id).is_some_and(|user| user.is_anonymized) {
                    return format!("@{ANONYMOUS_DISPLAY_NAME}");
                }
                return users
                    .get(user_id)
                    .and_then(|user| user.display_name.as_deref())
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("@{value}"))
                    .or_else(|| {
                        captures
                            .name("user_label")
                            .map(|value| format!("@{}", value.as_str().trim()))
                    })
                    .unwrap_or_else(|| format!("@{user_id}"));
            }
            if let Some(channel_id) = captures.name("channel").map(|value| value.as_str()) {
                return channels
                    .get(channel_id)
                    .and_then(|name| name.as_deref())
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("#{value}"))
                    .or_else(|| {
                        captures
                            .name("channel_label")
                            .map(|value| format!("#{}", value.as_str().trim()))
                    })
                    .unwrap_or_else(|| format!("#{channel_id}"));
            }
            if let Some(special) = captures.name("special").map(|value| value.as_str()) {
                return format!("@{special}");
            }
            if let Some(subteam) = captures.name("subteam").map(|value| value.as_str()) {
                return format!("@{subteam}");
            }

            captures
                .get(0)
                .map(|value| value.as_str().to_owned())
                .unwrap_or_default()
        })
        .into_owned()
}

pub(crate) fn decode_html_entities(value: &str) -> String {
    decode_entities(value).into_owned()
}

struct MentionCapture {
    user: Option<String>,
    channel: Option<String>,
}

fn mention_captures(value: &str) -> impl Iterator<Item = MentionCapture> + '_ {
    mention_pattern()
        .captures_iter(value)
        .map(|captures| MentionCapture {
            user: captures.name("user").map(|value| value.as_str().to_owned()),
            channel: captures
                .name("channel")
                .map(|value| value.as_str().to_owned()),
        })
}

fn mention_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(MENTION_PATTERN).expect("mention pattern should be valid"))
}

#[cfg(test)]
#[path = "slack_text_tests.rs"]
mod tests;
