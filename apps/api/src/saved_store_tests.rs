use super::{LocalSavedItemStore, SavedItemRecord};
use tempfile::tempdir;

#[tokio::test]
async fn local_saved_item_store_upserts_lists_and_removes_items() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("saved-items.json");
    let store = LocalSavedItemStore::open(&path).await.expect("store");

    store
        .upsert_item(SavedItemRecord {
            slack_user_id: "U123".to_owned(),
            thread_id: "C123:1700000000.000001".to_owned(),
            channel_id: "C123".to_owned(),
            root_ts: "1700000000.000001".to_owned(),
            title: "Launch update".to_owned(),
            preview: "We should ship it".to_owned(),
            last_activity_ts: "1700000000.000010".to_owned(),
            saved_at: "2026-03-09T10:00:00Z".to_owned(),
        })
        .await
        .expect("first save");
    store
        .upsert_item(SavedItemRecord {
            slack_user_id: "U123".to_owned(),
            thread_id: "C456:1700000000.000002".to_owned(),
            channel_id: "C456".to_owned(),
            root_ts: "1700000000.000002".to_owned(),
            title: "Design sync".to_owned(),
            preview: "New comments landed".to_owned(),
            last_activity_ts: "1700000000.000020".to_owned(),
            saved_at: "2026-03-09T11:00:00Z".to_owned(),
        })
        .await
        .expect("second save");

    let reopened = LocalSavedItemStore::open(&path).await.expect("reopened");
    let items = reopened.list_items("U123").await;

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].thread_id, "C456:1700000000.000002");

    assert!(
        reopened
            .remove_item("U123", "C456:1700000000.000002")
            .await
            .expect("remove")
    );
    assert_eq!(reopened.list_items("U123").await.len(), 1);
}
