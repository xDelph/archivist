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

pub const fn search_documents_table() -> &'static str {
    "search_documents"
}

#[cfg(test)]
mod tests {
    use super::{SearchBackend, search_documents_table};

    #[test]
    fn placeholder_backend_is_named() {
        assert_eq!(
            SearchBackend::PostgresTsvectorPlaceholder.as_str(),
            "postgres_tsvector_placeholder"
        );
        assert_eq!(search_documents_table(), "search_documents");
    }
}
