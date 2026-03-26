use db::ThreadSummaryRow;
use domain::Message;

const MAX_LANGUAGE_MESSAGES: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LanguageHint {
    pub(crate) code: &'static str,
    pub(crate) label: &'static str,
    stopwords: &'static [&'static str],
    marker_chars: &'static [char],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LanguageGuess {
    pub(crate) hint: &'static LanguageHint,
    score: usize,
}

const ENGLISH_STOPWORDS: &[&str] = &[
    "the", "and", "to", "for", "with", "that", "this", "from", "not", "you", "are", "is", "we",
    "it", "on", "in", "should", "can",
];
const FRENCH_STOPWORDS: &[&str] = &[
    "le", "la", "les", "des", "une", "un", "et", "pour", "avec", "que", "qui", "est", "dans",
    "sur", "pas", "plus", "nous", "vous", "ce", "cette", "ces", "de", "du",
];
const SPANISH_STOPWORDS: &[&str] = &[
    "el", "la", "los", "las", "una", "un", "y", "para", "con", "que", "como", "por", "del", "está",
    "esta", "pero", "más", "porque",
];
const GERMAN_STOPWORDS: &[&str] = &[
    "der", "die", "das", "und", "mit", "eine", "ein", "ist", "nicht", "für", "den", "dem", "auf",
    "wie", "wir", "ihr",
];
const ITALIAN_STOPWORDS: &[&str] = &[
    "il", "lo", "la", "gli", "le", "un", "una", "e", "per", "con", "che", "non", "più", "come",
    "del", "della", "nel",
];
const PORTUGUESE_STOPWORDS: &[&str] = &[
    "o", "a", "os", "as", "uma", "um", "e", "para", "com", "que", "não", "por", "mais", "esta",
    "está", "dos", "das",
];

const LANGUAGE_HINTS: &[LanguageHint] = &[
    LanguageHint {
        code: "en",
        label: "English",
        stopwords: ENGLISH_STOPWORDS,
        marker_chars: &[],
    },
    LanguageHint {
        code: "fr",
        label: "French",
        stopwords: FRENCH_STOPWORDS,
        marker_chars: &[
            'à', 'â', 'ç', 'é', 'è', 'ê', 'ë', 'î', 'ï', 'ô', 'ù', 'û', 'ü', 'œ',
        ],
    },
    LanguageHint {
        code: "es",
        label: "Spanish",
        stopwords: SPANISH_STOPWORDS,
        marker_chars: &['á', 'é', 'í', 'ó', 'ú', 'ñ', '¿', '¡'],
    },
    LanguageHint {
        code: "de",
        label: "German",
        stopwords: GERMAN_STOPWORDS,
        marker_chars: &['ä', 'ö', 'ü', 'ß'],
    },
    LanguageHint {
        code: "it",
        label: "Italian",
        stopwords: ITALIAN_STOPWORDS,
        marker_chars: &['à', 'è', 'é', 'ì', 'í', 'ò', 'ó', 'ù'],
    },
    LanguageHint {
        code: "pt",
        label: "Portuguese",
        stopwords: PORTUGUESE_STOPWORDS,
        marker_chars: &['ã', 'õ', 'á', 'â', 'ê', 'ç', 'é', 'í', 'ó', 'ú'],
    },
];

pub(crate) fn build_language_requirement(
    language_hint: Option<&'static LanguageHint>,
    retrying_after_language_mismatch: bool,
) -> String {
    match language_hint {
        Some(language_hint) => {
            let retry_notice = retrying_after_language_mismatch.then(|| {
                format!(
                    "- Previous attempt used the wrong language. Regenerate it strictly in {}.\n",
                    language_hint.label
                )
            });
            format!(
                "Language requirement:\n\
                 - The dominant language of this thread is {}.\n\
                 - You MUST write summary, full_summary, and why_it_mattered entirely in {}.\n\
                 - Never translate them to English unless {} is English.\n\
                 - If the messages mix languages, prefer the language of the root message unless the remainder clearly dominates.\n\
                 {}\
                 \n",
                language_hint.label,
                language_hint.label,
                language_hint.label,
                retry_notice.unwrap_or_default()
            )
        }
        None => "Language requirement:\n\
                 - Detect the dominant language from the provided messages.\n\
                 - Write summary, full_summary, and why_it_mattered entirely in that same dominant language.\n\
                 - Never translate them to English unless the thread is actually in English.\n\
                 \n"
        .to_owned(),
    }
}

pub(crate) fn detect_thread_language(
    summary: &ThreadSummaryRow,
    messages: &[Message],
) -> Option<&'static LanguageHint> {
    let root_guess = messages
        .iter()
        .find(|message| message.ts == summary.root_ts)
        .and_then(|message| detect_language_guess(&message.text));
    let transcript = messages
        .iter()
        .take(MAX_LANGUAGE_MESSAGES)
        .map(|message| message.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let transcript_guess = detect_language_guess(&transcript);

    match (root_guess, transcript_guess) {
        (Some(root_guess), Some(transcript_guess))
            if root_guess.hint.code == transcript_guess.hint.code =>
        {
            Some(root_guess.hint)
        }
        (Some(root_guess), Some(transcript_guess))
            if root_guess.score + 1 >= transcript_guess.score =>
        {
            Some(root_guess.hint)
        }
        (Some(_), Some(transcript_guess)) => Some(transcript_guess.hint),
        (Some(root_guess), None) => Some(root_guess.hint),
        (None, Some(transcript_guess)) => Some(transcript_guess.hint),
        (None, None) => None,
    }
}

pub(crate) fn detect_language_guess(text: &str) -> Option<LanguageGuess> {
    let normalized = text
        .chars()
        .flat_map(char::to_lowercase)
        .collect::<String>();
    let tokens = normalized
        .split(|character: char| !character.is_alphabetic())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut best_guess = None::<LanguageGuess>;
    let mut second_best_score = 0usize;

    for hint in LANGUAGE_HINTS {
        let stopword_score = tokens
            .iter()
            .filter(|token| hint.stopwords.contains(token))
            .count();
        let marker_score = normalized
            .chars()
            .filter(|character| hint.marker_chars.contains(character))
            .count()
            * 2;
        let score = stopword_score + marker_score;
        if score == 0 {
            continue;
        }

        if best_guess.is_none_or(|current| score > current.score) {
            second_best_score = best_guess.map_or(second_best_score, |current| current.score);
            best_guess = Some(LanguageGuess { hint, score });
        } else if score > second_best_score {
            second_best_score = score;
        }
    }

    best_guess.filter(|guess| guess.score >= 2 && guess.score > second_best_score)
}

pub(crate) fn is_output_language_mismatch(
    expected: Option<&'static LanguageHint>,
    summary: &str,
    why_it_mattered: Option<&str>,
    full_summary: Option<&str>,
) -> bool {
    let Some(expected) = expected else {
        return false;
    };

    let mut combined = summary.to_owned();
    if let Some(why_it_mattered) = why_it_mattered {
        combined.push(' ');
        combined.push_str(why_it_mattered);
    }
    if let Some(full_summary) = full_summary {
        combined.push(' ');
        combined.push_str(full_summary);
    }

    detect_language_guess(&combined).is_some_and(|actual| actual.hint.code != expected.code)
}
