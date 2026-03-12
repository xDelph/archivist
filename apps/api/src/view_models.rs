use crate::user_store::SyncedUserRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct UserSummaryResponse {
    pub(crate) slack_user_id: String,
    pub(crate) display_name: Option<String>,
    pub(crate) avatar_url: Option<String>,
}

impl From<SyncedUserRecord> for UserSummaryResponse {
    fn from(value: SyncedUserRecord) -> Self {
        Self {
            slack_user_id: value.slack_user_id,
            display_name: value.display_name,
            avatar_url: value.avatar_url,
        }
    }
}
