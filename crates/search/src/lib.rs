use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchBackend {
    PostgresTsvectorPlaceholder,
}

impl SearchBackend {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PostgresTsvectorPlaceholder => "postgres_tsvector_placeholder",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    Relevance,
    Newest,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub channel_ids: Vec<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub filters: SearchFilters,
    pub sort: SearchSort,
}

impl SearchQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: normalize_query_text(&text.into()),
            filters: SearchFilters::default(),
            sort: SearchSort::Relevance,
        }
    }
}

pub const fn search_documents_table() -> &'static str {
    "search_documents"
}

pub fn normalize_query_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub const fn search_rank_expression() -> &'static str {
    "ts_rank(document, websearch_to_tsquery('english', $1))"
}

pub const fn ranked_search_query() -> &'static str {
    r#"
SELECT team_id,
       channel_id,
       message_ts,
       title,
       body,
       ts_rank(document, websearch_to_tsquery('english', $1)) AS score
FROM search_documents
WHERE document @@ websearch_to_tsquery('english', $1)
  AND (coalesce(array_length($2::text[], 1), 0) = 0 OR channel_id = ANY($2))
  AND ($3::text IS NULL OR split_part(message_ts, '.', 1)::bigint >= split_part($3, '.', 1)::bigint)
  AND ($4::text IS NULL OR split_part(message_ts, '.', 1)::bigint <= split_part($4, '.', 1)::bigint)
ORDER BY score DESC,
         message_ts DESC
"#
}

#[cfg(test)]
mod tests {
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
        assert!(sql.contains("split_part(message_ts, '.', 1)::bigint >= split_part($3, '.', 1)::bigint"));
        assert!(sql.contains("split_part(message_ts, '.', 1)::bigint <= split_part($4, '.', 1)::bigint"));
        assert!(sql.contains("ORDER BY score DESC"));
    }
}
