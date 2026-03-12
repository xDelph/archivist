#![cfg_attr(not(test), allow(dead_code))]

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
#[cfg(test)]
use std::path::PathBuf;
use std::{collections::HashMap, path::Path, sync::Arc};
use thiserror::Error;
use tokio::{fs, sync::Mutex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SyncedUserRecord {
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

#[derive(Debug, Clone)]
pub(crate) struct PostgresUserStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub(crate) enum UserStore {
    Local(LocalUserStore),
    Postgres(PostgresUserStore),
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
        match state
            .iter_mut()
            .find(|existing| existing.slack_user_id == user.slack_user_id)
        {
            Some(existing) => *existing = user,
            None => state.push(user),
        }
        state.sort_by(|left, right| left.slack_user_id.cmp(&right.slack_user_id));
        persist_state(&self.path, &state).await
    }

    pub(crate) async fn find_user(&self, slack_user_id: &str) -> Option<SyncedUserRecord> {
        self.state
            .lock()
            .await
            .iter()
            .find(|user| user.slack_user_id == slack_user_id)
            .cloned()
    }

    #[cfg(test)]
    pub(crate) async fn users(&self) -> Vec<SyncedUserRecord> {
        self.state.lock().await.clone()
    }
}

#[cfg_attr(test, allow(dead_code))]
impl PostgresUserStore {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserStore {
    pub(crate) async fn find_user(&self, slack_user_id: &str) -> Option<SyncedUserRecord> {
        match self {
            Self::Local(store) => store.find_user(slack_user_id).await,
            Self::Postgres(store) => sqlx::query(
                r#"
                SELECT id, display_name, avatar_url, is_active
                FROM users
                WHERE id = $1
                "#,
            )
            .bind(slack_user_id)
            .fetch_optional(&store.pool)
            .await
            .ok()
            .flatten()
            .map(|row| SyncedUserRecord {
                slack_user_id: row.get("id"),
                display_name: row.get("display_name"),
                avatar_url: row.get("avatar_url"),
                is_active: row.get("is_active"),
            }),
        }
    }

    pub(crate) async fn find_users(
        &self,
        slack_user_ids: &[String],
    ) -> HashMap<String, SyncedUserRecord> {
        if slack_user_ids.is_empty() {
            return HashMap::new();
        }

        match self {
            Self::Local(store) => {
                let lookup = slack_user_ids
                    .iter()
                    .cloned()
                    .collect::<std::collections::HashSet<_>>();
                store
                    .state
                    .lock()
                    .await
                    .iter()
                    .filter(|user| lookup.contains(&user.slack_user_id))
                    .cloned()
                    .map(|user| (user.slack_user_id.clone(), user))
                    .collect()
            }
            Self::Postgres(store) => sqlx::query(
                r#"
                SELECT id, display_name, avatar_url, is_active
                FROM users
                WHERE id = ANY($1)
                "#,
            )
            .bind(slack_user_ids)
            .fetch_all(&store.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| {
                let user = SyncedUserRecord {
                    slack_user_id: row.get("id"),
                    display_name: row.get("display_name"),
                    avatar_url: row.get("avatar_url"),
                    is_active: row.get("is_active"),
                };
                (user.slack_user_id.clone(), user)
            })
            .collect(),
        }
    }
}

impl From<LocalUserStore> for UserStore {
    fn from(value: LocalUserStore) -> Self {
        Self::Local(value)
    }
}

impl From<PostgresUserStore> for UserStore {
    fn from(value: PostgresUserStore) -> Self {
        Self::Postgres(value)
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
