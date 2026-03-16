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
pub(crate) struct SavedItemRecord {
    pub(crate) slack_user_id: String,
    pub(crate) thread_id: String,
    pub(crate) channel_id: String,
    pub(crate) root_ts: String,
    pub(crate) last_activity_ts: String,
    pub(crate) saved_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalSavedItemStore {
    path: Arc<PathBuf>,
    state: SharedSavedItemState,
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresSavedItemStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub(crate) enum SavedItemStore {
    Local(LocalSavedItemStore),
    Postgres(PostgresSavedItemStore),
}

type SavedItemKey = (String, String);
type SavedItemMap = HashMap<SavedItemKey, SavedItemRecord>;
type SharedSavedItemState = Arc<Mutex<SavedItemMap>>;

#[derive(Debug, Error)]
pub(crate) enum SavedItemStoreError {
    #[error("failed to read saved items")]
    Read(#[source] std::io::Error),
    #[error("failed to parse saved items")]
    Parse(#[from] serde_json::Error),
    #[error("failed to query saved items")]
    Sqlx(#[source] sqlx::Error),
    #[error("failed to create saved item directory")]
    CreateDirectory(#[source] std::io::Error),
    #[error("failed to write saved items")]
    Write(#[source] std::io::Error),
}

impl LocalSavedItemStore {
    pub(crate) async fn open(path: impl AsRef<Path>) -> Result<Self, SavedItemStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = load_state(&path).await?;

        Ok(Self {
            path: Arc::new(path),
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub(crate) async fn upsert_item(
        &self,
        item: SavedItemRecord,
    ) -> Result<SavedItemRecord, SavedItemStoreError> {
        let mut state = self.state.lock().await;
        state.insert(
            (item.slack_user_id.clone(), item.thread_id.clone()),
            item.clone(),
        );
        persist_state(&self.path, &state).await?;
        Ok(item)
    }

    pub(crate) async fn list_items(&self, slack_user_id: &str) -> Vec<SavedItemRecord> {
        let state = self.state.lock().await;
        let mut items = state
            .values()
            .filter(|item| item.slack_user_id == slack_user_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            (&right.saved_at, &left.thread_id).cmp(&(&left.saved_at, &right.thread_id))
        });
        items
    }

    pub(crate) async fn remove_item(
        &self,
        slack_user_id: &str,
        thread_id: &str,
    ) -> Result<bool, SavedItemStoreError> {
        let mut state = self.state.lock().await;
        let removed = state
            .remove(&(slack_user_id.to_owned(), thread_id.to_owned()))
            .is_some();
        if removed {
            persist_state(&self.path, &state).await?;
        }
        Ok(removed)
    }
}

#[cfg_attr(test, allow(dead_code))]
impl PostgresSavedItemStore {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl SavedItemStore {
    pub(crate) async fn upsert_item(
        &self,
        item: SavedItemRecord,
    ) -> Result<SavedItemRecord, SavedItemStoreError> {
        match self {
            Self::Local(store) => store.upsert_item(item).await,
            Self::Postgres(store) => {
                sqlx::query(
                    r#"
                    INSERT INTO saved_items (user_id, channel_id, root_ts, saved_at)
                    VALUES ($1, $2, $3, to_timestamp($4))
                    ON CONFLICT (user_id, channel_id, root_ts) DO UPDATE
                    SET saved_at = EXCLUDED.saved_at
                    "#,
                )
                .bind(&item.slack_user_id)
                .bind(&item.channel_id)
                .bind(&item.root_ts)
                .bind(item.saved_at.parse::<f64>().unwrap_or_default())
                .execute(&store.pool)
                .await
                .map_err(SavedItemStoreError::Sqlx)?;
                Ok(item)
            }
        }
    }

    pub(crate) async fn list_items(&self, slack_user_id: &str) -> Vec<SavedItemRecord> {
        match self {
            Self::Local(store) => store.list_items(slack_user_id).await,
            Self::Postgres(store) => sqlx::query(
                r#"
                SELECT
                    saved_items.user_id,
                    saved_items.channel_id,
                    saved_items.root_ts,
                    to_char(
                        EXTRACT(EPOCH FROM thread_summaries.last_activity_at),
                        'FM999999999999999.000000'
                    ) AS last_activity_ts,
                    CAST(EXTRACT(EPOCH FROM saved_items.saved_at) AS bigint)::text
                        AS saved_at
                FROM saved_items
                JOIN thread_summaries
                    ON thread_summaries.channel_id = saved_items.channel_id
                   AND thread_summaries.root_ts = saved_items.root_ts
                WHERE saved_items.user_id = $1
                ORDER BY saved_items.saved_at DESC,
                         saved_items.channel_id ASC,
                         saved_items.root_ts ASC
                "#,
            )
            .bind(slack_user_id)
            .fetch_all(&store.pool)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| {
                        let channel_id: String = row.get("channel_id");
                        let root_ts: String = row.get("root_ts");
                        SavedItemRecord {
                            slack_user_id: row.get("user_id"),
                            thread_id: format!("{channel_id}:{root_ts}"),
                            channel_id,
                            root_ts,
                            last_activity_ts: row.get("last_activity_ts"),
                            saved_at: row.get("saved_at"),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
        }
    }

    pub(crate) async fn remove_item(
        &self,
        slack_user_id: &str,
        thread_id: &str,
    ) -> Result<bool, SavedItemStoreError> {
        match self {
            Self::Local(store) => store.remove_item(slack_user_id, thread_id).await,
            Self::Postgres(store) => {
                let Some((channel_id, root_ts)) = thread_id.split_once(':') else {
                    return Ok(false);
                };
                Ok(sqlx::query(
                    r#"
                    DELETE FROM saved_items
                    WHERE user_id = $1
                      AND channel_id = $2
                      AND root_ts = $3
                    "#,
                )
                .bind(slack_user_id)
                .bind(channel_id)
                .bind(root_ts)
                .execute(&store.pool)
                .await
                .map_err(SavedItemStoreError::Sqlx)?
                .rows_affected()
                    > 0)
            }
        }
    }
}

impl From<LocalSavedItemStore> for SavedItemStore {
    fn from(value: LocalSavedItemStore) -> Self {
        Self::Local(value)
    }
}

impl From<PostgresSavedItemStore> for SavedItemStore {
    fn from(value: PostgresSavedItemStore) -> Self {
        Self::Postgres(value)
    }
}

async fn load_state(path: &Path) -> Result<SavedItemMap, SavedItemStoreError> {
    if !fs::try_exists(path)
        .await
        .map_err(SavedItemStoreError::Read)?
    {
        return Ok(HashMap::new());
    }

    let contents = fs::read(path).await.map_err(SavedItemStoreError::Read)?;
    if contents.is_empty() {
        return Ok(HashMap::new());
    }

    let items: Vec<SavedItemRecord> = serde_json::from_slice(&contents)?;
    Ok(items
        .into_iter()
        .map(|item| ((item.slack_user_id.clone(), item.thread_id.clone()), item))
        .collect())
}

async fn persist_state(path: &Path, state: &SavedItemMap) -> Result<(), SavedItemStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(SavedItemStoreError::CreateDirectory)?;
    }

    let mut items = state.values().cloned().collect::<Vec<_>>();
    items.sort_by(|left, right| {
        (&left.slack_user_id, &left.thread_id).cmp(&(&right.slack_user_id, &right.thread_id))
    });
    let payload = serde_json::to_vec_pretty(&items)?;
    fs::write(path, payload)
        .await
        .map_err(SavedItemStoreError::Write)
}

#[cfg(test)]
#[path = "saved_store_tests.rs"]
mod tests;
