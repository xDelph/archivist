#![cfg_attr(not(test), allow(dead_code))]

use crate::user_privacy::mask_synced_user;
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
    #[serde(default)]
    pub(crate) is_anonymized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdminUserRecord {
    pub(crate) slack_user_id: String,
    pub(crate) email: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) avatar_url: Option<String>,
    pub(crate) is_active: bool,
    pub(crate) is_anonymized: bool,
    pub(crate) roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalUserStore {
    #[cfg(test)]
    path: Arc<PathBuf>,
    state: Arc<Mutex<Vec<SyncedUserRecord>>>,
    emails: Arc<Mutex<HashMap<String, String>>>,
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
    #[error("user not found")]
    NotFound,
    #[cfg(test)]
    #[error("failed to create synced user directory")]
    CreateDirectory(#[source] std::io::Error),
    #[cfg(test)]
    #[error("failed to write synced users")]
    Write(#[source] std::io::Error),
    #[error("failed to query synced users")]
    Sqlx(#[from] sqlx::Error),
}

impl LocalUserStore {
    pub(crate) async fn open(path: impl AsRef<Path>) -> Result<Self, UserStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = load_state(&path).await?;

        Ok(Self {
            #[cfg(test)]
            path: Arc::new(path),
            state: Arc::new(Mutex::new(state)),
            emails: Arc::new(Mutex::new(HashMap::new())),
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
            .map(mask_synced_user)
    }

    #[cfg(test)]
    pub(crate) async fn users(&self) -> Vec<SyncedUserRecord> {
        self.state.lock().await.clone()
    }

    async fn set_anonymized(
        &self,
        slack_user_id: &str,
        anonymized: bool,
    ) -> Result<(), UserStoreError> {
        let mut state = self.state.lock().await;
        let user = state
            .iter_mut()
            .find(|user| user.slack_user_id == slack_user_id)
            .ok_or(UserStoreError::NotFound)?;
        user.is_anonymized = anonymized;
        #[cfg(test)]
        persist_state(&self.path, &state).await?;
        Ok(())
    }

    async fn set_active(&self, slack_user_id: &str, active: bool) -> Result<(), UserStoreError> {
        let mut state = self.state.lock().await;
        let user = state
            .iter_mut()
            .find(|user| user.slack_user_id == slack_user_id)
            .ok_or(UserStoreError::NotFound)?;
        user.is_active = active;
        #[cfg(test)]
        persist_state(&self.path, &state).await?;
        Ok(())
    }

    async fn list_users(&self, query: Option<&str>, limit: usize) -> Vec<AdminUserRecord> {
        let normalized_query = query.map(str::trim).filter(|value| !value.is_empty());
        let emails = self.emails.lock().await;
        let mut users = self
            .state
            .lock()
            .await
            .iter()
            .filter(|user| {
                normalized_query.is_none_or(|needle| {
                    user.slack_user_id.contains(needle)
                        || user
                            .display_name
                            .as_deref()
                            .is_some_and(|name| name.contains(needle))
                        || emails
                            .get(&user.slack_user_id)
                            .is_some_and(|email| email.contains(needle))
                })
            })
            .map(|user| AdminUserRecord {
                slack_user_id: user.slack_user_id.clone(),
                email: emails.get(&user.slack_user_id).cloned(),
                display_name: user.display_name.clone(),
                avatar_url: user.avatar_url.clone(),
                is_active: user.is_active,
                is_anonymized: user.is_anonymized,
                roles: Vec::new(),
            })
            .collect::<Vec<_>>();
        users.sort_by(|left, right| left.slack_user_id.cmp(&right.slack_user_id));
        users.truncate(limit);
        users
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
        let record = match self {
            Self::Local(store) => store.find_user(slack_user_id).await,
            Self::Postgres(store) => fetch_synced_user(&store.pool, slack_user_id)
                .await
                .ok()
                .flatten(),
        };
        record.map(mask_synced_user)
    }

    pub(crate) async fn find_user_raw(&self, slack_user_id: &str) -> Option<SyncedUserRecord> {
        match self {
            Self::Local(store) => store
                .state
                .lock()
                .await
                .iter()
                .find(|user| user.slack_user_id == slack_user_id)
                .cloned(),
            Self::Postgres(store) => fetch_synced_user(&store.pool, slack_user_id)
                .await
                .ok()
                .flatten(),
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
                    .map(mask_synced_user)
                    .map(|user| (user.slack_user_id.clone(), user))
                    .collect()
            }
            Self::Postgres(store) => sqlx::query(
                r#"
                SELECT id, display_name, avatar_url, is_active, is_anonymized
                FROM users
                WHERE id = ANY($1)
                "#,
            )
            .bind(slack_user_ids)
            .fetch_all(&store.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| synced_user_from_row(&row))
            .map(mask_synced_user)
            .map(|user| (user.slack_user_id.clone(), user))
            .collect(),
        }
    }

    pub(crate) async fn set_anonymized(
        &self,
        slack_user_id: &str,
        anonymized: bool,
        initiated_by: Option<&str>,
    ) -> Result<(), UserStoreError> {
        match self {
            Self::Local(store) => store.set_anonymized(slack_user_id, anonymized).await,
            Self::Postgres(store) => {
                let rows = sqlx::query(
                    r#"
                    UPDATE users
                    SET is_anonymized = $2,
                        anonymized_at = CASE WHEN $2 THEN now() ELSE NULL END,
                        anonymized_by = CASE WHEN $2 THEN $3 ELSE NULL END
                    WHERE id = $1
                    "#,
                )
                .bind(slack_user_id)
                .bind(anonymized)
                .bind(initiated_by)
                .execute(&store.pool)
                .await?
                .rows_affected();
                if rows == 0 {
                    return Err(UserStoreError::NotFound);
                }
                Ok(())
            }
        }
    }

    pub(crate) async fn set_active(
        &self,
        slack_user_id: &str,
        active: bool,
    ) -> Result<(), UserStoreError> {
        match self {
            Self::Local(store) => store.set_active(slack_user_id, active).await,
            Self::Postgres(store) => {
                let rows = sqlx::query(
                    r#"
                    UPDATE users
                    SET is_active = $2
                    WHERE id = $1
                    "#,
                )
                .bind(slack_user_id)
                .bind(active)
                .execute(&store.pool)
                .await?
                .rows_affected();
                if rows == 0 {
                    return Err(UserStoreError::NotFound);
                }
                Ok(())
            }
        }
    }

    pub(crate) async fn list_users(
        &self,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<AdminUserRecord>, UserStoreError> {
        let capped_limit = limit.clamp(1, 200);
        match self {
            Self::Local(store) => Ok(store.list_users(query, capped_limit).await),
            Self::Postgres(store) => {
                let pattern = query
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| format!("%{value}%"));
                let rows = if let Some(pattern) = pattern {
                    sqlx::query(
                        r#"
                        SELECT
                            u.id,
                            u.email,
                            u.display_name,
                            u.avatar_url,
                            u.is_active,
                            u.is_anonymized,
                            COALESCE(array_agg(ur.role ORDER BY ur.role)
                                FILTER (WHERE ur.role IS NOT NULL), '{}') AS roles
                        FROM users u
                        LEFT JOIN user_roles ur ON ur.user_id = u.id
                        WHERE u.id ILIKE $1
                           OR u.email ILIKE $1
                           OR u.display_name ILIKE $1
                        GROUP BY u.id
                        ORDER BY u.display_name NULLS LAST, u.id ASC
                        LIMIT $2
                        "#,
                    )
                    .bind(pattern)
                    .bind(i64::try_from(capped_limit).unwrap_or(200))
                    .fetch_all(&store.pool)
                    .await?
                } else {
                    sqlx::query(
                        r#"
                        SELECT
                            u.id,
                            u.email,
                            u.display_name,
                            u.avatar_url,
                            u.is_active,
                            u.is_anonymized,
                            COALESCE(array_agg(ur.role ORDER BY ur.role)
                                FILTER (WHERE ur.role IS NOT NULL), '{}') AS roles
                        FROM users u
                        LEFT JOIN user_roles ur ON ur.user_id = u.id
                        GROUP BY u.id
                        ORDER BY u.display_name NULLS LAST, u.id ASC
                        LIMIT $1
                        "#,
                    )
                    .bind(i64::try_from(capped_limit).unwrap_or(200))
                    .fetch_all(&store.pool)
                    .await?
                };

                Ok(rows
                    .into_iter()
                    .map(|row| admin_user_from_row(&row))
                    .collect::<Vec<_>>())
            }
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

async fn fetch_synced_user(
    pool: &PgPool,
    slack_user_id: &str,
) -> Result<Option<SyncedUserRecord>, sqlx::Error> {
    sqlx::query(
        r#"
        SELECT id, display_name, avatar_url, is_active, is_anonymized
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(slack_user_id)
    .fetch_optional(pool)
    .await
    .map(|row| row.map(|row| synced_user_from_row(&row)))
}

fn synced_user_from_row(row: &sqlx::postgres::PgRow) -> SyncedUserRecord {
    SyncedUserRecord {
        slack_user_id: row.get("id"),
        display_name: row.get("display_name"),
        avatar_url: row.get("avatar_url"),
        is_active: row.get("is_active"),
        is_anonymized: row.get("is_anonymized"),
    }
}

fn admin_user_from_row(row: &sqlx::postgres::PgRow) -> AdminUserRecord {
    AdminUserRecord {
        slack_user_id: row.get("id"),
        email: row.get("email"),
        display_name: row.get("display_name"),
        avatar_url: row.get("avatar_url"),
        is_active: row.get("is_active"),
        is_anonymized: row.get("is_anonymized"),
        roles: row.get("roles"),
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
