use std::collections::HashMap;

use bytes::Bytes;
use chrono::Utc;
use serde_json::Value;

use crate::api::record::process;
use crate::db::{InMemoryRepository, PeriodRankedThread, ThreadSummary, ThreadWithWeeklyScore};
use crate::render::components::{
    PositionChange, render_filter_bar, render_thread_card, render_thread_card_with_meta,
    render_threads_content,
};
use crate::render::page::render_page;
use crate::render::text::{
    demojify, highlight_search, parse_slack_thread_url, render_slack_text, render_text_simple,
};

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
        file_count: 0,
        score,
    }
}

fn make_thread_days_ago(
    score: i64,
    channel_name: &str,
    display_name: &str,
    days_ago: i64,
    reply_count: i64,
) -> ThreadSummary {
    let ts = (Utc::now() - chrono::Duration::days(days_ago)).timestamp() as f64;
    ThreadSummary {
        channel_id: "C123".to_owned(),
        channel_name: channel_name.to_owned(),
        thread_ts: format!("{ts:.6}"),
        text: "Hello :thumbsup:".to_owned(),
        created_at: Utc::now(),
        display_name: display_name.to_owned(),
        avatar_url: String::new(),
        reaction_count: 0,
        reply_count,
        participant_count: 1,
        file_count: 0,
        score,
    }
}

fn make_top_thread_with_weekly(
    score: i64,
    score_week: i64,
    channel_name: &str,
) -> ThreadWithWeeklyScore {
    ThreadWithWeeklyScore {
        thread: make_thread(score, channel_name),
        score_week,
    }
}

