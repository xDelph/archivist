use crate::{
    BackfillBatchStats, GeneratedThreadSummaryRow, RepositoryHealth, SearchDocumentRow, StoreError,
    StoreOutcome, ThreadCardRow, ThreadSummaryRow,
    search_index::{MessageMap, SearchDocumentMap, refresh_search_documents},
    thread_card_index::{ThreadCardMap, build_thread_cards},
    thread_summary_index::{ThreadSummaryMap, build_thread_summaries},
};
use domain::{Channel, EventPayload, File, Message, ProcessEventJob, Reaction};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{fs, io::AsyncWriteExt, sync::Mutex};

type FileKey = (String, String, String);
type ReactionKey = (String, String, String, String);
type GeneratedThreadSummaryKey = (String, String);

#[derive(Debug, Clone)]
pub struct JsonlEventStore {
    path: Arc<PathBuf>,
    generated_summary_path: Arc<PathBuf>,
    state: Arc<Mutex<StoreState>>,
}

#[derive(Debug, Default)]
struct StoreState {
    seen_events: HashSet<String>,
    messages: MessageMap,
    files: HashMap<FileKey, File>,
    channels: HashMap<String, Channel>,
    reactions: HashSet<ReactionKey>,
    search_documents: SearchDocumentMap,
    thread_summaries: ThreadSummaryMap,
    thread_cards: ThreadCardMap,
    generated_thread_summaries: HashMap<GeneratedThreadSummaryKey, GeneratedThreadSummaryRow>,
}

impl JsonlEventStore {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        let generated_summary_path = generated_summary_path(&path);
        let mut state = load_state(&path).await?;
        state.generated_thread_summaries =
            load_generated_thread_summaries(&generated_summary_path).await?;

