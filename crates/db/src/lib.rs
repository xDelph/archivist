use domain::ProcessEventJob;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{fs, io::AsyncWriteExt, sync::Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryMode {
    LocalJsonlMock,
}

impl RepositoryMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalJsonlMock => "local_jsonl_mock",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryHealth {
    pub tracked_events: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOutcome {
    Inserted,
    Duplicate,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("failed to read event log")]
    Read(#[source] std::io::Error),
    #[error("failed to parse event log")]
    Parse(#[from] serde_json::Error),
    #[error("failed to create event log directory")]
    CreateDirectory(#[source] std::io::Error),
    #[error("failed to append to event log")]
    Append(#[source] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct JsonlEventStore {
    path: Arc<PathBuf>,
    seen_events: Arc<Mutex<HashSet<String>>>,
}

impl JsonlEventStore {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        let seen_events = load_known_events(&path).await?;

        Ok(Self {
            path: Arc::new(path),
            seen_events: Arc::new(Mutex::new(seen_events)),
        })
    }

    pub async fn health(&self) -> RepositoryHealth {
        RepositoryHealth {
            tracked_events: self.seen_events.lock().await.len(),
        }
    }

    pub async fn record_process_event(
        &self,
        job: &ProcessEventJob,
    ) -> Result<StoreOutcome, StoreError> {
        let mut seen_events = self.seen_events.lock().await;
        if !seen_events.insert(job.event_id.clone()) {
            return Ok(StoreOutcome::Duplicate);
        }

        drop(seen_events);
        append_job(&self.path, job).await?;

        Ok(StoreOutcome::Inserted)
    }
}

async fn load_known_events(path: &Path) -> Result<HashSet<String>, StoreError> {
    if !fs::try_exists(path).await.map_err(StoreError::Read)? {
        return Ok(HashSet::new());
    }

    let contents = fs::read_to_string(path).await.map_err(StoreError::Read)?;
    let mut seen_events = HashSet::new();

    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let job: ProcessEventJob = serde_json::from_str(line)?;
        seen_events.insert(job.event_id);
    }

    Ok(seen_events)
}

async fn append_job(path: &Path, job: &ProcessEventJob) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(StoreError::CreateDirectory)?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .map_err(StoreError::Append)?;
    let mut payload = serde_json::to_vec(job)?;
    payload.push(b'\n');

    file.write_all(&payload).await.map_err(StoreError::Append)
}

#[cfg(test)]
mod tests {
    use super::{JsonlEventStore, StoreOutcome};
    use domain::{ChannelKind, EventPayload, ProcessEventJob};
    use tempfile::tempdir;

    fn sample_job(event_id: &str) -> ProcessEventJob {
        ProcessEventJob {
            event_id: event_id.to_owned(),
            team_id: "team_1".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("hello".to_owned()),
                ts: "1700000000.000001".to_owned(),
                thread_ts: None,
            },
        }
    }

    #[tokio::test]
    async fn store_deduplicates_and_persists_ids() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&path).await.expect("store");

        assert_eq!(
            store
                .record_process_event(&sample_job("evt_1"))
                .await
                .expect("insert"),
            StoreOutcome::Inserted
        );
        assert_eq!(
            store
                .record_process_event(&sample_job("evt_1"))
                .await
                .expect("duplicate"),
            StoreOutcome::Duplicate
        );

        let reopened = JsonlEventStore::open(&path).await.expect("reopened");
        assert_eq!(reopened.health().await.tracked_events, 1);
    }
}
