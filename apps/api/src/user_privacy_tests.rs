use super::{ANONYMOUS_DISPLAY_NAME, mask_synced_user};
use crate::user_store::SyncedUserRecord;

#[test]
fn mask_synced_user_leaves_active_users_unchanged() {
    let record = SyncedUserRecord {
        slack_user_id: "U123".to_owned(),
        display_name: Some("Thomas".to_owned()),
        avatar_url: Some("https://example.com/a.png".to_owned()),
        is_active: true,
        is_anonymized: false,
    };

    assert_eq!(mask_synced_user(record.clone()), record);
}

#[test]
fn mask_synced_user_replaces_profile_for_anonymized_users() {
    let masked = mask_synced_user(SyncedUserRecord {
        slack_user_id: "U123".to_owned(),
        display_name: Some("Thomas".to_owned()),
        avatar_url: Some("https://example.com/a.png".to_owned()),
        is_active: true,
        is_anonymized: true,
    });

    assert_eq!(masked.display_name.as_deref(), Some(ANONYMOUS_DISPLAY_NAME));
    assert_eq!(masked.avatar_url, None);
    assert!(masked.is_active);
    assert_eq!(masked.slack_user_id, "U123");
}
