use maud::{DOCTYPE, Markup, html};

use crate::db::ThreadSummary;
use crate::render::components::{render_filter_bar, render_header, render_threads_content};

pub fn render_page(threads: &[ThreadSummary], workspace_url: Option<&str>) -> Markup {
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
            }
            body {
                (render_header(workspace_url))
                (render_filter_bar(threads, "score", "all", "", ""))
                div id="threads" {
                    (render_threads_content(&threads[..threads.len().min(50)], ""))
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
