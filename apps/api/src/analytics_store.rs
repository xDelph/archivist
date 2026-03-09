use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{fs, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AnalyticsEventRecord {
    pub(crate) event_type: String,
    pub(crate) user_id: Option<String>,
    pub(crate) metadata: serde_json::Value,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalAnalyticsStore {
    path: Arc<PathBuf>,
    state: Arc<Mutex<Vec<AnalyticsEventRecord>>>,
}

#[derive(Debug, Error)]
pub(crate) enum AnalyticsStoreError {
    #[error("failed to read analytics events")]
    Read(#[source] std::io::Error),
    #[error("failed to parse analytics events")]
    Parse(#[from] serde_json::Error),
    #[error("failed to create analytics directory")]
    CreateDirectory(#[source] std::io::Error),
    #[error("failed to write analytics events")]
    Write(#[source] std::io::Error),
}

impl LocalAnalyticsStore {
    pub(crate) async fn open(path: impl AsRef<Path>) -> Result<Self, AnalyticsStoreError> {
        let path = path.as_ref().to_path_buf();
        let state = load_state(&path).await?;

        Ok(Self {
            path: Arc::new(path),
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub(crate) async fn record_event(
        &self,
        event: AnalyticsEventRecord,
    ) -> Result<(), AnalyticsStoreError> {
        let mut state = self.state.lock().await;
        state.push(event);
        persist_state(&self.path, &state).await
    }

    pub(crate) async fn list_events(
        &self,
        event_type: Option<&str>,
        limit: usize,
    ) -> Vec<AnalyticsEventRecord> {
        let state = self.state.lock().await;
        let mut results: Vec<AnalyticsEventRecord> = match event_type {
            Some(kind) => state
                .iter()
                .filter(|e| e.event_type == kind)
                .cloned()
                .collect(),
            None => state.clone(),
        };
        results.reverse();
        results.truncate(limit);
        results
    }

    pub(crate) async fn count_by_type(&self) -> Vec<(String, usize)> {
        let state = self.state.lock().await;
        let mut counts = std::collections::HashMap::<String, usize>::new();
        for event in state.iter() {
            *counts.entry(event.event_type.clone()).or_default() += 1;
        }
        let mut result: Vec<_> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }
}

async fn load_state(path: &Path) -> Result<Vec<AnalyticsEventRecord>, AnalyticsStoreError> {
    if !fs::try_exists(path)
        .await
        .map_err(AnalyticsStoreError::Read)?
    {
        return Ok(Vec::new());
    }

    let contents = fs::read(path).await.map_err(AnalyticsStoreError::Read)?;
    if contents.is_empty() {
        return Ok(Vec::new());
    }

    let events: Vec<AnalyticsEventRecord> = serde_json::from_slice(&contents)?;
    Ok(events)
}

async fn persist_state(
    path: &Path,
    state: &[AnalyticsEventRecord],
) -> Result<(), AnalyticsStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(AnalyticsStoreError::CreateDirectory)?;
    }

    let payload = serde_json::to_vec_pretty(state)?;
    fs::write(path, payload)
        .await
        .map_err(AnalyticsStoreError::Write)
}

#[cfg(test)]
#[path = "analytics_store_tests.rs"]
mod tests;
