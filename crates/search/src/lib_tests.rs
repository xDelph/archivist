use super::{
    SearchBackend, SearchQuery, SearchSort, normalize_query_text, ranked_search_query,
    search_documents_table, search_rank_expression,
};

#[test]
fn placeholder_backend_is_named() {
    assert_eq!(
        SearchBackend::PostgresTsvectorPlaceholder.as_str(),
        "postgres_tsvector_placeholder"
    );
    assert_eq!(search_documents_table(), "search_documents");
}

#[test]
fn query_text_is_normalized() {
    assert_eq!(
        normalize_query_text("  release    notes   search "),
        "release notes search"
    );
}

#[test]
fn search_query_defaults_to_relevance() {
    let query = SearchQuery::new("hello world");

    assert_eq!(query.text, "hello world");
    assert_eq!(query.sort, SearchSort::Relevance);
    assert!(query.filters.channel_ids.is_empty());
}

#[test]
fn ranked_search_sql_targets_search_documents() {
    let sql = ranked_search_query();

    assert!(sql.contains("FROM search_documents"));
    assert_eq!(
        search_rank_expression(),
        "ts_rank(document, websearch_to_tsquery('english', $1))"
    );
    assert!(sql.contains("AS score"));
    assert!(sql.contains("websearch_to_tsquery"));
    assert!(sql.contains("channel_id = ANY($2)"));
    assert!(
        sql.contains("split_part(message_ts, '.', 1)::bigint >= split_part($3, '.', 1)::bigint")
    );
    assert!(
        sql.contains("split_part(message_ts, '.', 1)::bigint <= split_part($4, '.', 1)::bigint")
    );
    assert!(sql.contains("ORDER BY score DESC"));
}
