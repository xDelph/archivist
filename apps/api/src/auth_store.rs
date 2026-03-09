use crate::auth::SlackIdentityResponse;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{fs, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AuthIdentity {
    pub(crate) slack_user_id: String,
    pub(crate) team_id: String,
    pub(crate) display_name: Option<String>,
    pub(crate) avatar_url: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalAuthStore {
    path: Arc<PathBuf>,
    state: Arc<Mutex<HashMap<(String, String), AuthIdentity>>>,
}

#[derive(Debug, Error)]
pub(crate) enum AuthStoreError {
    #[error("failed to read auth store")]
    Read(#[source] std::io::Error),
    #[error("failed to parse auth store")]
    Parse(#[from] serde_json::Error),
    #[error("failed to create auth store directory")]
    CreateDirectory(#[source] std::io::Error),
    #[error("failed to write auth store")]
    Write(#[source] std::io::Error),
}

impl LocalAuthStore {
    pub(crate) async fn open(path: impl AsRef<Path>) -> Result<Self, AuthStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = load_state(&path).await?;

        Ok(Self {
            path: Arc::new(path),
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub(crate) async fn upsert_identity(
        &self,
        identity: &SlackIdentityResponse,
    ) -> Result<(), AuthStoreError> {
        let mut state = self.state.lock().await;
        state.insert(
            (identity.team_id.clone(), identity.slack_user_id.clone()),
            AuthIdentity {
                slack_user_id: identity.slack_user_id.clone(),
                team_id: identity.team_id.clone(),
                display_name: identity.display_name.clone(),
                avatar_url: identity.avatar_url.clone(),
            },
        );
        persist_state(&self.path, &state).await
    }

    #[cfg(test)]
    pub(crate) async fn identities(&self) -> Vec<AuthIdentity> {
        let state = self.state.lock().await;
        let mut identities = state.values().cloned().collect::<Vec<_>>();
        identities.sort_by(|left, right| {
            (&left.team_id, &left.slack_user_id).cmp(&(&right.team_id, &right.slack_user_id))
        });
        identities
    }
}

async fn load_state(
    path: &Path,
) -> Result<HashMap<(String, String), AuthIdentity>, AuthStoreError> {
    if !fs::try_exists(path).await.map_err(AuthStoreError::Read)? {
        return Ok(HashMap::new());
    }

    let contents = fs::read(path).await.map_err(AuthStoreError::Read)?;
    if contents.is_empty() {
        return Ok(HashMap::new());
    }

    let identities: Vec<AuthIdentity> = serde_json::from_slice(&contents)?;
    Ok(identities
        .into_iter()
        .map(|identity| {
            (
                (identity.team_id.clone(), identity.slack_user_id.clone()),
                identity,
            )
        })
        .collect())
}

async fn persist_state(
    path: &Path,
    state: &HashMap<(String, String), AuthIdentity>,
) -> Result<(), AuthStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(AuthStoreError::CreateDirectory)?;
    }

    let mut identities = state.values().cloned().collect::<Vec<_>>();
    identities.sort_by(|left, right| {
        (&left.team_id, &left.slack_user_id).cmp(&(&right.team_id, &right.slack_user_id))
    });
    let payload = serde_json::to_vec_pretty(&identities)?;
    fs::write(path, payload)
        .await
        .map_err(AuthStoreError::Write)
}

#[cfg(test)]
#[path = "auth_store_tests.rs"]
mod tests;
