use std::collections::HashMap;

use maud::{Markup, PreEscaped};

// ── Public API ────────────────────────────────────────────────────────────────

/// Render Slack mrkdwn: resolves `<@U..>`, `<#C..>`, `<!here>`, `<url|label>`,
/// escapes plain text, converts `\n` to `<br>`. Returns pre-escaped HTML.
pub fn render_slack_text(
    text: &str,
    users: &HashMap<String, String>,
    channels: &HashMap<String, String>,
) -> Markup {
    PreEscaped(render_slack_text_inner(text, users, channels))
}

/// Convenience: render with an empty channel map (for thread card previews).
pub fn render_text_simple(text: &str, users: &HashMap<String, String>) -> Markup {
    let empty = HashMap::new();
    PreEscaped(demojify_str(&render_slack_text_inner(text, users, &empty)))
}

/// Replace `:shortcode:` patterns with Unicode emoji characters.
pub fn demojify(text: &str) -> String {
    demojify_str(text)
}

// ── Internals ─────────────────────────────────────────────────────────────────

fn demojify_str(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;

    while !remaining.is_empty() {
        match remaining.find(':') {
            None => {
                result.push_str(remaining);
                break;
            }
            Some(colon_start) => {
                result.push_str(&remaining[..colon_start]);
                let after_colon = &remaining[colon_start + 1..];

                // Try to find a closing ':' and validate the name between them
                if let Some(colon_end) = after_colon.find(':') {
                    let name = &after_colon[..colon_end];
                    let valid = !name.is_empty()
                        && name.chars().all(|c| {
                            c.is_ascii_lowercase()
                                || c.is_ascii_digit()
                                || matches!(c, '_' | '+' | '-')
                        });

                    if let Some(emoji) = valid.then(|| emojis::get_by_shortcode(name)).flatten() {
                        result.push_str(emoji.as_str());
                        remaining = &after_colon[colon_end + 1..];
                        continue;
                    }
                }

                // Not a valid/matched shortcode — emit ':' and continue
                result.push(':');
                remaining = after_colon;
            }
        }
    }

    result
}

fn render_slack_text_inner(
    text: &str,
    users: &HashMap<String, String>,
    channels: &HashMap<String, String>,
) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' {
            let start = i + 1;
            let mut end = start;
            while end < len && bytes[end] != b'>' {
                end += 1;
            }
            if end < len {
                let inner = &text[start..end];
                out.push_str(&render_token(inner, users, channels));
                i = end + 1;
            } else {
                // No closing '>' — treat literal '<'
                out.push_str("&lt;");
                i += 1;
            }
        } else if bytes[i] == b'\n' {
            out.push_str("<br>");
            i += 1;
        } else {
            // Accumulate plain text until '<' or '\n', then HTML-escape it.
            // Slack pre-encodes &, < and > as &amp;/&lt;/&gt; in message text,
            // so decode those entities first to avoid double-encoding.
            let start = i;
            while i < len && bytes[i] != b'<' && bytes[i] != b'\n' {
                i += 1;
            }
            out.push_str(&escape_html(&decode_slack_entities(&text[start..i])));
        }
    }

    out
}

