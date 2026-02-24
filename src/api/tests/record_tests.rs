use std::collections::HashMap;

use bytes::Bytes;
use chrono::Utc;

use crate::api::record::process;
use crate::db::{InMemoryRepository, ThreadSummary};
use crate::render::components::{render_filter_bar, render_thread_card, render_threads_content};
use crate::render::page::render_page;
use crate::render::text::highlight_search;
use crate::render::text::{demojify, render_slack_text, render_text_simple};

fn empty_users() -> HashMap<String, String> {
    HashMap::new()
}

fn make_get(path: &str) -> http::Request<Bytes> {
    http::Request::builder()
        .method("GET")
        .uri(path)
        .body(Bytes::new())
        .unwrap()
}

fn make_thread(score: i64, channel_name: &str) -> ThreadSummary {
    ThreadSummary {
        channel_id: "C123".to_owned(),
        channel_name: channel_name.to_owned(),
        thread_ts: "1700000000.000000".to_owned(),
        text: "Hello :thumbsup:".to_owned(),
        created_at: Utc::now(),
        display_name: "Alice".to_owned(),
        avatar_url: String::new(),
        reaction_count: 0,
        reply_count: 0,
        participant_count: 1,
        score,
    }
}

// ── Text rendering — pure functions ───────────────────────────────────────────

#[test]
fn test_demojify_replaces_shortcode() {
    assert!(demojify(":thumbsup:").contains("👍"));
}

#[test]
fn test_demojify_no_match_is_unchanged() {
    let input = ":notarealemoji:";
    assert_eq!(demojify(input), input);
}

#[test]
fn test_render_slack_text_user_mention() {
    let users = HashMap::from([("U123".to_owned(), "Alice".to_owned())]);
    let html = render_slack_text("<@U123>", &users, &HashMap::new()).into_string();
    assert!(html.contains("@Alice"));
    assert!(html.contains("mention"));
}

#[test]
fn test_render_slack_text_channel_mention() {
    let channels = HashMap::from([("C456".to_owned(), "general".to_owned())]);
    let html = render_slack_text("<#C456|general>", &HashMap::new(), &channels).into_string();
    assert!(html.contains("#general"));
    assert!(html.contains("mention"));
}

#[test]
fn test_render_slack_text_escapes_plain() {
    let html = render_slack_text("<script>", &HashMap::new(), &HashMap::new()).into_string();
    assert!(!html.contains("<script>"));
}

#[test]
fn test_render_slack_text_broadcast_here() {
    let html = render_slack_text("<!here>", &HashMap::new(), &HashMap::new()).into_string();
    assert!(html.contains("@here"));
    assert!(html.contains("mention"));
}

#[test]
fn test_render_slack_text_url_with_label() {
    let html = render_slack_text(
        "<https://example.com|Example>",
        &HashMap::new(),
        &HashMap::new(),
    )
    .into_string();
    assert!(html.contains("href=\"https://example.com\""));
    assert!(html.contains("Example"));
}

#[test]
fn test_render_text_simple_decodes_slack_entities() {
    let users = HashMap::new();
    // Slack sends "-&gt;" for "->"; should render as "->" not "-&gt;"
    let html = render_text_simple("a -&gt; b", &users).into_string();
    assert!(
        html.contains("a -&gt; b"),
        "expected HTML-encoded arrow, got: {html}"
    );
    assert!(!html.contains("&amp;gt;"), "must not double-encode: {html}");
}

#[test]
fn test_render_text_simple_user_mention_resolved() {
    let users = HashMap::from([("U123".to_owned(), "Alice".to_owned())]);
    let html = render_text_simple("<@U123>", &users).into_string();
    assert!(html.contains("@Alice"), "expected resolved name: {html}");
}

#[test]
fn test_render_text_simple_user_mention_fallback_to_id() {
    // No users map, no pipe fallback: should show the raw ID, not @…
    let html = render_text_simple("<@U999XYZ>", &HashMap::new()).into_string();
    assert!(html.contains("@U999XYZ"), "expected ID fallback: {html}");
    assert!(!html.contains("@…"), "should not show ellipsis: {html}");
}

// ── Component rendering — pure functions ──────────────────────────────────────

#[test]
fn test_thread_card_has_htmx_attrs() {
    let t = make_thread(42, "eng");
    let html = render_thread_card(&t, "", &empty_users()).into_string();
    assert!(html.contains("hx-get"));
    assert!(html.contains("/record/thread"));
}

#[test]
fn test_thread_card_shows_score() {
    let t = make_thread(99, "general");
    let html = render_thread_card(&t, "", &empty_users()).into_string();
    assert!(html.contains("99"));
}

#[test]
fn test_page_empty_state() {
    assert!(
        render_page(&[], None, &empty_users())
            .into_string()
            .contains("No threads yet")
    );
}

#[test]
fn test_page_has_htmx_script() {
    let html = render_page(&[], None, &empty_users()).into_string();
    assert!(html.contains("htmx.org"));
}

