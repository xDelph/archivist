use std::collections::HashMap;

use maud::{Markup, PreEscaped, html};

use crate::db::{FileRow, ThreadMessage};
use crate::render::components::render_avatar;
use crate::render::text::{demojify, highlight_search, render_slack_text};

// ── Public API ────────────────────────────────────────────────────────────────

/// Render the HTMX fragment returned by `GET /record/thread`.
pub fn render_thread_fragment(
    messages: &[ThreadMessage],
    files_by_ts: &HashMap<String, Vec<FileRow>>,
    users: &HashMap<String, String>,
    channels: &HashMap<String, String>,
    search: &str,
) -> Markup {
    if messages.is_empty() {
        return html! { p class="empty" { "No messages." } };
    }

    let count = messages.len();
    html! {
        @for msg in messages {
            @let rendered = demojify(&render_slack_text(&msg.text, users, channels).into_string());
            @let text_html = if search.is_empty() { rendered.clone() } else { highlight_search(&rendered, search) };
            div class="message" {
                (render_avatar(&msg.avatar_url, &msg.display_name, "large"))
                div class="msg-body" {
                    div class="msg-top" {
                        span class="msg-author" { (&msg.display_name) }
                        span class="msg-time" { (format_ts_time(&msg.ts)) }
                    }
                    div class="msg-text" { (PreEscaped(text_html)) }
                    @if let Some(files) = files_by_ts.get(&msg.ts) {
                        @for f in files {
                            (render_file(f))
                        }
                    }
                    (render_reactions(&msg.reactions))
                }
            }
        }
        p style="padding-top:8px;color:var(--muted);font-size:0.78rem" {
            (count) " message" @if count != 1 { "s" }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn format_ts_time(ts: &str) -> String {
    let secs: f64 = ts.parse().unwrap_or(0.0);
    let dt =
        chrono::DateTime::from_timestamp(secs as i64, 0).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    dt.format("%H:%M").to_string()
}

fn render_file(f: &FileRow) -> Markup {
    let url = &f.storage_url;
    let name = &f.name;
    let mime = &f.mimetype;

    if mime.starts_with("image/") {
        html! {
            div class="msg-file" {
                img src=(url) alt=(name) loading="lazy" class="msg-file-img"
                    data-file-url=(url) data-file-name=(name) data-file-mime=(mime)
                    title="Click to enlarge";
            }
        }
    } else if mime.starts_with("video/") || mime == "application/pdf" || mime.starts_with("text/") {
        let icon = if mime.starts_with("video/") {
            "▶"
        } else {
            "📄"
        };
        html! {
            div class="msg-file" {
                span class="msg-file-preview"
                     data-file-url=(url) data-file-name=(name) data-file-mime=(mime)
                {
                    (icon) " " (name) " " span class="mime-badge" { (mime) }
                }
            }
        }
    } else {
        html! {
            div class="msg-file" {
                a href=(url) target="_blank" rel="noopener" class="msg-file-link" {
                    "📎 " (name)
                }
            }
        }
    }
}

fn render_reactions(reactions: &serde_json::Value) -> Markup {
    let arr = match reactions.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return html! {},
    };

    html! {
        div class="msg-reactions" {
            @for rx in arr {
                @let name = rx.get("name").and_then(|v| v.as_str()).unwrap_or("");
                @let count = rx.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
                @let emoji_display = emojis::get_by_shortcode(name)
                    .map(|e| e.as_str())
                    .unwrap_or(name);
                span class="reaction" { (emoji_display) " " (count) }
            }
        }
    }
}
