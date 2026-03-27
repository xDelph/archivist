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
    Date,
    Replies,
    Reactions,
    People,
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
SELECT channel_id,
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
#[path = "lib_tests.rs"]
mod tests;
