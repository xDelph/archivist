use crate::{
    GeneratedThreadSummaryRow, RepositoryHealth, SearchDocumentRow, StoreOutcome, ThreadCardRow,
    ThreadSummaryRow,
    search_index::{MessageMap, SearchDocumentMap, refresh_search_documents},
    thread_card_index::{ThreadCardMap, build_thread_cards},
    thread_summary_index::{ThreadSummaryMap, build_thread_summaries},
};
use domain::{Channel, EventPayload, File, Message, ProcessEventJob, Reaction};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

type FileKey = (String, String, String);
type ReactionKey = (String, String, String, String);
type GeneratedThreadSummaryKey = (String, String);

#[derive(Debug, Clone, Default)]
pub struct InMemoryEventStore {
    state: Arc<Mutex<InMemoryState>>,
}

#[derive(Debug, Default)]
struct InMemoryState {
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

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self::default()
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

    pub async fn record_process_event(&self, job: &ProcessEventJob) -> StoreOutcome {
        let mut state = self.state.lock().await;
        if state.seen_events.contains(&job.event_id) {
            return StoreOutcome::Duplicate;
        }

        state.seen_events.insert(job.event_id.clone());
        state.apply_job(job);
        state.thread_summaries =
            build_thread_summaries(&state.messages, &state.reactions, &state.files);
        state.thread_cards = build_thread_cards(&state.messages, &state.thread_summaries);

        StoreOutcome::Inserted
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

    pub async fn upsert_generated_thread_summary(&self, row: &GeneratedThreadSummaryRow) {
        let mut state = self.state.lock().await;
        state
            .generated_thread_summaries
            .insert((row.channel_id.clone(), row.root_ts.clone()), row.clone());
    }
}

impl InMemoryState {
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

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
