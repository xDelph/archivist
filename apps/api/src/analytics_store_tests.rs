use super::*;
use tempfile::NamedTempFile;

fn test_event(event_type: &str, user_id: Option<&str>) -> AnalyticsEventRecord {
    AnalyticsEventRecord {
        event_type: event_type.to_owned(),
        user_id: user_id.map(|id| id.to_owned()),
        metadata: serde_json::json!({}),
        created_at: "2026-03-09T12:00:00Z".to_owned(),
    }
}

#[tokio::test]
async fn record_and_list_events() {
    let file = NamedTempFile::new().unwrap();
    let store = LocalAnalyticsStore::open(file.path()).await.unwrap();

    store
        .record_event(test_event("catch_up_open", Some("U001")))
        .await
        .unwrap();
    store
        .record_event(test_event("search_query", Some("U001")))
        .await
        .unwrap();
    store
        .record_event(test_event("thread_view", Some("U002")))
        .await
        .unwrap();

    let all = store.list_events(None, 100).await;
    assert_eq!(all.len(), 3);

    let searches = store.list_events(Some("search_query"), 100).await;
    assert_eq!(searches.len(), 1);
    assert_eq!(searches[0].event_type, "search_query");
}

#[tokio::test]
async fn count_by_type_aggregates() {
    let file = NamedTempFile::new().unwrap();
    let store = LocalAnalyticsStore::open(file.path()).await.unwrap();

    store
        .record_event(test_event("search_query", Some("U001")))
        .await
        .unwrap();
    store
        .record_event(test_event("search_query", Some("U002")))
        .await
        .unwrap();
    store
        .record_event(test_event("thread_view", Some("U001")))
        .await
        .unwrap();

    let counts = store.count_by_type().await;
    assert_eq!(counts.len(), 2);
    assert_eq!(counts[0], ("search_query".to_owned(), 2));
    assert_eq!(counts[1], ("thread_view".to_owned(), 1));
}

#[tokio::test]
async fn list_events_respects_limit() {
    let file = NamedTempFile::new().unwrap();
    let store = LocalAnalyticsStore::open(file.path()).await.unwrap();

    for i in 0..10 {
        store
            .record_event(test_event(&format!("event_{i}"), None))
            .await
            .unwrap();
    }

    let limited = store.list_events(None, 3).await;
    assert_eq!(limited.len(), 3);
}

#[tokio::test]
async fn persists_across_reopens() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_path_buf();

    {
        let store = LocalAnalyticsStore::open(&path).await.unwrap();
        store
            .record_event(test_event("catch_up_open", None))
            .await
            .unwrap();
    }

    let store = LocalAnalyticsStore::open(&path).await.unwrap();
    let events = store.list_events(None, 100).await;
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn opens_nonexistent_file_as_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");
    let store = LocalAnalyticsStore::open(&path).await.unwrap();
    let events = store.list_events(None, 100).await;
    assert!(events.is_empty());
}
