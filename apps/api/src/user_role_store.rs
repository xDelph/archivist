#![cfg_attr(not(test), allow(dead_code))]

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{fs, sync::Mutex};

pub(crate) const ADMIN_ROLE: &str = "admin";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct UserRoleRecord {
    pub(crate) slack_user_id: String,
    pub(crate) role: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalUserRoleStore {
    path: Arc<PathBuf>,
    state: SharedUserRoleState,
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresUserRoleStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub(crate) enum UserRoleStore {
    Local(LocalUserRoleStore),
    Postgres(PostgresUserRoleStore),
}

type UserRoleState = HashMap<String, BTreeSet<String>>;
type SharedUserRoleState = Arc<Mutex<UserRoleState>>;

#[derive(Debug, Error)]
pub(crate) enum UserRoleStoreError {
    #[error("failed to read user roles")]
    Read(#[source] std::io::Error),
    #[error("failed to parse user roles")]
    Parse(#[from] serde_json::Error),
    #[error("failed to query user roles")]
    Sqlx(#[source] sqlx::Error),
    #[error("failed to create user role directory")]
    CreateDirectory(#[source] std::io::Error),
    #[error("failed to write user roles")]
    Write(#[source] std::io::Error),
}

impl LocalUserRoleStore {
    pub(crate) async fn open(path: impl AsRef<Path>) -> Result<Self, UserRoleStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = load_state(&path).await?;

        Ok(Self {
            path: Arc::new(path),
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub(crate) async fn list_roles(
        &self,
        slack_user_id: &str,
    ) -> Result<Vec<String>, UserRoleStoreError> {
        let state = self.state.lock().await;
        Ok(state
            .get(slack_user_id)
            .map(|roles| roles.iter().cloned().collect())
            .unwrap_or_default())
    }

    #[cfg(test)]
    pub(crate) async fn grant_role(
        &self,
        slack_user_id: &str,
        role: &str,
    ) -> Result<bool, UserRoleStoreError> {
        let Some(normalized_role) = normalize_role(role) else {
            return Ok(false);
        };

        let mut state = self.state.lock().await;
        let inserted = state
            .entry(slack_user_id.to_owned())
            .or_default()
            .insert(normalized_role);
        if inserted {
            persist_state(&self.path, &state).await?;
        }
        Ok(inserted)
    }

    #[cfg(test)]
    pub(crate) async fn revoke_role(
        &self,
        slack_user_id: &str,
        role: &str,
    ) -> Result<bool, UserRoleStoreError> {
        let Some(normalized_role) = normalize_role(role) else {
            return Ok(false);
        };

        let mut state = self.state.lock().await;
        let Some(roles) = state.get_mut(slack_user_id) else {
            return Ok(false);
        };
        let removed = roles.remove(&normalized_role);
        if roles.is_empty() {
            state.remove(slack_user_id);
        }
        if removed {
            persist_state(&self.path, &state).await?;
        }
        Ok(removed)
    }
}

#[cfg_attr(test, allow(dead_code))]
impl PostgresUserRoleStore {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserRoleStore {
    pub(crate) async fn list_roles(
        &self,
        slack_user_id: &str,
    ) -> Result<Vec<String>, UserRoleStoreError> {
        match self {
            Self::Local(store) => store.list_roles(slack_user_id).await,
            Self::Postgres(store) => {
                let mut roles = sqlx::query(
                    r#"
                    SELECT role
                    FROM user_roles
                    WHERE user_id = $1
                    ORDER BY role ASC
                    "#,
                )
                .bind(slack_user_id)
                .fetch_all(&store.pool)
                .await
                .map_err(UserRoleStoreError::Sqlx)?
                .into_iter()
                .map(|row| row.get::<String, _>("role"))
                .collect::<Vec<_>>();
                roles.sort();
                roles.dedup();
                Ok(roles)
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn grant_role(
        &self,
        slack_user_id: &str,
        role: &str,
    ) -> Result<bool, UserRoleStoreError> {
        match self {
            Self::Local(store) => store.grant_role(slack_user_id, role).await,
            Self::Postgres(store) => {
                let Some(normalized_role) = normalize_role(role) else {
                    return Ok(false);
                };

                Ok(sqlx::query(
                    r#"
                    INSERT INTO user_roles (user_id, role)
                    VALUES ($1, $2)
                    ON CONFLICT (user_id, role) DO NOTHING
                    "#,
                )
                .bind(slack_user_id)
                .bind(normalized_role)
                .execute(&store.pool)
                .await
                .map_err(UserRoleStoreError::Sqlx)?
                .rows_affected()
                    > 0)
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn revoke_role(
        &self,
        slack_user_id: &str,
        role: &str,
    ) -> Result<bool, UserRoleStoreError> {
        match self {
            Self::Local(store) => store.revoke_role(slack_user_id, role).await,
            Self::Postgres(store) => {
                let Some(normalized_role) = normalize_role(role) else {
                    return Ok(false);
                };

                Ok(sqlx::query(
                    r#"
                    DELETE FROM user_roles
                    WHERE user_id = $1
                      AND role = $2
                    "#,
                )
                .bind(slack_user_id)
                .bind(normalized_role)
                .execute(&store.pool)
                .await
                .map_err(UserRoleStoreError::Sqlx)?
                .rows_affected()
                    > 0)
            }
        }
    }

    pub(crate) async fn has_role(
        &self,
        slack_user_id: &str,
        role: &str,
    ) -> Result<bool, UserRoleStoreError> {
        let Some(normalized_role) = normalize_role(role) else {
            return Ok(false);
        };
        Ok(self
            .list_roles(slack_user_id)
            .await?
            .into_iter()
            .any(|candidate| candidate == normalized_role))
    }
}

impl From<LocalUserRoleStore> for UserRoleStore {
    fn from(value: LocalUserRoleStore) -> Self {
        Self::Local(value)
    }
}

impl From<PostgresUserRoleStore> for UserRoleStore {
    fn from(value: PostgresUserRoleStore) -> Self {
        Self::Postgres(value)
    }
}

async fn load_state(path: &Path) -> Result<UserRoleState, UserRoleStoreError> {
    if !fs::try_exists(path)
        .await
        .map_err(UserRoleStoreError::Read)?
    {
        return Ok(HashMap::new());
    }

    let contents = fs::read(path).await.map_err(UserRoleStoreError::Read)?;
    if contents.is_empty() {
        return Ok(HashMap::new());
    }

    let records: Vec<UserRoleRecord> = serde_json::from_slice(&contents)?;
    let mut state = HashMap::<String, BTreeSet<String>>::new();
    for record in records {
        if let Some(role) = normalize_role(&record.role) {
            state.entry(record.slack_user_id).or_default().insert(role);
        }
    }
    Ok(state)
}

async fn persist_state(path: &Path, state: &UserRoleState) -> Result<(), UserRoleStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(UserRoleStoreError::CreateDirectory)?;
    }

    let payload = serde_json::to_vec_pretty(&flatten_state(state))?;
    fs::write(path, payload)
        .await
        .map_err(UserRoleStoreError::Write)
}

fn flatten_state(state: &UserRoleState) -> Vec<UserRoleRecord> {
    let mut records = state
        .iter()
        .flat_map(|(slack_user_id, roles)| {
            roles.iter().map(|role| UserRoleRecord {
                slack_user_id: slack_user_id.clone(),
                role: role.clone(),
            })
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        (&left.slack_user_id, &left.role).cmp(&(&right.slack_user_id, &right.role))
    });
    records
}

fn normalize_role(role: &str) -> Option<String> {
    let normalized = role.trim().to_ascii_lowercase();
    (!normalized.is_empty()).then_some(normalized)
}

#[cfg(test)]
#[path = "user_role_store_tests.rs"]
mod tests;
