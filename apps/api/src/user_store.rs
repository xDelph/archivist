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
#[path = "user_store_tests.rs"]
mod tests;