#[test]
fn test_page_renders_thread_cards() {
    let threads = vec![make_thread(10, "eng"), make_thread(5, "general")];
    let html = render_page(&threads, None, &empty_users()).into_string();
    assert!(html.contains("hx-get"));
    assert!(html.contains("#eng"));
    assert!(html.contains("#general"));
}

// ── Filter bar ────────────────────────────────────────────────────────────────

#[test]
fn test_filter_bar_marks_active_sort() {
    let threads = vec![make_thread(10, "eng")];
    let html = render_filter_bar(&threads, "reactions", "all", "", "").into_string();
    assert!(html.contains(r#"value="reactions" selected"#));
    assert!(!html.contains(r#"value="score" selected"#));
}

#[test]
fn test_filter_bar_marks_active_period() {
    let threads = vec![make_thread(10, "eng")];
    let html = render_filter_bar(&threads, "score", "30d", "", "").into_string();
    assert!(html.contains(r#"value="30d" selected"#));
}

#[test]
fn test_filter_bar_has_htmx_attrs() {
    let html = render_filter_bar(&[], "score", "all", "", "").into_string();
    assert!(html.contains("hx-get=\"/record/threads\""));
    assert!(html.contains("hx-target=\"#threads\""));
}

#[test]
fn test_filter_bar_populates_channels() {
    let threads = vec![make_thread(10, "eng"), make_thread(5, "general")];
    let html = render_filter_bar(&threads, "score", "all", "", "").into_string();
    assert!(html.contains("#eng"));
    assert!(html.contains("#general"));
}

#[test]
fn test_filter_bar_marks_active_channel() {
    let threads = vec![make_thread(10, "eng")];
    let html = render_filter_bar(&threads, "score", "all", "", "eng").into_string();
    assert!(html.contains(r#"value="eng" selected"#));
}

#[test]
fn test_threads_content_empty() {
    let html = render_threads_content(&[], "", &empty_users()).into_string();
    assert!(html.contains("No threads yet"));
}

// ── Search ────────────────────────────────────────────────────────────────────

#[test]
fn test_highlight_search_wraps_match() {
    let html = highlight_search("Hello world", "world");
    assert!(html.contains("<mark class=\"search-highlight\">world</mark>"));
}

#[test]
fn test_highlight_search_case_insensitive() {
    let html = highlight_search("Hello World", "world");
    assert!(html.contains("<mark class=\"search-highlight\">World</mark>"));
}

#[test]
fn test_highlight_search_skips_tags() {
    let html = highlight_search(r#"<span class="mention">@Alice</span>"#, "span");
    // "span" inside the tag attribute should NOT be wrapped
    assert!(!html.contains("<mark"));
}

#[test]
fn test_highlight_search_empty_is_noop() {
    let input = "Hello world";
    assert_eq!(highlight_search(input, ""), input);
}

#[test]
fn test_thread_card_search_includes_search_in_url() {
    let t = make_thread(10, "eng");
    let html = render_thread_card(&t, "hello", &empty_users()).into_string();
    assert!(html.contains("search=hello"));
}

#[test]
fn test_thread_card_no_search_no_search_param() {
    let t = make_thread(10, "eng");
    let html = render_thread_card(&t, "", &empty_users()).into_string();
    assert!(!html.contains("search="));
}

// ── Handler tests — async, InMemoryRepository ─────────────────────────────────

#[tokio::test]
async fn test_record_page_returns_html() {
    let repo = InMemoryRepository::default();
    let resp = process(&repo, make_get("/record")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .contains("text/html")
    );
}

#[tokio::test]
async fn test_thread_fragment_missing_params_returns_400() {
    let repo = InMemoryRepository::default();
    let resp = process(&repo, make_get("/record/thread")).await.unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn test_threads_fragment_returns_html() {
    let repo = InMemoryRepository::default();
    *repo.threads.lock().unwrap() = vec![make_thread(10, "eng"), make_thread(5, "general")];
    let resp = process(&repo, make_get("/record/threads?sort=score&period=all"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    assert!(body.contains("thread-card"));
}

#[tokio::test]
async fn test_threads_fragment_sort_replies() {
    let mut t1 = make_thread(100, "eng");
    t1.reply_count = 2;
    let mut t2 = make_thread(50, "general");
    t2.reply_count = 10;
    let repo = InMemoryRepository::default();
    *repo.threads.lock().unwrap() = vec![t1, t2];
    let resp = process(&repo, make_get("/record/threads?sort=replies&period=all"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    // #general (10 replies) should appear before #eng (2 replies)
    let pos_general = body.find("#general").unwrap();
    let pos_eng = body.find("#eng").unwrap();
    assert!(pos_general < pos_eng);
}

#[tokio::test]
async fn test_thread_fragment_returns_html() {
    let repo = InMemoryRepository::default();
    let resp = process(
        &repo,
        make_get("/record/thread?channel_id=C123&ts=1700000000.000000"),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .contains("text/html")
    );
}
