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

#[derive(Clone)]
pub enum PositionChange {
    Up(i64),
    Down(i64),
    New,
}

fn render_position_badge(change: &PositionChange) -> Markup {
    match change {
        PositionChange::Up(delta) => html! { span class="rank-badge rank-up" { "↑" (delta) } },
        PositionChange::Down(delta) => {
            html! { span class="rank-badge rank-down" { "↓" (delta) } }
        }
        PositionChange::New => html! { span class="rank-badge rank-new" { "NEW" } },
    }
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
    render_thread_card_with_meta(t, search, users, None, None, None)
}

pub fn render_thread_card_with_meta(
    t: &ThreadSummary,
    search: &str,
    users: &HashMap<String, String>,
    score_override: Option<i64>,
    score_week_badge: Option<i64>,
    position_change: Option<PositionChange>,
) -> Markup {
    let date = format_ts_date(&t.thread_ts);
    let displayed_score = score_override.unwrap_or(t.score);

    // Base preview (always first non-empty line, no highlight) — stored in data-preview
    // so client-side JS can restore it before re-applying a new search highlight.
    let base_line = t
        .text
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("(no text)");
    let preview_base = render_text_simple(base_line, users).into_string();

    // Server-side render: when search is provided (e.g. via /record/threads), prefer
    // the line containing the match and apply a highlight.
    let preview_html = if search.is_empty() {
        preview_base.clone()
    } else {
        let sl = search.to_lowercase();
        let best = t
            .text
            .lines()
            .find(|l| l.to_lowercase().contains(&sl))
            .unwrap_or(base_line);
        highlight_search(&render_text_simple(best, users).into_string(), search)
    };

    // search is appended at request time via hx-include, not baked into the URL.
    let hx_url = format!(
        "/record/thread?channel_id={}&ts={}",
        t.channel_id, t.thread_ts
    );

    html! {
        details class="thread-card"
                data-channel=(t.channel_id)
                data-channel-name=(t.channel_name)
                data-ts=(t.thread_ts)
                data-user=(t.display_name)
                data-score=(displayed_score)
                data-replies=(t.reply_count)
                data-reactions=(t.reaction_count)
                data-text=(t.text.to_lowercase())
                data-preview=(preview_base)
        {
            summary class="thread-header"
                    "hx-get"=(hx_url)
                    "hx-target"="next .thread-messages"
                    "hx-trigger"="click once"
                    "hx-swap"="innerHTML"
                    "hx-include"="#search-input"
            {
                div class="thread-rank-col" {
                    (render_score_badge(displayed_score))
                    @if let Some(change) = &position_change {
                        (render_position_badge(change))
                    }
                }
                div class="thread-meta" {
                    div class="thread-top" {
                        div class="thread-top-left" {
                            (render_avatar(&t.avatar_url, &t.display_name, "small"))
                            span class="author" { (t.display_name) }
                            span class="channel" { "#" (t.channel_name) }
                        }
                        div class="thread-top-right" {
                            span class="thread-top-stats" {
                                span class="thread-top-stat" {
                                    span class="thread-top-stat-icon" { "💬" }
                                    span class="thread-top-stat-num" { (t.reply_count) }
                                }
                                span class="thread-top-stat" {
                                    span class="thread-top-stat-icon" { "⚡" }
                                    span class="thread-top-stat-num" { (t.reaction_count) }
                                }
                                span class="thread-top-stat" {
                                    span class="thread-top-stat-icon" { "👥" }
                                    span class="thread-top-stat-num" { (t.participant_count) }
                                }
                            }
                            span class="date" { (date) }
                            @if let Some(score_week) = score_week_badge.filter(|v| *v > 0) {
                                span class="rank-badge rank-week" { "W " (score_week) }
                            }
                        }
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
    render_header_with_subtitle(workspace_url, "top")
}

fn tab_class(active: bool) -> &'static str {
    if active {
        "header-tab active"
    } else {
        "header-tab"
    }
}

pub fn render_header_with_subtitle(workspace_url: Option<&str>, active_tab: &str) -> Markup {
    html! {
        header {
            h1 { "Archivist" }
            nav class="header-tabs" {
                a href="/record" class=(tab_class(active_tab == "top")) { "Top threads" }
                a href="/record/weekly?tab=week" class=(tab_class(active_tab == "week")) { "This week" }
                a href="/record/weekly?tab=month" class=(tab_class(active_tab == "month")) { "This month" }
            }
            div class="header-right" {
                div class="header-preferences" {
                    div class="header-segment" role="group" aria-label="Card density" {
                        button type="button" class="header-toggle" data-density-choice="compact" { "Compact" }
                        button type="button" class="header-toggle" data-density-choice="normal" { "Normal" }
                    }
                    div class="header-segment" role="group" aria-label="Color theme" {
                        button type="button" class="header-toggle" data-theme-choice="light" { "Light" }
                        button type="button" class="header-toggle" data-theme-choice="dark" { "Dark" }
                    }
                }
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
        form class="filter-bar" id="filter-form" onsubmit="return false" {
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
