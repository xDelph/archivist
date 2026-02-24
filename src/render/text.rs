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

/// Convenience: render with empty user/channel maps (for thread card previews).
pub fn render_text_simple(text: &str) -> Markup {
    let empty = HashMap::new();
    PreEscaped(demojify_str(&render_slack_text_inner(text, &empty, &empty)))
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
            // Accumulate plain text until '<' or '\n', then HTML-escape it
            let start = i;
            while i < len && bytes[i] != b'<' && bytes[i] != b'\n' {
                i += 1;
            }
            out.push_str(&escape_html(&text[start..i]));
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
        // If no display name in text (fallback == id) and not in map, show @… not the raw ID
        let name = users
            .get(id)
            .map(String::as_str)
            .unwrap_or(if fallback != id { fallback } else { "…" });
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
        let (url, label) = split_pipe(inner);
        return format!(
            r#"<a href="{}" target="_blank" rel="noopener">{}</a>"#,
            escape_html(url),
            escape_html(label),
        );
    }

    // Unknown — escape and render as literal
    format!("&lt;{}&gt;", escape_html(inner))
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
            // Copy HTML tag verbatim — never replace inside tags
            let tag_start = i;
            i += 1;
            while i < len && bytes[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            result.push_str(&html[tag_start..i]);
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
