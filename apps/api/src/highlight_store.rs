#![cfg_attr(not(test), allow(dead_code))]

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{fs, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct HighlightedThreadRecord {
    pub(crate) thread_id: String,
    pub(crate) channel_id: String,
    pub(crate) root_ts: String,
    pub(crate) pinned_by_user_id: String,
    pub(crate) pinned_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalHighlightStore {
    path: Arc<PathBuf>,
    state: SharedHighlightState,
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresHighlightStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub(crate) enum HighlightStore {
    Local(LocalHighlightStore),
    Postgres(PostgresHighlightStore),
}

type HighlightState = HashMap<String, HighlightedThreadRecord>;
type SharedHighlightState = Arc<Mutex<HighlightState>>;

#[cfg_attr(test, allow(dead_code))]
#[derive(Debug, Error)]
pub(crate) enum HighlightStoreError {
    #[error("failed to read highlighted threads")]
    Read(#[source] std::io::Error),
    #[error("failed to parse highlighted threads")]
    Parse(#[from] serde_json::Error),
    #[error("failed to query highlighted threads")]
    Sqlx(#[source] sqlx::Error),
    #[error("failed to create highlighted thread directory")]
    CreateDirectory(#[source] std::io::Error),
    #[error("failed to write highlighted threads")]
    Write(#[source] std::io::Error),
}

impl LocalHighlightStore {
    pub(crate) async fn open(path: impl AsRef<Path>) -> Result<Self, HighlightStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = load_state(&path).await?;

        Ok(Self {
            path: Arc::new(path),
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub(crate) async fn pin_thread(
        &self,
        item: HighlightedThreadRecord,
    ) -> Result<HighlightedThreadRecord, HighlightStoreError> {
        let mut state = self.state.lock().await;
        state.insert(item.thread_id.clone(), item.clone());
        persist_state(&self.path, &state).await?;
        Ok(item)
    }

    pub(crate) async fn list_threads(&self) -> Vec<HighlightedThreadRecord> {
        let state = self.state.lock().await;
        sort_records(state.values().cloned().collect())
    }

    pub(crate) async fn unpin_thread(&self, thread_id: &str) -> Result<bool, HighlightStoreError> {
        let mut state = self.state.lock().await;
        let removed = state.remove(thread_id).is_some();
        if removed {
            persist_state(&self.path, &state).await?;
        }
        Ok(removed)
    }
}

#[cfg_attr(test, allow(dead_code))]
impl PostgresHighlightStore {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg_attr(test, allow(dead_code))]
impl HighlightStore {
    pub(crate) async fn pin_thread(
        &self,
        item: HighlightedThreadRecord,
    ) -> Result<HighlightedThreadRecord, HighlightStoreError> {
        match self {
            Self::Local(store) => store.pin_thread(item).await,
            Self::Postgres(store) => {
                sqlx::query(
                    r#"
                    INSERT INTO highlighted_threads (
                        channel_id,
                        root_ts,
                        pinned_by_user_id,
                        pinned_at
                    )
                    VALUES ($1, $2, $3, to_timestamp($4))
                    ON CONFLICT (channel_id, root_ts) DO UPDATE
                    SET pinned_by_user_id = EXCLUDED.pinned_by_user_id,
                        pinned_at = EXCLUDED.pinned_at
                    "#,
                )
                .bind(&item.channel_id)
                .bind(&item.root_ts)
                .bind(&item.pinned_by_user_id)
                .bind(item.pinned_at.parse::<f64>().unwrap_or_default())
                .execute(&store.pool)
                .await
                .map_err(HighlightStoreError::Sqlx)?;
                Ok(item)
            }
        }
    }

    pub(crate) async fn list_threads(&self) -> Vec<HighlightedThreadRecord> {
        match self {
            Self::Local(store) => store.list_threads().await,
            Self::Postgres(store) => sqlx::query(
                r#"
                SELECT
                    channel_id,
                    root_ts,
                    pinned_by_user_id,
                    CAST(EXTRACT(EPOCH FROM pinned_at) AS bigint)::text AS pinned_at
                FROM highlighted_threads
                ORDER BY pinned_at DESC,
                         channel_id ASC,
                         root_ts ASC
                "#,
            )
            .fetch_all(&store.pool)
            .await
            .map(|rows| {
                sort_records(
                    rows.into_iter()
                        .map(|row| {
                            let channel_id: String = row.get("channel_id");
                            let root_ts: String = row.get("root_ts");
                            HighlightedThreadRecord {
                                thread_id: format!("{channel_id}:{root_ts}"),
                                channel_id,
                                root_ts,
                                pinned_by_user_id: row.get("pinned_by_user_id"),
                                pinned_at: row.get("pinned_at"),
                            }
                        })
                        .collect(),
                )
            })
            .unwrap_or_default(),
        }
    }

    pub(crate) async fn unpin_thread(&self, thread_id: &str) -> Result<bool, HighlightStoreError> {
        match self {
            Self::Local(store) => store.unpin_thread(thread_id).await,
            Self::Postgres(store) => {
                let Some((channel_id, root_ts)) = thread_id.split_once(':') else {
                    return Ok(false);
                };

                Ok(sqlx::query(
                    r#"
                    DELETE FROM highlighted_threads
                    WHERE channel_id = $1
                      AND root_ts = $2
                    "#,
                )
                .bind(channel_id)
                .bind(root_ts)
                .execute(&store.pool)
                .await
                .map_err(HighlightStoreError::Sqlx)?
                .rows_affected()
                    > 0)
            }
        }
    }
}

impl From<LocalHighlightStore> for HighlightStore {
    fn from(value: LocalHighlightStore) -> Self {
        Self::Local(value)
    }
}

impl From<PostgresHighlightStore> for HighlightStore {
    fn from(value: PostgresHighlightStore) -> Self {
        Self::Postgres(value)
    }
}

async fn load_state(path: &Path) -> Result<HighlightState, HighlightStoreError> {
    if !fs::try_exists(path)
        .await
        .map_err(HighlightStoreError::Read)?
    {
        return Ok(HashMap::new());
    }

    let contents = fs::read(path).await.map_err(HighlightStoreError::Read)?;
    if contents.is_empty() {
        return Ok(HashMap::new());
    }

    let items: Vec<HighlightedThreadRecord> = serde_json::from_slice(&contents)?;
    Ok(items
        .into_iter()
        .map(|item| (item.thread_id.clone(), item))
        .collect())
}

async fn persist_state(path: &Path, state: &HighlightState) -> Result<(), HighlightStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(HighlightStoreError::CreateDirectory)?;
    }

    let payload = serde_json::to_vec_pretty(&sort_records(state.values().cloned().collect()))?;
    fs::write(path, payload)
        .await
        .map_err(HighlightStoreError::Write)
}

fn sort_records(mut items: Vec<HighlightedThreadRecord>) -> Vec<HighlightedThreadRecord> {
    items.sort_by(|left, right| {
        (&right.pinned_at, &left.thread_id).cmp(&(&left.pinned_at, &right.thread_id))
    });
    items
}

#[cfg(test)]
#[path = "highlight_store_tests.rs"]
mod tests;
