use std::collections::HashMap;

use maud::{Markup, PreEscaped, html};

use crate::db::ThreadSummary;
use crate::render::text::{highlight_search, render_text_simple};

// Slack logo SVG (14×14)
const SLACK_ICON_SVG: &str = r#"<svg width="14" height="14" viewBox="0 0 122 122" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M25.6 76.8a12.8 12.8 0 1 1-12.8-12.8H25.6v12.8zm6.4 0a12.8 12.8 0 0 1 25.6 0v32a12.8 12.8 0 0 1-25.6 0v-32zM45.2 25.6a12.8 12.8 0 1 1 12.8-12.8V25.6H45.2zm0 6.4a12.8 12.8 0 0 1 0 25.6H13.2a12.8 12.8 0 0 1 0-25.6h32zM96.4 45.2a12.8 12.8 0 1 1 12.8 12.8H96.4V45.2zm-6.4 0a12.8 12.8 0 0 1-25.6 0v-32a12.8 12.8 0 0 1 25.6 0v32zM76.8 96.4a12.8 12.8 0 1 1-12.8 12.8V96.4h12.8zm0-6.4a12.8 12.8 0 0 1 0-25.6h32a12.8 12.8 0 0 1 0 25.6h-32z" fill="currentColor"/></svg>"#;

/// Format a Slack timestamp (Unix seconds as string) to "DD Mon YYYY".
pub fn format_ts_date(ts: &str) -> String {
    let secs: f64 = ts.parse().unwrap_or(0.0);
    let dt =
        chrono::DateTime::from_timestamp(secs as i64, 0).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    dt.format("%d %b %Y").to_string()
}

pub fn render_score_badge(score: i64) -> Markup {
    html! { div class="score-badge" { (score) } }
}

pub fn render_avatar(avatar_url: &str, name: &str, size: &str) -> Markup {
    let initial = name
        .chars()
        .next()
        .and_then(|c| c.to_uppercase().next())
        .unwrap_or('?');

    if avatar_url.is_empty() {
        let cls = if size == "large" {
            "msg-avatar-placeholder"
        } else {
            "avatar-placeholder"
        };
        html! { span class=(cls) { (initial) } }
    } else {
        let cls = if size == "large" {
            "msg-avatar"
        } else {
            "avatar"
        };
        html! { img class=(cls) src=(avatar_url) alt=(name) loading="lazy"; }
    }
}

pub fn render_thread_card(
    t: &ThreadSummary,
    search: &str,
    users: &HashMap<String, String>,
) -> Markup {
    let date = format_ts_date(&t.thread_ts);

    // With search: show the first line containing the match so the highlight is visible.
    // Without search: just use the first non-empty line.
    let first_line = if search.is_empty() {
        t.text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("(no text)")
    } else {
        let sl = search.to_lowercase();
        t.text
            .lines()
            .find(|l| l.to_lowercase().contains(&sl))
            .or_else(|| t.text.lines().find(|l| !l.trim().is_empty()))
            .unwrap_or("(no text)")
    };

    // Render preview then optionally highlight the search term
    let preview_html = {
        let rendered = render_text_simple(first_line, users).into_string();
        if search.is_empty() {
            rendered
        } else {
            highlight_search(&rendered, search)
        }
    };

    // Pass the search term to the thread fragment so it can highlight matches there too
    let hx_url = if search.is_empty() {
        format!(
            "/record/thread?channel_id={}&ts={}",
            t.channel_id, t.thread_ts
        )
    } else {
        format!(
            "/record/thread?channel_id={}&ts={}&search={}",
            t.channel_id,
            t.thread_ts,
            encode_query_param(search)
        )
    };

    html! {
        details class="thread-card"
                data-channel=(t.channel_id)
                data-ts=(t.thread_ts)
        {
            summary class="thread-header"
                    "hx-get"=(hx_url)
                    "hx-target"="next .thread-messages"
                    "hx-trigger"="click once"
                    "hx-swap"="innerHTML"
            {
                (render_score_badge(t.score))
                div class="thread-meta" {
                    div class="thread-top" {
                        (render_avatar(&t.avatar_url, &t.display_name, "small"))
                        span class="author" { (t.display_name) }
                        span class="channel" { "#" (t.channel_name) }
                        span class="date" { (date) }
                    }
                    div class="thread-preview" { (PreEscaped(preview_html)) }
                    div class="thread-stats" {
                        span { "💬 " (t.reply_count) " replies" }
                        span { "⚡ " (t.reaction_count) " reactions" }
                        span { "👥 " (t.participant_count) " people" }
                    }
                }
            }
            // Loading placeholder: hidden while <details> is closed,
            // visible immediately on expand, replaced by HTMX on load
            div class="thread-messages" {
                p class="loading" { "Loading thread…" }
            }
        }
    }
}