        Ok(Self {
            path: Arc::new(path),
            generated_summary_path: Arc::new(generated_summary_path),
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
        state.thread_cards = build_thread_cards(&state.messages, &state.thread_summaries);

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

    pub async fn latest_message_ts(&self, channel_id: &str) -> Option<String> {
        let state = self.state.lock().await;
        state
            .messages
            .values()
            .filter(|message| message.channel_id == channel_id)
            .max_by(|left, right| left.ts.cmp(&right.ts))
            .map(|message| message.ts.clone())
    }

    pub async fn reactions(&self) -> Vec<Reaction> {
        let state = self.state.lock().await;
        let mut reactions = state
            .reactions
            .iter()
            .map(
                |(channel_id, message_ts, user_id, reaction_name)| Reaction {
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
        channels.sort_by(|left, right| left.id.cmp(&right.id));
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

    pub async fn thread_cards(&self) -> Vec<ThreadCardRow> {
        let state = self.state.lock().await;
        let mut thread_cards = state.thread_cards.values().cloned().collect::<Vec<_>>();
        thread_cards.sort_by(|left, right| {
            (&left.channel_id, &left.root_ts).cmp(&(&right.channel_id, &right.root_ts))
        });
        thread_cards
    }

    pub async fn generated_thread_summaries(&self) -> Vec<GeneratedThreadSummaryRow> {
        let state = self.state.lock().await;
        let mut generated_thread_summaries = state
            .generated_thread_summaries
            .values()
            .cloned()
            .collect::<Vec<_>>();
        generated_thread_summaries.sort_by(|left, right| {
            (&left.channel_id, &left.root_ts).cmp(&(&right.channel_id, &right.root_ts))
        });
        generated_thread_summaries
    }

    pub async fn refresh_thread_summaries(&self) -> usize {
        let mut state = self.state.lock().await;
        state.thread_summaries =
            build_thread_summaries(&state.messages, &state.reactions, &state.files);
        state.thread_cards = build_thread_cards(&state.messages, &state.thread_summaries);
        state.thread_summaries.len()
    }

    pub async fn upsert_user_profile(
        &self,
        _user_id: &str,
        _email: Option<&str>,
        _display_name: Option<&str>,
        _avatar_url: Option<&str>,
        _is_active: bool,
    ) {
    }

    pub async fn set_file_archive(&self, file_id: &str, storage_key: &str, storage_url: &str) {
        let mut state = self.state.lock().await;
        for file in state.files.values_mut() {
            if file.id == file_id {
                file.permalink = Some(storage_url.to_owned());
                tracing::debug!(%file_id, %storage_key, "updated local file archive metadata");
            }
        }
    }

    pub async fn upsert_generated_thread_summary(
        &self,
        row: &GeneratedThreadSummaryRow,
    ) -> Result<(), StoreError> {
        let mut state = self.state.lock().await;
        state
            .generated_thread_summaries
            .insert((row.channel_id.clone(), row.root_ts.clone()), row.clone());
        persist_generated_thread_summaries(
            &self.generated_summary_path,
            &state.generated_thread_summaries,
        )
        .await
    }

    pub async fn backfill_channel_jobs(
        &self,
        channel_job: Option<&ProcessEventJob>,
        message_jobs: &[ProcessEventJob],
        reaction_jobs: &[ProcessEventJob],
    ) -> Result<BackfillBatchStats, StoreError> {
        let mut stats = BackfillBatchStats::default();

        if let Some(job) = channel_job {
            let _ = self.record_process_event(job).await?;
        }

        for job in message_jobs {
            match self.record_process_event(job).await? {
                StoreOutcome::Inserted => stats.messages_inserted += 1,
                StoreOutcome::Duplicate => stats.messages_duplicate += 1,
            }
        }

        for job in reaction_jobs {
            match self.record_process_event(job).await? {
                StoreOutcome::Inserted => stats.reactions_inserted += 1,
                StoreOutcome::Duplicate => stats.reactions_duplicate += 1,
            }
        }

        Ok(stats)
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
                        channel_id: job.channel_id.clone(),
                        ts: ts.clone(),
                        thread_ts: thread_ts.clone(),
                        user_id: user_id.clone(),
                        text: text.clone().unwrap_or_default(),
                    },
                );
                for file in files {
                    self.files.insert(
                        (job.channel_id.clone(), ts.clone(), file.id.clone()),
                        File {
                            id: file.id.clone(),
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
                    job.channel_id.clone(),
                    item_ts.clone(),
                    user_id.clone(),
                    reaction.clone(),
                ));
            }
            EventPayload::ChannelUpdated { name, is_archived } => {
                let key = job.channel_id.clone();
                let existing = self.channels.remove(&key);
                let mut channel = existing.unwrap_or(Channel {
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
    state.thread_cards = build_thread_cards(&state.messages, &state.thread_summaries);

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

fn generated_summary_path(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("events.jsonl");
    path.with_file_name(format!("{filename}.generated-thread-summaries.json"))
}

async fn load_generated_thread_summaries(
    path: &Path,
) -> Result<HashMap<GeneratedThreadSummaryKey, GeneratedThreadSummaryRow>, StoreError> {
    if !fs::try_exists(path).await.map_err(StoreError::Read)? {
        return Ok(HashMap::new());
    }

    let contents = fs::read_to_string(path).await.map_err(StoreError::Read)?;
    if contents.trim().is_empty() {
        return Ok(HashMap::new());
    }

    let rows = serde_json::from_str::<Vec<GeneratedThreadSummaryRow>>(&contents)?;
    Ok(rows
        .into_iter()
        .map(|row| ((row.channel_id.clone(), row.root_ts.clone()), row))
        .collect())
}

async fn persist_generated_thread_summaries(
    path: &Path,
    summaries: &HashMap<GeneratedThreadSummaryKey, GeneratedThreadSummaryRow>,
) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(StoreError::CreateDirectory)?;
    }

    let mut rows = summaries.values().cloned().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (&left.channel_id, &left.root_ts).cmp(&(&right.channel_id, &right.root_ts))
    });
    let payload = serde_json::to_vec_pretty(&rows)?;
    fs::write(path, payload).await.map_err(StoreError::Append)
}

#[cfg(not(test))]
pub(crate) fn runtime_database_url() -> Result<String, StoreError> {
    std::env::var("POSTGRES_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or(StoreError::MissingDatabaseUrl)
}
