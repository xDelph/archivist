#[cfg(not(test))]
use crate::local_store::runtime_database_url;
use crate::{
    BackfillBatchStats, GeneratedThreadSummaryRow, JsonlEventStore, PgEventStore,
    SearchDocumentRow, ThreadCardRow, ThreadSummaryRow,
};
use domain::{Channel, File, Message, ProcessEventJob, Reaction};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryMode {
    TestJsonl,
    Postgres,
}

impl RepositoryMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TestJsonl => "test_jsonl",
            Self::Postgres => "postgres",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryHealth {
    pub tracked_events: usize,
    pub tracked_messages: usize,
    pub tracked_reactions: usize,
    pub tracked_files: usize,
    pub tracked_channels: usize,
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
    #[error("database operation failed")]
    Sqlx(#[source] sqlx::Error),
    #[error("missing postgres database url")]
    MissingDatabaseUrl,
    #[error("invalid runtime configuration: {0}")]
    InvalidRuntimeConfig(&'static str),
    #[error("failed to create event log directory")]
    CreateDirectory(#[source] std::io::Error),
    #[error("failed to append to event log")]
    Append(#[source] std::io::Error),
}

#[derive(Debug, Clone)]
pub enum EventStore {
    Local(JsonlEventStore),
    Postgres(PgEventStore),
}

impl EventStore {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        #[cfg(test)]
        {
            JsonlEventStore::open(path).await.map(Self::Local)
        }

        #[cfg(not(test))]
        {
            let _ = path.as_ref();
            PgEventStore::open(&runtime_database_url()?)
                .await
                .map(Self::Postgres)
        }
    }

    pub const fn mode(&self) -> RepositoryMode {
        match self {
            Self::Local(_) => RepositoryMode::TestJsonl,
            Self::Postgres(_) => RepositoryMode::Postgres,
        }
    }

    pub fn postgres_pool(&self) -> Option<sqlx::PgPool> {
        match self {
            Self::Local(_) => None,
            Self::Postgres(store) => Some(store.pool()),
        }
    }

    pub async fn health(&self) -> Result<RepositoryHealth, StoreError> {
        match self {
            Self::Local(store) => Ok(store.health().await),
            Self::Postgres(store) => store.health().await,
        }
    }

    pub async fn record_process_event(
        &self,
        job: &ProcessEventJob,
    ) -> Result<StoreOutcome, StoreError> {
        match self {
            Self::Local(store) => store.record_process_event(job).await,
            Self::Postgres(store) => store.record_process_event(job).await,
        }
    }

    pub async fn messages(&self) -> Result<Vec<Message>, StoreError> {
        match self {
            Self::Local(store) => Ok(store.messages().await),
            Self::Postgres(store) => store.messages().await,
        }
    }

    pub async fn latest_message_ts(&self, channel_id: &str) -> Result<Option<String>, StoreError> {
        match self {
            Self::Local(store) => Ok(store.latest_message_ts(channel_id).await),
            Self::Postgres(store) => store.latest_message_ts(channel_id).await,
        }
    }

    pub async fn reactions(&self) -> Result<Vec<Reaction>, StoreError> {
        match self {
            Self::Local(store) => Ok(store.reactions().await),
            Self::Postgres(store) => store.reactions().await,
        }
    }

    pub async fn files(&self) -> Result<Vec<File>, StoreError> {
        match self {
            Self::Local(store) => Ok(store.files().await),
            Self::Postgres(store) => store.files().await,
        }
    }

    pub async fn channels(&self) -> Result<Vec<Channel>, StoreError> {
        match self {
            Self::Local(store) => Ok(store.channels().await),
            Self::Postgres(store) => store.channels().await,
        }
    }

    pub async fn search_documents(&self) -> Result<Vec<SearchDocumentRow>, StoreError> {
        match self {
            Self::Local(store) => Ok(store.search_documents().await),
            Self::Postgres(store) => store.search_documents().await,
        }
    }

    pub async fn thread_summaries(&self) -> Result<Vec<ThreadSummaryRow>, StoreError> {
        match self {
            Self::Local(store) => Ok(store.thread_summaries().await),
            Self::Postgres(store) => store.thread_summaries().await,
        }
    }

    pub async fn thread_cards(&self) -> Result<Vec<ThreadCardRow>, StoreError> {
        match self {
            Self::Local(store) => Ok(store.thread_cards().await),
            Self::Postgres(store) => store.thread_cards().await,
        }
    }

    pub async fn generated_thread_summaries(
        &self,
    ) -> Result<Vec<GeneratedThreadSummaryRow>, StoreError> {
        match self {
            Self::Local(store) => Ok(store.generated_thread_summaries().await),
            Self::Postgres(store) => store.generated_thread_summaries().await,
        }
    }

    pub async fn refresh_thread_summaries(&self) -> Result<usize, StoreError> {
        match self {
            Self::Local(store) => Ok(store.refresh_thread_summaries().await),
            Self::Postgres(store) => store.refresh_thread_summaries().await,
        }
    }

    pub async fn upsert_user_profile(
        &self,
        user_id: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
        is_active: bool,
    ) -> Result<(), StoreError> {
        match self {
            Self::Local(store) => {
                store
                    .upsert_user_profile(user_id, email, display_name, avatar_url, is_active)
                    .await;
                Ok(())
            }
            Self::Postgres(store) => {
                store
                    .upsert_user_profile(user_id, email, display_name, avatar_url, is_active)
                    .await
            }
        }
    }

    pub async fn set_file_archive(
        &self,
        file_id: &str,
        storage_key: &str,
        storage_url: &str,
    ) -> Result<(), StoreError> {
        match self {
            Self::Local(store) => {
                store
                    .set_file_archive(file_id, storage_key, storage_url)
                    .await;
                Ok(())
            }
            Self::Postgres(store) => {
                store
                    .set_file_archive(file_id, storage_key, storage_url)
                    .await
            }
        }
    }

    pub async fn upsert_generated_thread_summary(
        &self,
        row: &GeneratedThreadSummaryRow,
    ) -> Result<(), StoreError> {
        match self {
            Self::Local(store) => store.upsert_generated_thread_summary(row).await,
            Self::Postgres(store) => store.upsert_generated_thread_summary(row).await,
        }
    }

    pub async fn backfill_channel_jobs(
        &self,
        channel_job: Option<&ProcessEventJob>,
        message_jobs: &[ProcessEventJob],
        reaction_jobs: &[ProcessEventJob],
    ) -> Result<BackfillBatchStats, StoreError> {
        match self {
            Self::Local(store) => {
                store
                    .backfill_channel_jobs(channel_job, message_jobs, reaction_jobs)
                    .await
            }
            Self::Postgres(store) => {
                store
                    .backfill_channel_jobs(channel_job, message_jobs, reaction_jobs)
                    .await
            }
        }
    }
}

impl From<JsonlEventStore> for EventStore {
    fn from(value: JsonlEventStore) -> Self {
        Self::Local(value)
    }
}

impl From<PgEventStore> for EventStore {
    fn from(value: PgEventStore) -> Self {
        Self::Postgres(value)
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