fn render_token(
    inner: &str,
    users: &HashMap<String, String>,
    channels: &HashMap<String, String>,
) -> String {
    // User mention: @U123 or @U123|name
    if let Some(rest) = inner.strip_prefix('@') {
        let (id, fallback) = split_pipe(rest);
        // Prefer: users map > inline fallback name > raw ID
        let name = users
            .get(id)
            .map(String::as_str)
            .unwrap_or(if fallback != id { fallback } else { id });
        return format!(r#"<span class="mention">@{}</span>"#, escape_html(name));
    }

    // Channel mention: #C123|name
    if let Some(rest) = inner.strip_prefix('#') {
        let (id, fallback) = split_pipe(rest);
        let name = channels.get(id).map(String::as_str).unwrap_or(fallback);
        return format!(r#"<span class="mention">#{}</span>"#, escape_html(name));
    }

    // Broadcast mentions
    if inner == "!here" {
        return r#"<span class="mention">@here</span>"#.to_owned();
    }
    if inner == "!channel" {
        return r#"<span class="mention">@channel</span>"#.to_owned();
    }
    if inner == "!everyone" {
        return r#"<span class="mention">@everyone</span>"#.to_owned();
    }

    // URL: <https://...|label> or <https://...>
    if inner.starts_with("http://") || inner.starts_with("https://") {
        let has_label = inner.contains('|');
        let (url, label) = split_pipe(inner);

        // Rewrite internal Slack thread links to archiver URLs
        let workspace = std::env::var("SLACK_WORKSPACE_URL").ok();
        if let Some((channel_id, ts)) = parse_slack_thread_url(url, workspace.as_deref()) {
            let display = if has_label {
                escape_html(label)
            } else {
                let ch_name = channels
                    .get(&channel_id)
                    .map(|n| format!("#{n}"))
                    .unwrap_or_else(|| channel_id.clone());
                format!("↗ thread in {}", escape_html(&ch_name))
            };
            return format!(
                r#"<a href="/record/thread?channel_id={}&ts={}" data-src="{}" target="_blank" rel="noopener">{}</a>"#,
                channel_id,
                ts,
                escape_html(url),
                display
            );
        }

        return format!(
            r#"<a href="{}" target="_blank" rel="noopener">{}</a>"#,
            escape_html(url),
            escape_html(label),
        );
    }

    // Unknown — escape and render as literal
    format!("&lt;{}&gt;", escape_html(inner))
}

/// If `tag` is an `<a href="...">` whose href contains `needle`, inject `class="url-highlight"`.
/// Otherwise returns the tag unchanged.
fn highlight_link_href(tag: &str, needle: &str) -> String {
    if needle.is_empty() {
        return tag.to_owned();
    }
    let tag_lower = tag.to_lowercase();
    if !tag_lower.starts_with("<a ") {
        return tag.to_owned();
    }
    let href_match = tag_lower.find("href=\"").and_then(|i| {
        let after = &tag_lower[i + 6..];
        after.find('"').map(|e| after[..e].contains(needle))
    });
    let src_match = tag_lower.find("data-src=\"").and_then(|i| {
        let after = &tag_lower[i + 10..];
        after.find('"').map(|e| after[..e].contains(needle))
    });
    if href_match.unwrap_or(false) || src_match.unwrap_or(false) {
        return tag.replacen("<a ", "<a class=\"url-highlight\" ", 1);
    }
    tag.to_owned()
}

/// Wrap occurrences of `search` in `<mark class="search-highlight">` inside pre-rendered HTML.
/// Only replaces in text nodes — never inside HTML tags — to avoid corrupting markup.
pub fn highlight_search(html: &str, search: &str) -> String {
    if search.is_empty() {
        return html.to_owned();
    }
    // Match the HTML-escaped form so we find what's actually in the text nodes
    let needle = escape_html(search).to_lowercase();
    let mut result = String::with_capacity(html.len() + 64);
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' {
            // Copy HTML tag verbatim — never replace inside tags.
            // Exception: <a> tags whose href contains the needle get a url-highlight class.
            let tag_start = i;
            i += 1;
            while i < len && bytes[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            let tag = &html[tag_start..i];
            result.push_str(&highlight_link_href(tag, &needle));
        } else {
            // Text node: collect until next '<' then replace matches
            let text_start = i;
            while i < len && bytes[i] != b'<' {
                i += 1;
            }
            let segment = &html[text_start..i];
            let seg_lower = segment.to_lowercase();
            let mut pos = 0;
            while pos < segment.len() {
                match seg_lower[pos..].find(&needle) {
                    None => {
                        result.push_str(&segment[pos..]);
                        break;
                    }
                    Some(rel) => {
                        let abs = pos + rel;
                        result.push_str(&segment[pos..abs]);
                        result.push_str("<mark class=\"search-highlight\">");
                        result.push_str(&segment[abs..abs + needle.len()]);
                        result.push_str("</mark>");
                        pos = abs + needle.len();
                    }
                }
            }
        }
    }
    result
}

/// Decode the three HTML entities that Slack pre-encodes in message text.
/// Must be applied before `escape_html` to avoid double-encoding.
fn decode_slack_entities(s: &str) -> String {
    // Order matters: decode &lt;/&gt; before &amp; so we don't turn
    // "&amp;lt;" into "<" instead of the correct "&lt;".
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Parse an internal Slack thread URL into `(channel_id, ts)` when it matches
/// `workspace`. Returns `None` for external or unrecognised URLs.
///
/// Slack archive URL formats:
///   root message : `https://{host}/archives/{channel_id}/p{ts_no_dot}`
///   thread reply : `https://{host}/archives/{channel_id}/p{ts}?thread_ts={root_ts}&cid=...`
pub(crate) fn parse_slack_thread_url(
    url: &str,
    workspace: Option<&str>,
) -> Option<(String, String)> {
    let host = workspace?
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');

    let rest = url.strip_prefix(&format!("https://{}/archives/", host))?;

    let slash = rest.find('/')?;
    let channel_id = rest[..slash].to_owned();
    let ts_part = &rest[slash + 1..];
    let ts_digits = ts_part.strip_prefix('p')?;

    let ts = if let Some(q) = ts_digits.find('?') {
        // Reply URL: thread_ts query param holds the root message ts (already dotted)
        let query = &ts_digits[q + 1..];
        query
            .split('&')
            .find_map(|p| p.strip_prefix("thread_ts="))
            .map(|v| v.to_owned())
            .unwrap_or_else(|| p_digits_to_ts(&ts_digits[..q]))
    } else {
        p_digits_to_ts(ts_digits)
    };

    Some((channel_id, ts))
}

/// Convert p-format timestamp digits (dot removed) back to Slack ts.
/// "1700000000123456" → "1700000000.123456"
fn p_digits_to_ts(digits: &str) -> String {
    if digits.len() > 10 {
        format!("{}.{}", &digits[..10], &digits[10..])
    } else {
        digits.to_owned()
    }
}

fn split_pipe(s: &str) -> (&str, &str) {
    if let Some(pos) = s.find('|') {
        (&s[..pos], &s[pos + 1..])
    } else {
        (s, s)
    }
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}
