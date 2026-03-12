use super::{LocalUserStore, SyncedUserRecord};
use tempfile::tempdir;

#[tokio::test]
async fn local_user_store_upserts_and_reloads_users() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("synced-users.json");
    let store = LocalUserStore::open(&path).await.expect("user store");

    store
        .upsert_user(SyncedUserRecord {
            slack_user_id: "U123".to_owned(),
            display_name: Some("Thomas".to_owned()),
            avatar_url: Some("https://images.example.com/avatar.png".to_owned()),
            is_active: true,
        })
        .await
        .expect("first upsert");
    store
        .upsert_user(SyncedUserRecord {
            slack_user_id: "U123".to_owned(),
            display_name: Some("Tom".to_owned()),
            avatar_url: Some("https://images.example.com/avatar-2.png".to_owned()),
            is_active: false,
        })
        .await
        .expect("second upsert");

    let reopened = LocalUserStore::open(&path)
        .await
        .expect("reopened user store");
    let users = reopened.users().await;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].display_name.as_deref(), Some("Tom"));
    assert!(!users[0].is_active);
}

#[tokio::test]
async fn local_user_store_finds_multiple_users_by_id() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("synced-users.json");
    let store = LocalUserStore::open(&path).await.expect("user store");

    for user in [
        SyncedUserRecord {
            slack_user_id: "U123".to_owned(),
            display_name: Some("Thomas".to_owned()),
            avatar_url: Some("https://images.example.com/avatar.png".to_owned()),
            is_active: true,
        },
        SyncedUserRecord {
            slack_user_id: "U456".to_owned(),
            display_name: Some("Greg".to_owned()),
            avatar_url: None,
            is_active: true,
        },
    ] {
        store.upsert_user(user).await.expect("upsert");
    }

    let users = super::UserStore::from(store)
        .find_users(&["U123".to_owned(), "U999".to_owned(), "U456".to_owned()])
        .await;

    assert_eq!(users.len(), 2);
    assert_eq!(
        users
            .get("U123")
            .and_then(|user| user.display_name.as_deref()),
        Some("Thomas")
    );
    assert_eq!(
        users
            .get("U456")
            .and_then(|user| user.display_name.as_deref()),
        Some("Greg")
    );
}
