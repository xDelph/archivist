use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::path::PathBuf;
use std::{path::Path, sync::Arc};
use thiserror::Error;
use tokio::{fs, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SyncedUserRecord {
    pub(crate) team_id: String,
    pub(crate) slack_user_id: String,
    pub(crate) display_name: Option<String>,
    pub(crate) avatar_url: Option<String>,
    pub(crate) is_active: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalUserStore {
    #[cfg(test)]
    path: Arc<PathBuf>,
    state: Arc<Mutex<Vec<SyncedUserRecord>>>,
}

#[derive(Debug, Error)]
pub(crate) enum UserStoreError {
    #[error("failed to read synced users")]
    Read(#[source] std::io::Error),
    #[error("failed to parse synced users")]
    Parse(#[from] serde_json::Error),
    #[cfg(test)]
    #[error("failed to create synced user directory")]
    CreateDirectory(#[source] std::io::Error),
    #[cfg(test)]
    #[error("failed to write synced users")]
    Write(#[source] std::io::Error),
}

impl LocalUserStore {
    pub(crate) async fn open(path: impl AsRef<Path>) -> Result<Self, UserStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = load_state(&path).await?;

        Ok(Self {
            #[cfg(test)]
            path: Arc::new(path),
            state: Arc::new(Mutex::new(state)),
        })
    }

    #[cfg(test)]
    pub(crate) async fn upsert_user(&self, user: SyncedUserRecord) -> Result<(), UserStoreError> {
        let mut state = self.state.lock().await;
        match state.iter_mut().find(|existing| {
            existing.team_id == user.team_id && existing.slack_user_id == user.slack_user_id
        }) {
            Some(existing) => *existing = user,
            None => state.push(user),
        }
        state.sort_by(|left, right| {
            (&left.team_id, &left.slack_user_id).cmp(&(&right.team_id, &right.slack_user_id))
        });
        persist_state(&self.path, &state).await
    }

    pub(crate) async fn find_user(
        &self,
        team_id: &str,
        slack_user_id: &str,
    ) -> Option<SyncedUserRecord> {
        self.state
            .lock()
            .await
            .iter()
            .find(|user| user.team_id == team_id && user.slack_user_id == slack_user_id)
            .cloned()
    }

    #[cfg(test)]
    pub(crate) async fn users(&self) -> Vec<SyncedUserRecord> {
        self.state.lock().await.clone()
    }
}

async fn load_state(path: &Path) -> Result<Vec<SyncedUserRecord>, UserStoreError> {
    match fs::read(path).await {
        Ok(bytes) => {
            if bytes.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(serde_json::from_slice(&bytes)?)
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(UserStoreError::Read(error)),
    }
}

#[cfg(test)]
async fn persist_state(path: &Path, users: &[SyncedUserRecord]) -> Result<(), UserStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(UserStoreError::CreateDirectory)?;
    }
    let payload = serde_json::to_vec_pretty(users)?;
    fs::write(path, payload)
        .await
        .map_err(UserStoreError::Write)
}

#[cfg(test)]
mod tests {
    use super::{LocalUserStore, SyncedUserRecord};
    use tempfile::tempdir;

    #[tokio::test]
    async fn local_user_store_upserts_and_reloads_users() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("synced-users.json");
        let store = LocalUserStore::open(&path).await.expect("user store");

        store
            .upsert_user(SyncedUserRecord {
                team_id: "T123".to_owned(),
                slack_user_id: "U123".to_owned(),
                display_name: Some("Thomas".to_owned()),
                avatar_url: Some("https://images.example.com/avatar.png".to_owned()),
                is_active: true,
            })
            .await
            .expect("first upsert");
        store
            .upsert_user(SyncedUserRecord {
                team_id: "T123".to_owned(),
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
}
