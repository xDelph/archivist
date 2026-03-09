pub const fn create_search_documents_table_query() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS search_documents (
    team_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    message_ts TEXT NOT NULL,
    title TEXT,
    body TEXT NOT NULL,
    document TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('english', trim(concat_ws(' ', coalesce(title, ''), body)))
    ) STORED,
    PRIMARY KEY (team_id, channel_id, message_ts)
);

CREATE INDEX IF NOT EXISTS search_documents_document_idx
ON search_documents
USING GIN (document);

CREATE INDEX IF NOT EXISTS search_documents_channel_ts_idx
ON search_documents (team_id, channel_id, message_ts DESC);
"#
}

#[cfg(test)]
mod tests {
    use super::create_search_documents_table_query;

    #[test]
    fn search_document_schema_creates_tsvector_indexes() {
        let schema = create_search_documents_table_query();

        assert!(schema.contains("CREATE TABLE IF NOT EXISTS search_documents"));
        assert!(schema.contains("document TSVECTOR GENERATED ALWAYS AS"));
        assert!(schema.contains("to_tsvector('english'"));
        assert!(schema.contains("USING GIN (document)"));
        assert!(schema.contains("search_documents_channel_ts_idx"));
    }
}
