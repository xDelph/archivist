use super::{
    GeneratedSummaryPayload, build_prompt, normalize_generated_payload_key,
    repair_generated_summary_payload,
};
use crate::ai_openrouter_language::{
    detect_language_guess, detect_thread_language, is_output_language_mismatch,
};
use db::ThreadSummaryRow;
use domain::Message;

#[test]
fn repair_generated_summary_payload_normalizes_common_bad_keys() {
    let raw = r###"{
  "summary: ": "A concise summary.",
  "full-summary": "## Details\nA longer recap.",
  "why_it_mattered": null,
  "status": "unresolved",
  "topic_tags": ["ollama", "configuration"]
}"###;

    let repaired = repair_generated_summary_payload(raw);
    let payload =
        serde_json::from_str::<GeneratedSummaryPayload>(&repaired).expect("payload should parse");

    assert_eq!(payload.summary, "A concise summary.");
    assert_eq!(
        payload.full_summary.as_deref(),
        Some("## Details\nA longer recap.")
    );
    assert_eq!(payload.status.as_deref(), Some("unresolved"));
    assert_eq!(
        payload.topic_tags.expect("topic tags"),
        vec!["ollama".to_owned(), "configuration".to_owned()]
    );
}

#[test]
fn normalize_generated_payload_key_strips_trailing_colons() {
    assert_eq!(normalize_generated_payload_key("summary: "), "summary");
    assert_eq!(
        normalize_generated_payload_key("why-it-mattered:"),
        "why_it_mattered"
    );
    assert_eq!(
        normalize_generated_payload_key("full-summary"),
        "full_summary"
    );
}

#[test]
fn detect_thread_language_prefers_the_root_message_language() {
    let summary = sample_thread_summary("1700000000.000001");
    let messages = vec![
        sample_message(
            "1700000000.000001",
            None,
            "Bonjour, est-ce qu'on garde cette configuration pour le déploiement ?",
        ),
        sample_message(
            "1700000000.000002",
            Some("1700000000.000001"),
            "Oui, on garde cette approche et on documente la procédure pour l'équipe.",
        ),
        sample_message(
            "1700000000.000003",
            Some("1700000000.000001"),
            "LGTM, let's ship it once the checklist is updated.",
        ),
    ];

    let detected = detect_thread_language(&summary, &messages).expect("language");
    assert_eq!(detected.code, "fr");
    assert_eq!(detected.label, "French");
}

#[test]
fn build_prompt_includes_a_strict_language_requirement_when_detected() {
    let summary = sample_thread_summary("1700000000.000001");
    let messages = vec![
        sample_message(
            "1700000000.000001",
            None,
            "Bonjour, est-ce qu'on garde cette configuration pour le déploiement ?",
        ),
        sample_message(
            "1700000000.000002",
            Some("1700000000.000001"),
            "Oui, on garde cette approche et on documente la procédure pour l'équipe.",
        ),
    ];
    let language_hint = detect_thread_language(&summary, &messages);

    let prompt = build_prompt(&summary, &messages, language_hint, true);

    assert!(prompt.contains("The dominant language of this thread is French."));
    assert!(
        prompt.contains(
            "You MUST write summary, full_summary, and why_it_mattered entirely in French."
        )
    );
    assert!(prompt.contains("Previous attempt used the wrong language."));
    assert!(prompt.contains("Return a JSON object with keys: summary, full_summary, why_it_mattered, status, topic_tags."));
    assert!(prompt.contains("Keep the text generic and self-contained, as if it could be published outside the chat context."));
    assert!(prompt.contains("Avoid phrases like \"this thread\", \"the discussion\", \"someone asked\", \"a user said\", \"in Slack\", \"in the channel\""));
    assert!(
        prompt.contains(
            "do not mention people unless their identity materially matters to the outcome"
        )
    );
}

#[test]
fn detects_when_generated_output_uses_the_wrong_language() {
    let expected = detect_language_guess(
        "Bonjour, est-ce qu'on garde cette configuration pour le déploiement ?",
    )
    .expect("expected language")
    .hint;

    assert!(is_output_language_mismatch(
        Some(expected),
        "This thread settled on the deployment configuration and a short rollout checklist.",
        Some("It captures the final next steps for shipping."),
        Some("## Decision\nThe team kept the deployment setup and documented the rollout."),
    ));
    assert!(!is_output_language_mismatch(
        Some(expected),
        "La configuration de déploiement est validée avec une courte checklist de mise en ligne.",
        Some("Elle fixe les prochaines étapes pour l'équipe."),
        Some("## Décision\nL'équipe conserve la configuration et documente la mise en ligne."),
    ));
}

fn sample_thread_summary(root_ts: &str) -> ThreadSummaryRow {
    ThreadSummaryRow {
        channel_id: "C123".to_owned(),
        root_ts: root_ts.to_owned(),
        reply_count: 2,
        participant_count: 2,
        reaction_count: 0,
        file_count: 0,
        root_message_at: root_ts.to_owned(),
        last_activity_ts: "1700000000.000003".to_owned(),
    }
}

fn sample_message(ts: &str, thread_ts: Option<&str>, text: &str) -> Message {
    Message {
        channel_id: "C123".to_owned(),
        ts: ts.to_owned(),
        thread_ts: thread_ts.map(str::to_owned),
        user_id: Some("U123".to_owned()),
        text: text.to_owned(),
    }
}