fn make_ranked_thread(rank_score: i64, channel_name: &str) -> PeriodRankedThread {
    let mut thread = make_thread(rank_score, channel_name);
    thread.score = rank_score;
    PeriodRankedThread {
        thread,
        rank_score,
        rank: 1,
        prev_rank: None,
    }
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
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
fn test_render_slack_text_plain_url_is_linkified() {
    let html = render_slack_text(
        "See https://example.com/docs.",
        &HashMap::new(),
        &HashMap::new(),
    )
    .into_string();
    assert!(html.contains(r#"href="https://example.com/docs""#));
    assert!(html.contains("</a>."));
}

#[test]
fn test_render_slack_text_preserves_inline_code() {
    let html =
        render_slack_text("Run `cargo test` now", &HashMap::new(), &HashMap::new()).into_string();
    assert!(html.contains(r#"<code class="slack-inline-code">cargo test</code>"#));
}

#[test]
fn test_render_slack_text_preserves_code_block() {
    let html = render_slack_text("```rust\nlet x = 1;\n```", &HashMap::new(), &HashMap::new())
        .into_string();
    assert!(html.contains(r#"<pre class="slack-code"><code data-lang="rust">"#));
    assert!(html.contains("let x = 1;"));
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
fn test_page_has_ranking_tabs() {
    let html = render_page(&[], None, &empty_users()).into_string();
    assert!(html.contains("/record/weekly?tab=week"));
    assert!(html.contains("/record/weekly?tab=month"));
}

#[test]
fn test_page_header_has_no_subtitle_text_span() {
    let html = render_page(&[], None, &empty_users()).into_string();
    assert!(!html.contains("header-subtitle"));
}

#[test]
fn test_page_has_tabs_loader_script() {
    let html = render_page(&[], None, &empty_users()).into_string();
    assert!(html.contains("/tabs-loader.js"));
}

#[test]
fn test_page_has_view_preferences_script_and_controls() {
    let html = render_page(&[], None, &empty_users()).into_string();
    assert!(html.contains("/view-preferences.js"));
    assert!(html.contains("data-theme-toggle"));
    assert!(html.contains("data-density-toggle"));
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
fn test_filter_bar_has_form_id() {
    let html = render_filter_bar(&[], "score", "all", "", "").into_string();
    assert!(html.contains("id=\"filter-form\""));
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

// ── Internal Slack link rewriting ─────────────────────────────────────────────

#[test]
fn test_parse_slack_thread_url_root_message() {
    let r = parse_slack_thread_url(
        "https://devwithai.slack.com/archives/C123/p1700000000123456",
        Some("devwithai.slack.com"),
    );
    assert_eq!(r, Some(("C123".to_owned(), "1700000000.123456".to_owned())));
}

#[test]
fn test_parse_slack_thread_url_reply_uses_thread_ts() {
    let r = parse_slack_thread_url(
        "https://devwithai.slack.com/archives/C123/p1700000000000000?thread_ts=1700000001.000000&cid=C123",
        Some("devwithai.slack.com"),
    );
    assert_eq!(r, Some(("C123".to_owned(), "1700000001.000000".to_owned())));
}

#[test]
fn test_parse_slack_thread_url_external_no_match() {
    let r = parse_slack_thread_url("https://example.com/foo", Some("devwithai.slack.com"));
    assert!(r.is_none());
}

#[test]
fn test_parse_slack_thread_url_no_workspace() {
    let r = parse_slack_thread_url(
        "https://devwithai.slack.com/archives/C123/p1700000000123456",
        None,
    );
    assert!(r.is_none());
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
fn test_highlight_search_url_match_adds_class() {
    // Search term is in the href but not in the visible label
    let html = r#"<a href="https://example.com/foo" target="_blank">Click here</a>"#;
    let result = highlight_search(html, "foo");
    assert!(
        result.contains("url-highlight"),
        "expected url-highlight class: {result}"
    );
    assert!(
        !result.contains("<mark"),
        "label should not be wrapped in mark: {result}"
    );
}

#[test]
fn test_highlight_search_empty_is_noop() {
    let input = "Hello world";
    assert_eq!(highlight_search(input, ""), input);
}

#[test]
fn test_thread_card_has_hx_include() {
    // Search is appended dynamically via hx-include, not baked into the URL.
    let t = make_thread(10, "eng");
    let html = render_thread_card(&t, "hello", &empty_users()).into_string();
    assert!(html.contains("hx-include=\"#search-input\""));
    assert!(!html.contains("search=hello"));
}

#[test]
fn test_thread_card_no_search_no_search_param() {
    let t = make_thread(10, "eng");
    let html = render_thread_card(&t, "", &empty_users()).into_string();
    assert!(!html.contains("search="));
}

#[test]
fn test_thread_card_has_filter_data_attrs() {
    let t = make_thread(42, "eng");
    let html = render_thread_card(&t, "", &empty_users()).into_string();
    assert!(html.contains("data-user=\"Alice\""));
    assert!(html.contains("data-score=\"42\""));
    assert!(html.contains("data-channel-name=\"eng\""));
    assert!(html.contains("data-text="));
    assert!(html.contains("data-preview="));
}

#[test]
fn test_thread_card_has_compact_top_stats_markup() {
    let mut t = make_thread(42, "eng");
    t.reply_count = 12;
    t.reaction_count = 34;
    t.participant_count = 5;
    let html = render_thread_card(&t, "", &empty_users()).into_string();
    assert!(html.contains("thread-top-stats"));
    assert!(html.contains("thread-top-stat-num\">12<"));
    assert!(html.contains("thread-top-stat-num\">34<"));
    assert!(html.contains("thread-top-stat-num\">5<"));
}

#[test]
fn test_thread_card_with_weekly_score_badge() {
    let t = make_thread(42, "eng");
    let html =
        render_thread_card_with_meta(&t, "", &empty_users(), None, Some(9), None).into_string();
    assert!(html.contains("W 9"));
}

#[test]
fn test_thread_card_with_position_change_badge() {
    let t = make_thread(42, "eng");
    let html = render_thread_card_with_meta(
        &t,
        "",
        &empty_users(),
        Some(12),
        None,
        Some(PositionChange::Up(3)),
    )
    .into_string();
    assert!(html.contains("↑3"));
    assert!(html.contains("data-score=\"12\""));
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
async fn test_api_record_threads_returns_json() {
    let repo = InMemoryRepository::default();
    *repo.threads.lock().unwrap() = vec![make_thread(10, "eng"), make_thread(5, "general")];

    let resp = process(&repo, make_get("/api/record/threads?tab=top"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .contains("application/json")
    );

    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    let payload: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(payload.get("tab").and_then(Value::as_str), Some("top"));
    assert_eq!(
        payload
            .get("threads")
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        Some(2)
    );
    assert_eq!(
        payload
            .get("users")
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        Some(1)
    );
}

#[tokio::test]
async fn test_api_record_threads_accepts_hashed_channel_filter() {
    let repo = InMemoryRepository::default();
    *repo.threads.lock().unwrap() = vec![make_thread(10, "eng"), make_thread(5, "general")];

    let resp = process(
        &repo,
        make_get("/api/record/threads?tab=top&channel=%23eng"),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    let payload: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        payload
            .get("threads")
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        Some(1)
    );
}

#[tokio::test]
async fn test_api_record_threads_overview_changes_are_computed() {
    let repo = InMemoryRepository::default();
    *repo.threads.lock().unwrap() = vec![
        make_thread_days_ago(10, "eng", "Alice", 2, 1),
        make_thread_days_ago(9, "eng", "Bob", 5, 0),
        make_thread_days_ago(8, "eng", "Alice", 35, 0),
    ];

    let resp = process(&repo, make_get("/api/record/threads?tab=top&period=all"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    let payload: Value = serde_json::from_str(&body).unwrap();
    let overview = payload.get("overviewStats").unwrap();

    assert_eq!(
        overview.get("messagesChange").and_then(Value::as_f64),
        Some(200.0)
    );
    assert_eq!(
        overview.get("threadsChange").and_then(Value::as_f64),
        Some(100.0)
    );
    assert_eq!(
        overview.get("usersChange").and_then(Value::as_f64),
        Some(100.0)
    );
}

#[tokio::test]
async fn test_api_record_thread_returns_json() {
    let repo = InMemoryRepository::default();
    let resp = process(
        &repo,
        make_get("/api/record/thread?channel_id=C123&ts=1700000000.000000"),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .contains("application/json")
    );

    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    let payload: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        payload.get("channelId").and_then(Value::as_str),
        Some("C123")
    );
    assert!(
        payload
            .get("messages")
            .and_then(Value::as_array)
            .is_some_and(std::vec::Vec::is_empty)
    );
}

#[tokio::test]
async fn test_weekly_page_returns_html() {
    let repo = InMemoryRepository::default();
    *repo.top_threads_with_weekly.lock().unwrap() = vec![make_top_thread_with_weekly(42, 7, "eng")];

    let resp = process(&repo, make_get("/record/weekly")).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    assert!(body.contains("Top threads"));
    assert!(body.contains("W 7"));
    assert!(body.contains("id=\"filter-form\""));
    assert!(body.contains("/filter.js"));
    assert!(body.contains("/tabs-loader.js"));
    assert!(body.contains("/view-preferences.js"));
}

#[tokio::test]
async fn test_weekly_page_with_trailing_slash_returns_tabs() {
    let repo = InMemoryRepository::default();
    *repo.top_threads_with_weekly.lock().unwrap() = vec![make_top_thread_with_weekly(42, 7, "eng")];

    let resp = process(&repo, make_get("/record/weekly/")).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    assert!(body.contains("Top threads"));
    assert!(body.contains("This week"));
    assert!(body.contains("This month"));
}

#[tokio::test]
async fn test_weekly_page_week_tab_has_no_position_change_badge() {
    let repo = InMemoryRepository::default();
    *repo.weekly_ranked_threads.lock().unwrap() = vec![make_ranked_thread(13, "eng")];

    let resp = process(&repo, make_get("/record/weekly?tab=week"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    assert!(body.contains("This week"));
    assert!(!body.contains("rank-new"));
    assert!(!body.contains("rank-up"));
    assert!(!body.contains("rank-down"));
    assert!(body.contains("id=\"filter-form\""));
}

#[tokio::test]
async fn test_weekly_page_month_tab_has_no_position_change_badge() {
    let repo = InMemoryRepository::default();
    *repo.monthly_ranked_threads.lock().unwrap() = vec![make_ranked_thread(21, "general")];

    let resp = process(&repo, make_get("/record/weekly?tab=month"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    assert!(body.contains("This month"));
    assert!(!body.contains("rank-new"));
    assert!(!body.contains("rank-up"));
    assert!(!body.contains("rank-down"));
}

#[tokio::test]
async fn test_weekly_top_page_renders_more_than_fifty_cards_when_available() {
    let repo = InMemoryRepository::default();
    *repo.top_threads_with_weekly.lock().unwrap() = (0..60)
        .map(|i| make_top_thread_with_weekly(100 - i, i, &format!("ch{i}")))
        .collect();

    let resp = process(&repo, make_get("/record/weekly?tab=top"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body = String::from_utf8(resp.into_body().to_vec()).unwrap();
    assert_eq!(count_occurrences(&body, "class=\"thread-card\""), 60);
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
