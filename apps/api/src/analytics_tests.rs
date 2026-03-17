use super::*;
use crate::analytics_store::LocalAnalyticsStore;

async fn test_state() -> AppState {
    let analytics_file = tempfile::NamedTempFile::new().unwrap();
    let analytics_store = LocalAnalyticsStore::open(analytics_file.path())
        .await
        .unwrap();

    let event_file = tempfile::NamedTempFile::new().unwrap();
    let store = db::JsonlEventStore::open(event_file.path()).await.unwrap();

    let auth_file = tempfile::NamedTempFile::new().unwrap();
    let auth_store = crate::auth_store::LocalAuthStore::open(auth_file.path())
        .await
        .unwrap();

    let user_file = tempfile::NamedTempFile::new().unwrap();
    let user_store = crate::user_store::LocalUserStore::open(user_file.path())
        .await
        .unwrap();

    let user_role_file = tempfile::NamedTempFile::new().unwrap();
    let user_role_store = crate::user_role_store::LocalUserRoleStore::open(user_role_file.path())
        .await
        .unwrap();

    let saved_file = tempfile::NamedTempFile::new().unwrap();
    let saved_store = crate::saved_store::LocalSavedItemStore::open(saved_file.path())
        .await
        .unwrap();

    let highlight_file = tempfile::NamedTempFile::new().unwrap();
    let highlight_store = crate::highlight_store::LocalHighlightStore::open(highlight_file.path())
        .await
        .unwrap();

    AppState {
        store: store.into(),
        slack_auth: crate::auth::SlackAuthConfig {
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            team_id: None,
            token_url: None,
        },
        web_origin: crate::LOCAL_DEV_WEB_ORIGIN.to_owned(),
        session_secret: None,
        auth_store: auth_store.into(),
        user_store: user_store.into(),
        user_role_store: user_role_store.into(),
        highlight_store: highlight_store.into(),
        saved_store: saved_store.into(),
        analytics_store: analytics_store.into(),
    }
}

#[tokio::test]
async fn record_analytics_helper_does_not_panic() {
    let state = test_state().await;
    record_analytics(
        &state,
        "test_event",
        Some("U001"),
        serde_json::json!({"key": "value"}),
    );
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let events = state.analytics_store.list_events(None, 100).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "test_event");
}
