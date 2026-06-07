use crate::user_store::SyncedUserRecord;

pub(crate) const ANONYMOUS_DISPLAY_NAME: &str = "anonymous";

pub(crate) fn mask_synced_user(record: SyncedUserRecord) -> SyncedUserRecord {
    if !record.is_anonymized {
        return record;
    }

    SyncedUserRecord {
        display_name: Some(ANONYMOUS_DISPLAY_NAME.to_owned()),
        avatar_url: None,
        ..record
    }
}

#[cfg(test)]
#[path = "user_privacy_tests.rs"]
mod tests;
