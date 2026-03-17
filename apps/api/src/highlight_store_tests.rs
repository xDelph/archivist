use super::{HighlightedThreadRecord, LocalHighlightStore};
use tempfile::NamedTempFile;

fn highlighted_thread(
    thread_id: &str,
    pinned_by_user_id: &str,
    pinned_at: &str,
) -> HighlightedThreadRecord {
    let (channel_id, root_ts) = thread_id.split_once(':').expect("thread id");
    HighlightedThreadRecord {
        thread_id: thread_id.to_owned(),
        channel_id: channel_id.to_owned(),
        root_ts: root_ts.to_owned(),
        pinned_by_user_id: pinned_by_user_id.to_owned(),
        pinned_at: pinned_at.to_owned(),
    }
}

#[tokio::test]
async fn local_highlight_store_pins_lists_and_unpins_threads() {
    let file = NamedTempFile::new().expect("file");
    let store = LocalHighlightStore::open(file.path()).await.expect("store");

    store
        .pin_thread(highlighted_thread(
            "C123:1700000000.000001",
            "U-admin",
            "10",
        ))
        .await
        .expect("pin first");
    store
        .pin_thread(highlighted_thread(
            "C234:1700000000.000002",
            "U-editor",
            "20",
        ))
        .await
        .expect("pin second");

    assert_eq!(
        store.list_threads().await,
        vec![
            highlighted_thread("C234:1700000000.000002", "U-editor", "20"),
            highlighted_thread("C123:1700000000.000001", "U-admin", "10"),
        ]
    );

    assert!(
        store
            .unpin_thread("C123:1700000000.000001")
            .await
            .expect("unpin")
    );
    assert_eq!(
        store.list_threads().await,
        vec![highlighted_thread(
            "C234:1700000000.000002",
            "U-editor",
            "20"
        )]
    );
}

#[tokio::test]
async fn local_highlight_store_persists_pinned_threads() {
    let file = NamedTempFile::new().expect("file");
    let first_store = LocalHighlightStore::open(file.path()).await.expect("store");
    first_store
        .pin_thread(highlighted_thread(
            "C123:1700000000.000001",
            "U-admin",
            "10",
        ))
        .await
        .expect("pin");

    let second_store = LocalHighlightStore::open(file.path()).await.expect("store");
    assert_eq!(
        second_store.list_threads().await,
        vec![highlighted_thread(
            "C123:1700000000.000001",
            "U-admin",
            "10"
        )]
    );
}
