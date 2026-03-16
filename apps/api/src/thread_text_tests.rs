use super::thread_preview_text;

#[test]
fn thread_preview_preserves_newlines() {
    assert_eq!(
        thread_preview_text("Salut\n\tService IA : Claude\nMerci"),
        "Salut\n\tService IA : Claude\nMerci"
    );
}
