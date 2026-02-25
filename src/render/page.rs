use std::collections::HashMap;

use maud::{DOCTYPE, Markup, html};

use crate::db::{FileRow, PeriodRankedThread, ThreadMessage, ThreadSummary, ThreadWithWeeklyScore};
use crate::render::components::{
    PositionChange, render_filter_bar, render_header, render_header_with_subtitle,
    render_thread_card_with_meta, render_threads_content,
};
use crate::render::thread::render_thread_fragment;

pub fn render_thread_page(
    messages: &[ThreadMessage],
    files_by_ts: &HashMap<String, Vec<FileRow>>,
    users: &HashMap<String, String>,
    channels: &HashMap<String, String>,
    search: &str,
    workspace_url: Option<&str>,
) -> Markup {
    let fragment = render_thread_fragment(messages, files_by_ts, users, channels, search);
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Archivist — Thread" }
                link rel="icon" type="image/svg+xml" href="/favicon.svg";
                link rel="stylesheet" href="/record.css";
                script src="https://unpkg.com/htmx.org@2.0.4" defer {}
                script src="/modal.js" defer {}
                script src="/tabs-loader.js" defer {}
            }
            body {
                (render_header(workspace_url))
                div id="page-loader" aria-hidden="true" {}
                div id="threads" {
                    div class="thread-messages" style="border:none" {
                        (fragment)
                    }
                }
                div id="file-modal" class="file-modal-overlay"
                    style="display:none" role="dialog" aria-modal="true"
                {
                    button class="modal-nav modal-prev" id="modal-prev" title="Previous (←)" { "←" }
                    div class="file-modal-box" {
                        div class="file-modal-header" {
                            span class="file-modal-name" id="modal-filename" {}
                            div class="file-modal-actions" {
                                span class="file-modal-counter" id="modal-counter" {}
                                a id="modal-download" class="file-modal-download"
                                  target="_blank" rel="noopener" { "↓ Download" }
                                button class="file-modal-close" id="modal-close"
                                       title="Close (Esc)" { "✕" }
                            }
                        }
                        div class="file-modal-body" id="modal-body" {}
                    }
                    button class="modal-nav modal-next" id="modal-next" title="Next (→)" { "→" }
                }
            }
        }
    }
}

pub fn render_page(
    threads: &[ThreadSummary],
    workspace_url: Option<&str>,
    users: &HashMap<String, String>,
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Archivist — Top Threads" }
                link rel="icon" type="image/svg+xml" href="/favicon.svg";
                link rel="stylesheet" href="/record.css";
                script src="https://unpkg.com/htmx.org@2.0.4" defer {}
                script src="/modal.js" defer {}
                script src="/filter.js" defer {}
                script src="/tabs-loader.js" defer {}
            }
            body {
                (render_header(workspace_url))
                (render_filter_bar(threads, "score", "all", "", ""))
                div id="threads" {
                    (render_threads_content(threads, "", users))
                }
                // Page-level loading overlay — shown via hx-indicator="#page-loader"
                div id="page-loader" aria-hidden="true" {}
                // File preview modal (controlled by modal.js)
                div id="file-modal" class="file-modal-overlay"
                    style="display:none" role="dialog" aria-modal="true"
                {
                    button class="modal-nav modal-prev" id="modal-prev"
                           title="Previous (←)" { "←" }
                    div class="file-modal-box" {
                        div class="file-modal-header" {
                            span class="file-modal-name" id="modal-filename" {}
                            div class="file-modal-actions" {
                                span class="file-modal-counter" id="modal-counter" {}
                                a id="modal-download" class="file-modal-download"
                                  target="_blank" rel="noopener" { "↓ Download" }
                                button class="file-modal-close" id="modal-close"
                                       title="Close (Esc)" { "✕" }
                            }
                        }
                        div class="file-modal-body" id="modal-body" {}
                    }
                    button class="modal-nav modal-next" id="modal-next"
                           title="Next (→)" { "→" }
                }
            }
        }
    }
}

fn change_from_rank(rank: i64, prev_rank: Option<i64>) -> Option<PositionChange> {
    match prev_rank {
        None => Some(PositionChange::New),
        Some(prev) if prev > rank => Some(PositionChange::Up(prev - rank)),
        Some(prev) if prev < rank => Some(PositionChange::Down(rank - prev)),
        _ => None,
    }
}

pub fn render_weekly_page(
    tab: &str,
    top_threads: &[ThreadWithWeeklyScore],
    ranked_threads: &[PeriodRankedThread],
    workspace_url: Option<&str>,
    users: &HashMap<String, String>,
) -> Markup {
    let filter_threads: Vec<ThreadSummary> = if tab == "top" {
        top_threads.iter().map(|row| row.thread.clone()).collect()
    } else {
        ranked_threads
            .iter()
            .map(|row| row.thread.clone())
            .collect()
    };

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Archivist — Weekly Rankings" }
                link rel="icon" type="image/svg+xml" href="/favicon.svg";
                link rel="stylesheet" href="/record.css";
                script src="https://unpkg.com/htmx.org@2.0.4" defer {}
                script src="/modal.js" defer {}
                script src="/filter.js" defer {}
                script src="/tabs-loader.js" defer {}
            }
            body {
                (render_header_with_subtitle(workspace_url, tab))
                (render_filter_bar(&filter_threads, "score", "all", "", ""))
                div id="threads" {
                    @if tab == "top" {
                        @if top_threads.is_empty() {
                            p class="empty" { "No ranked threads yet. Run a backfill to get started." }
                        } @else {
                            @for row in top_threads {
                                (render_thread_card_with_meta(
                                    &row.thread,
                                    "",
                                    users,
                                    None,
                                    Some(row.score_week),
                                    None,
                                ))
                            }
                        }
                    } @else if ranked_threads.is_empty() {
                        p class="empty" { "No ranking data for this period yet." }
                    } @else {
                        @for row in ranked_threads {
                            (render_thread_card_with_meta(
                                &row.thread,
                                "",
                                users,
                                Some(row.rank_score),
                                None,
                                change_from_rank(row.rank, row.prev_rank),
                            ))
                        }
                    }
                }
                div id="page-loader" aria-hidden="true" {}
                div id="file-modal" class="file-modal-overlay"
                    style="display:none" role="dialog" aria-modal="true"
                {
                    button class="modal-nav modal-prev" id="modal-prev"
                           title="Previous (←)" { "←" }
                    div class="file-modal-box" {
                        div class="file-modal-header" {
                            span class="file-modal-name" id="modal-filename" {}
                            div class="file-modal-actions" {
                                span class="file-modal-counter" id="modal-counter" {}
                                a id="modal-download" class="file-modal-download"
                                  target="_blank" rel="noopener" { "↓ Download" }
                                button class="file-modal-close" id="modal-close"
                                       title="Close (Esc)" { "✕" }
                            }
                        }
                        div class="file-modal-body" id="modal-body" {}
                    }
                    button class="modal-nav modal-next" id="modal-next"
                           title="Next (→)" { "→" }
                }
            }
        }
    }
}
