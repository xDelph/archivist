use db::GeneratedThreadSummaryRow;
use std::collections::HashMap;

pub(crate) const AI_PREVIEW_SOURCE: &str = "ai";
pub(crate) const FALLBACK_PREVIEW_SOURCE: &str = "fallback";

pub(crate) struct ThreadPreview {
    pub(crate) text: Option<String>,
    pub(crate) source: &'static str,
}

pub(crate) fn build_generated_summary_lookup(
    rows: Vec<GeneratedThreadSummaryRow>,
) -> HashMap<(String, String), GeneratedThreadSummaryRow> {
    rows.into_iter()
        .map(|row| ((row.channel_id.clone(), row.root_ts.clone()), row))
        .collect()
}

pub(crate) fn resolve_thread_preview(
    generated_summaries: &HashMap<(String, String), GeneratedThreadSummaryRow>,
    channel_id: &str,
    root_ts: &str,
) -> ThreadPreview {
    let text = generated_summaries
        .get(&(channel_id.to_owned(), root_ts.to_owned()))
        .and_then(|summary| normalize_preview_text(&summary.summary));
    if let Some(text) = text {
        return ThreadPreview {
            text: Some(text),
            source: AI_PREVIEW_SOURCE,
        };
    }

    ThreadPreview {
        text: None,
        source: FALLBACK_PREVIEW_SOURCE,
    }
}

fn normalize_preview_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        AI_PREVIEW_SOURCE, FALLBACK_PREVIEW_SOURCE, build_generated_summary_lookup,
        resolve_thread_preview,
    };
    use db::GeneratedThreadSummaryRow;
    use std::collections::HashMap;

    #[test]
    fn resolve_thread_preview_prefers_generated_summary_text() {
        let lookup = build_generated_summary_lookup(vec![GeneratedThreadSummaryRow {
            channel_id: "C123".to_owned(),
            root_ts: "1700000000.000001".to_owned(),
            summary: "Short AI summary".to_owned(),
            full_summary: Some("## Full summary".to_owned()),
            why_it_mattered: None,
            status: "discussion".to_owned(),
            topic_tags: vec![],
            source_last_activity_ts: "1700000000.000001".to_owned(),
            model: "test".to_owned(),
            generated_at: 1,
        }]);

        let preview = resolve_thread_preview(&lookup, "C123", "1700000000.000001");

        assert_eq!(preview.text.as_deref(), Some("Short AI summary"));
        assert_eq!(preview.source, AI_PREVIEW_SOURCE);
    }

    #[test]
    fn resolve_thread_preview_falls_back_when_no_generated_summary_exists() {
        let preview = resolve_thread_preview(&HashMap::new(), "C123", "1700000000.000001");

        assert_eq!(preview.text, None);
        assert_eq!(preview.source, FALLBACK_PREVIEW_SOURCE);
    }
}