pub fn render_header(workspace_url: Option<&str>) -> Markup {
    html! {
        header {
            h1 { "Archivist" }
            span { "Top Threads" }
            div class="header-right" {
                @if let Some(url) = workspace_url {
                    @let full_url = if url.starts_with("http://") || url.starts_with("https://") {
                        url.to_owned()
                    } else {
                        format!("https://{url}")
                    };
                    a href=(full_url) target="_blank" rel="noopener" class="slack-link" {
                        (PreEscaped(SLACK_ICON_SVG))
                        "Open Slack"
                    }
                }
            }
        }
    }
}

pub fn render_filter_bar(
    threads: &[ThreadSummary],
    sort: &str,
    period: &str,
    user: &str,
    channel: &str,
) -> Markup {
    // Collect unique, sorted channels and users from the provided thread list
    let mut channels: Vec<&str> = {
        let mut seen = std::collections::BTreeSet::new();
        for t in threads {
            seen.insert(t.channel_name.as_str());
        }
        seen.into_iter().collect()
    };
    channels.sort_unstable_by_key(|s| s.to_ascii_lowercase());

    let mut users: Vec<&str> = {
        let mut seen = std::collections::BTreeSet::new();
        for t in threads {
            seen.insert(t.display_name.as_str());
        }
        seen.into_iter().collect()
    };
    users.sort_unstable_by_key(|s| s.to_ascii_lowercase());

    html! {
        form class="filter-bar"
             "hx-get"="/record/threads"
             "hx-target"="#threads"
             "hx-trigger"="change, input delay:300ms from:#search-input, keyup[key=='Enter' && target.value.trim()!==''] changed from:#search-input"
             "hx-indicator"="#page-loader"
             "hx-sync"="this:replace"
             onsubmit="return false"
        {
            // Row 1: search (grows) + sort (compact)
            div class="filter-row filter-row-top" {
                div class="filter-group filter-group-search" {
                    input type="search" id="search-input" name="search"
                          placeholder="Search messages…" class="filter-search"
                          autocomplete="off";
                }
                div class="filter-group" {
                    label "for"="sort-select" { "Sort" }
                    select id="sort-select" name="sort" {
                        option value="score"     selected[sort == "score"]     { "Score" }
                        option value="date"      selected[sort == "date"]      { "Date" }
                        option value="reactions" selected[sort == "reactions"] { "Reactions" }
                        option value="replies"   selected[sort == "replies"]   { "Replies" }
                    }
                }
            }
            // Row 2: period + channel + user (share space equally)
            div class="filter-row filter-row-bottom" {
                div class="filter-group" {
                    label "for"="period-select" { "Period" }
                    select id="period-select" name="period" {
                        option value="all" selected[period == "all"] { "All time" }
                        option value="30d" selected[period == "30d"] { "30 days" }
                        option value="7d"  selected[period == "7d"]  { "7 days" }
                    }
                }
                div class="filter-group" {
                    label "for"="channel-select" { "Channel" }
                    select id="channel-select" name="channel" {
                        option value="" selected[channel.is_empty()] { "All channels" }
                        @for ch in &channels {
                            option value=(ch) selected[*ch == channel] { "#" (ch) }
                        }
                    }
                }
                div class="filter-group" {
                    label "for"="user-select" { "User" }
                    select id="user-select" name="user" {
                        option value="" selected[user.is_empty()] { "All users" }
                        @for u in &users {
                            option value=(u) selected[*u == user] { (u) }
                        }
                    }
                }
            }
        }
    }
}

pub fn render_threads_content(
    threads: &[ThreadSummary],
    search: &str,
    users: &HashMap<String, String>,
) -> Markup {
    if threads.is_empty() {
        html! { p class="empty" { "No threads yet. Run a backfill to get started." } }
    } else {
        html! { @for t in threads { (render_thread_card(t, search, users)) } }
    }
}

fn encode_query_param(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(
                    char::from_digit((b >> 4) as u32, 16)
                        .unwrap()
                        .to_ascii_uppercase(),
                );
                out.push(
                    char::from_digit((b & 0xf) as u32, 16)
                        .unwrap()
                        .to_ascii_uppercase(),
                );
            }
        }
    }
    out
}
