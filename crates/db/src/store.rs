use domain::{EventPayload, File, Message, ProcessEventJob, Reaction};
use std::{
    collections::{HashMap, HashSet},
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
    pub tracked_messages: usize,
    pub tracked_reactions: usize,
    pub tracked_files: usize,
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
    state: Arc<Mutex<StoreState>>,
}

type MessageKey = (String, String);
type FileKey = (String, String);
type ReactionKey = (String, String, String, String, String);

#[derive(Debug, Default)]
struct StoreState {
    seen_events: HashSet<String>,
    messages: HashMap<MessageKey, Message>,
    files: HashMap<FileKey, File>,
    reactions: HashSet<ReactionKey>,
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
                    (job.channel_id.clone(), ts.clone()),
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

#[cfg(test)]
mod tests {
    use super::{JsonlEventStore, StoreOutcome};
    use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};
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
                files: vec![],
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
        let health = reopened.health().await;

        assert_eq!(health.tracked_events, 1);
        assert_eq!(health.tracked_messages, 1);
        assert_eq!(health.tracked_reactions, 0);
        assert_eq!(health.tracked_files, 0);
    }

    #[tokio::test]
    async fn message_jobs_upsert_by_channel_and_timestamp() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&path).await.expect("store");
        let original = sample_job("evt_1");
        let EventPayload::Message {
            user_id,
            ts,
            thread_ts,
            ..
        } = original.payload.clone()
        else {
            unreachable!("sample job is a message");
        };
        let updated = ProcessEventJob {
            event_id: "evt_2".to_owned(),
            payload: EventPayload::Message {
                user_id,
                text: Some("updated".to_owned()),
                ts,
                thread_ts,
                files: vec![],
            },
            ..original
        };

        store
            .record_process_event(&sample_job("evt_1"))
            .await
            .expect("insert original");
        store
            .record_process_event(&updated)
            .await
            .expect("insert updated");

        let messages = store.messages().await;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "updated");
    }

    #[tokio::test]
    async fn reaction_jobs_are_tracked_separately() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&path).await.expect("store");
        let reaction_job = ProcessEventJob {
            event_id: "evt_reaction".to_owned(),
            team_id: "team_1".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ReactionAdded {
                user_id: "U123".to_owned(),
                reaction: "thumbsup".to_owned(),
                item_ts: "1700000000.000001".to_owned(),
            },
        };

        store
            .record_process_event(&reaction_job)
            .await
            .expect("insert reaction");

        let reactions = store.reactions().await;
        let health = store.health().await;

        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].name, "thumbsup");
        assert_eq!(health.tracked_reactions, 1);
    }

    #[tokio::test]
    async fn file_share_messages_track_attached_files() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("events.jsonl");
        let store = JsonlEventStore::open(&path).await.expect("store");
        let file_job = ProcessEventJob {
            event_id: "evt_file".to_owned(),
            team_id: "team_1".to_owned(),
            event_time: 5,
            received_at: 6,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("uploaded brief".to_owned()),
                ts: "1700000000.000002".to_owned(),
                thread_ts: None,
                files: vec![SharedFile {
                    id: "F123".to_owned(),
                    name: "brief.pdf".to_owned(),
                    mimetype: Some("application/pdf".to_owned()),
                    permalink: Some("https://files.example.com/brief.pdf".to_owned()),
                    size: Some(42),
                }],
            },
        };

        store
            .record_process_event(&file_job)
            .await
            .expect("insert file event");

        let files = store.files().await;
        let health = store.health().await;

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "brief.pdf");
        assert_eq!(health.tracked_files, 1);
    }
}
