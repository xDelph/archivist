#![cfg_attr(not(test), allow(dead_code))]

use crate::auth::SlackIdentityResponse;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::{fs, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AuthIdentity {
    pub(crate) slack_user_id: String,
    pub(crate) display_name: Option<String>,
    pub(crate) avatar_url: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalAuthStore {
    path: Arc<PathBuf>,
    state: Arc<Mutex<HashMap<String, AuthIdentity>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresAuthStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub(crate) enum AuthStore {
    Local(LocalAuthStore),
    Postgres(PostgresAuthStore),
}

#[derive(Debug, Error)]
pub(crate) enum AuthStoreError {
    #[error("failed to read auth store")]
    Read(#[source] std::io::Error),
    #[error("failed to parse auth store")]
    Parse(#[from] serde_json::Error),
    #[error("failed to query auth store")]
    Sqlx(#[source] sqlx::Error),
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
            identity.slack_user_id.clone(),
            AuthIdentity {
                slack_user_id: identity.slack_user_id.clone(),
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
        identities.sort_by(|left, right| left.slack_user_id.cmp(&right.slack_user_id));
        identities
    }
}

#[cfg_attr(test, allow(dead_code))]
impl PostgresAuthStore {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AuthStore {
    pub(crate) async fn upsert_identity(
        &self,
        identity: &SlackIdentityResponse,
    ) -> Result<(), AuthStoreError> {
        match self {
            Self::Local(store) => store.upsert_identity(identity).await,
            Self::Postgres(store) => {
                sqlx::query(
                    r#"
                    INSERT INTO auth_identities (
                        slack_user_id,
                        email,
                        display_name,
                        avatar_url,
                        last_authenticated_at
                    )
                    VALUES ($1, $2, $3, $4, to_timestamp($5))
                    ON CONFLICT (slack_user_id) DO UPDATE
                    SET email = EXCLUDED.email,
                        display_name = EXCLUDED.display_name,
                        avatar_url = EXCLUDED.avatar_url,
                        last_authenticated_at = EXCLUDED.last_authenticated_at
                    "#,
                )
                .bind(&identity.slack_user_id)
                .bind(&identity.email)
                .bind(&identity.display_name)
                .bind(&identity.avatar_url)
                .bind(current_unix_timestamp() as f64)
                .execute(&store.pool)
                .await
                .map_err(AuthStoreError::Sqlx)?;
                Ok(())
            }
        }
    }
}

impl From<LocalAuthStore> for AuthStore {
    fn from(value: LocalAuthStore) -> Self {
        Self::Local(value)
    }
}

impl From<PostgresAuthStore> for AuthStore {
    fn from(value: PostgresAuthStore) -> Self {
        Self::Postgres(value)
    }
}

async fn load_state(path: &Path) -> Result<HashMap<String, AuthIdentity>, AuthStoreError> {
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
        .map(|identity| (identity.slack_user_id.clone(), identity))
        .collect())
}

async fn persist_state(
    path: &Path,
    state: &HashMap<String, AuthIdentity>,
) -> Result<(), AuthStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(AuthStoreError::CreateDirectory)?;
    }

    let mut identities = state.values().cloned().collect::<Vec<_>>();
    identities.sort_by(|left, right| left.slack_user_id.cmp(&right.slack_user_id));
    let payload = serde_json::to_vec_pretty(&identities)?;
    fs::write(path, payload)
        .await
        .map_err(AuthStoreError::Write)
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs() as i64
}

#[cfg(test)]
#[path = "auth_store_tests.rs"]
mod tests;
