use crate::{
    PgEventStore, SearchDocumentRow, ThreadSummaryRow,
    search_index::{MessageMap, SearchDocumentMap, refresh_search_documents},
    thread_summary_index::{ThreadSummaryMap, build_thread_summaries},
};
use domain::{Channel, EventPayload, File, Message, ProcessEventJob, Reaction};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{fs, io::AsyncWriteExt, sync::Mutex};

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

    pub async fn refresh_thread_summaries(&self) -> Result<usize, StoreError> {
        match self {
            Self::Local(store) => Ok(store.refresh_thread_summaries().await),
            Self::Postgres(store) => store.refresh_thread_summaries().await,
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

#[derive(Debug, Clone)]
pub struct JsonlEventStore {
    path: Arc<PathBuf>,
    state: Arc<Mutex<StoreState>>,
}

type FileKey = (String, String);
type ReactionKey = (String, String, String, String, String);

#[derive(Debug, Default)]
struct StoreState {
    seen_events: HashSet<String>,
    messages: MessageMap,
    files: HashMap<FileKey, File>,
    channels: HashMap<(String, String), Channel>,
    reactions: HashSet<ReactionKey>,
    search_documents: SearchDocumentMap,
    thread_summaries: ThreadSummaryMap,
}

impl JsonlEventStore {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        let state = load_state(&path).await?;

        Ok(Self {
            path: Arc::new(path),
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub async fn health(&self) -> RepositoryHealth {
        let state = self.state.lock().await;

        RepositoryHealth {
            tracked_events: state.seen_events.len(),
            tracked_messages: state.messages.len(),
            tracked_reactions: state.reactions.len(),
            tracked_files: state.files.len(),
            tracked_channels: state.channels.len(),
        }
    }

    pub async fn record_process_event(
        &self,
        job: &ProcessEventJob,
    ) -> Result<StoreOutcome, StoreError> {
        let mut state = self.state.lock().await;
        if state.seen_events.contains(&job.event_id) {
            return Ok(StoreOutcome::Duplicate);
        }

        append_job(&self.path, job).await?;
        state.seen_events.insert(job.event_id.clone());
        state.apply_job(job);
        state.thread_summaries =
            build_thread_summaries(&state.messages, &state.reactions, &state.files);

        Ok(StoreOutcome::Inserted)
    }

    pub async fn messages(&self) -> Vec<Message> {
        let state = self.state.lock().await;
        let mut messages = state.messages.values().cloned().collect::<Vec<_>>();
        messages.sort_by(|left, right| {
            (&left.channel_id, &left.ts).cmp(&(&right.channel_id, &right.ts))
        });
        messages
    }

    pub async fn reactions(&self) -> Vec<Reaction> {
        let state = self.state.lock().await;
        let mut reactions = state
            .reactions
            .iter()
            .map(
                |(team_id, channel_id, message_ts, user_id, reaction_name)| Reaction {
                    team_id: team_id.clone(),
                    channel_id: channel_id.clone(),
                    message_ts: message_ts.clone(),
                    user_id: user_id.clone(),
                    name: reaction_name.clone(),
                },
            )
            .collect::<Vec<_>>();
        reactions.sort_by(|left, right| {
            (
                &left.channel_id,
                &left.message_ts,
                &left.user_id,
                &left.name,
            )
                .cmp(&(
                    &right.channel_id,
                    &right.message_ts,
                    &right.user_id,
                    &right.name,
                ))
        });
        reactions
    }

    pub async fn files(&self) -> Vec<File> {
        let state = self.state.lock().await;
        let mut files = state.files.values().cloned().collect::<Vec<_>>();
        files.sort_by(|left, right| {
            (&left.channel_id, &left.message_ts, &left.id).cmp(&(
                &right.channel_id,
                &right.message_ts,
                &right.id,
            ))
        });
        files
    }

    pub async fn channels(&self) -> Vec<Channel> {
        let state = self.state.lock().await;
        let mut channels = state.channels.values().cloned().collect::<Vec<_>>();
        channels.sort_by(|left, right| (&left.team_id, &left.id).cmp(&(&right.team_id, &right.id)));
        channels
    }

    pub async fn search_documents(&self) -> Vec<SearchDocumentRow> {
        let state = self.state.lock().await;
        let mut search_documents = state.search_documents.values().cloned().collect::<Vec<_>>();
        search_documents.sort_by(|left, right| {
            (&left.channel_id, &left.message_ts).cmp(&(&right.channel_id, &right.message_ts))
        });
        search_documents
    }

    pub async fn thread_summaries(&self) -> Vec<ThreadSummaryRow> {
        let state = self.state.lock().await;
        let mut thread_summaries = state.thread_summaries.values().cloned().collect::<Vec<_>>();
        thread_summaries.sort_by(|left, right| {
            (&left.channel_id, &left.root_ts).cmp(&(&right.channel_id, &right.root_ts))
        });
        thread_summaries
    }

    pub async fn refresh_thread_summaries(&self) -> usize {
        let mut state = self.state.lock().await;
        state.thread_summaries =
            build_thread_summaries(&state.messages, &state.reactions, &state.files);
        state.thread_summaries.len()
    }
}

impl StoreState {
    fn apply_job(&mut self, job: &ProcessEventJob) {
        match &job.payload {
            EventPayload::Message {
                user_id,
                text,
                ts,
                thread_ts,
                files,
            } => {
                self.messages.insert(
                    (job.team_id.clone(), job.channel_id.clone(), ts.clone()),
                    Message {
                        team_id: job.team_id.clone(),
                        channel_id: job.channel_id.clone(),
                        ts: ts.clone(),
                        thread_ts: thread_ts.clone(),
                        user_id: user_id.clone(),
                        text: text.clone().unwrap_or_default(),
                    },
                );
                for file in files {
                    self.files.insert(
                        (job.team_id.clone(), file.id.clone()),
                        File {
                            id: file.id.clone(),
                            team_id: job.team_id.clone(),
                            channel_id: job.channel_id.clone(),
                            message_ts: ts.clone(),
                            name: file.name.clone(),
                            mimetype: file.mimetype.clone(),
                            permalink: file.permalink.clone(),
                            size: file.size,
                        },
                    );
                }
                refresh_search_documents(
                    &mut self.search_documents,
                    &self.messages,
                    &job.team_id,
                    &job.channel_id,
                    thread_ts.as_deref().unwrap_or(ts),
                );
            }
            EventPayload::ReactionAdded {
                user_id,
                reaction,
                item_ts,
            } => {
                self.reactions.insert((
                    job.team_id.clone(),
                    job.channel_id.clone(),
                    item_ts.clone(),
                    user_id.clone(),
                    reaction.clone(),
                ));
            }
            EventPayload::ChannelUpdated { name, is_archived } => {
                let key = (job.team_id.clone(), job.channel_id.clone());
                let existing = self.channels.remove(&key);
                let mut channel = existing.unwrap_or(Channel {
                    team_id: job.team_id.clone(),
                    id: job.channel_id.clone(),
                    kind: job.channel_kind,
                    name: None,
                    is_archived: false,
                });

                if let Some(name) = name {
                    channel.name = Some(name.clone());
                }
                if let Some(is_archived) = is_archived {
                    channel.is_archived = *is_archived;
                }

                self.channels.insert(key, channel);
            }
        }
    }
}

async fn load_state(path: &Path) -> Result<StoreState, StoreError> {
    if !fs::try_exists(path).await.map_err(StoreError::Read)? {
        return Ok(StoreState::default());
    }

    let contents = fs::read_to_string(path).await.map_err(StoreError::Read)?;
    let mut state = StoreState::default();

    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let job: ProcessEventJob = serde_json::from_str(line)?;
        state.seen_events.insert(job.event_id.clone());
        state.apply_job(&job);
    }
    state.thread_summaries =
        build_thread_summaries(&state.messages, &state.reactions, &state.files);

    Ok(state)
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

#[cfg(not(test))]
fn runtime_database_url() -> Result<String, StoreError> {
    std::env::var("POSTGRES_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or(StoreError::MissingDatabaseUrl)
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
