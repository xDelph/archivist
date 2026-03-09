use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{fs, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SavedItemRecord {
    pub(crate) team_id: String,
    pub(crate) slack_user_id: String,
    pub(crate) thread_id: String,
    pub(crate) channel_id: String,
    pub(crate) root_ts: String,
    pub(crate) title: String,
    pub(crate) preview: String,
    pub(crate) last_activity_ts: String,
    pub(crate) saved_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalSavedItemStore {
    path: Arc<PathBuf>,
    state: SharedSavedItemState,
}

type SavedItemKey = (String, String, String);
type SavedItemMap = HashMap<SavedItemKey, SavedItemRecord>;
type SharedSavedItemState = Arc<Mutex<SavedItemMap>>;

#[derive(Debug, Error)]
pub(crate) enum SavedItemStoreError {
    #[error("failed to read saved items")]
    Read(#[source] std::io::Error),
    #[error("failed to parse saved items")]
    Parse(#[from] serde_json::Error),
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
            (
                item.team_id.clone(),
                item.slack_user_id.clone(),
                item.thread_id.clone(),
            ),
            item.clone(),
        );
        persist_state(&self.path, &state).await?;
        Ok(item)
    }

    pub(crate) async fn list_items(
        &self,
        team_id: &str,
        slack_user_id: &str,
    ) -> Vec<SavedItemRecord> {
        let state = self.state.lock().await;
        let mut items = state
            .values()
            .filter(|item| item.team_id == team_id && item.slack_user_id == slack_user_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            (&right.saved_at, &left.thread_id).cmp(&(&left.saved_at, &right.thread_id))
        });
        items
    }

    pub(crate) async fn remove_item(
        &self,
        team_id: &str,
        slack_user_id: &str,
        thread_id: &str,
    ) -> Result<bool, SavedItemStoreError> {
        let mut state = self.state.lock().await;
        let removed = state
            .remove(&(
                team_id.to_owned(),
                slack_user_id.to_owned(),
                thread_id.to_owned(),
            ))
            .is_some();
        if removed {
            persist_state(&self.path, &state).await?;
        }
        Ok(removed)
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
        .map(|item| {
            (
                (
                    item.team_id.clone(),
                    item.slack_user_id.clone(),
                    item.thread_id.clone(),
                ),
                item,
            )
        })
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
        (&left.team_id, &left.slack_user_id, &left.thread_id).cmp(&(
            &right.team_id,
            &right.slack_user_id,
            &right.thread_id,
        ))
    });
    let payload = serde_json::to_vec_pretty(&items)?;
    fs::write(path, payload)
        .await
        .map_err(SavedItemStoreError::Write)
}

#[cfg(test)]
mod tests {
    use super::{LocalSavedItemStore, SavedItemRecord};
    use tempfile::tempdir;

    #[tokio::test]
    async fn local_saved_item_store_upserts_lists_and_removes_items() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("saved-items.json");
        let store = LocalSavedItemStore::open(&path).await.expect("store");

        store
            .upsert_item(SavedItemRecord {
                team_id: "T123".to_owned(),
                slack_user_id: "U123".to_owned(),
                thread_id: "C123:1700000000.000001".to_owned(),
                channel_id: "C123".to_owned(),
                root_ts: "1700000000.000001".to_owned(),
                title: "Launch update".to_owned(),
                preview: "We should ship it".to_owned(),
                last_activity_ts: "1700000000.000010".to_owned(),
                saved_at: "2026-03-09T10:00:00Z".to_owned(),
            })
            .await
            .expect("first save");
        store
            .upsert_item(SavedItemRecord {
                team_id: "T123".to_owned(),
                slack_user_id: "U123".to_owned(),
                thread_id: "C456:1700000000.000002".to_owned(),
                channel_id: "C456".to_owned(),
                root_ts: "1700000000.000002".to_owned(),
                title: "Design sync".to_owned(),
                preview: "New comments landed".to_owned(),
                last_activity_ts: "1700000000.000020".to_owned(),
                saved_at: "2026-03-09T11:00:00Z".to_owned(),
            })
            .await
            .expect("second save");

        let reopened = LocalSavedItemStore::open(&path).await.expect("reopened");
        let items = reopened.list_items("T123", "U123").await;

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].thread_id, "C456:1700000000.000002");

        assert!(
            reopened
                .remove_item("T123", "U123", "C456:1700000000.000002")
                .await
                .expect("remove")
        );
        assert_eq!(reopened.list_items("T123", "U123").await.len(), 1);
    }
}
