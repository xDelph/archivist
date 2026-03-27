const NO_TEXT_PLACEHOLDER: &str = "(no text)";
const MAX_PREVIEW_CHARS: usize = 700;

pub(crate) fn thread_preview_text(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").trim().to_owned();
    if normalized.is_empty() {
        return NO_TEXT_PLACEHOLDER.to_owned();
    }

    truncate_text(&normalized, MAX_PREVIEW_CHARS)
}

pub(crate) fn normalize_thread_text(value: &str) -> String {
    thread_preview_text(value)
}

fn truncate_text(value: &str, limit: usize) -> String {
    let mut truncated = value.chars().take(limit).collect::<String>();
    if value.chars().count() > limit {
        truncated.push('…');
    }

    truncated
}

#[cfg(test)]
#[path = "thread_text_tests.rs"]
mod tests;
