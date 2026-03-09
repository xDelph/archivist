use crate::{
    RepositoryHealth, SearchDocumentRow, StoreOutcome,
    search_index::{MessageMap, SearchDocumentMap, refresh_search_documents},
};
use domain::{Channel, EventPayload, File, Message, ProcessEventJob, Reaction};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

type MessageKey = (String, String);
type FileKey = (String, String);
type ReactionKey = (String, String, String, String, String);

#[derive(Debug, Clone, Default)]
pub struct InMemoryEventStore {
    state: Arc<Mutex<InMemoryState>>,
}

#[derive(Debug, Default)]
struct InMemoryState {
    seen_events: HashSet<String>,
    messages: MessageMap,
    files: HashMap<FileKey, File>,
    channels: HashMap<(String, String), Channel>,
    reactions: HashSet<ReactionKey>,
    search_documents: SearchDocumentMap,
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

#[cfg(test)]
mod tests {
    use super::InMemoryEventStore;
    use crate::StoreOutcome;
    use domain::{ChannelKind, EventPayload, ProcessEventJob, SharedFile};

    #[tokio::test]
    async fn in_memory_store_deduplicates_and_tracks_entities() {
        let store = InMemoryEventStore::new();
        let rename = ProcessEventJob {
            event_id: "evt_channel".to_owned(),
            team_id: "T123".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ChannelUpdated {
                name: Some("announcements".to_owned()),
                is_archived: Some(false),
            },
        };
        let message = ProcessEventJob {
            event_id: "evt_message".to_owned(),
            team_id: "T123".to_owned(),
            event_time: 3,
            received_at: 4,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("hello".to_owned()),
                ts: "1700000000.000001".to_owned(),
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

        assert_eq!(
            store.record_process_event(&rename).await,
            StoreOutcome::Inserted
        );
        assert_eq!(
            store.record_process_event(&message).await,
            StoreOutcome::Inserted
        );
        assert_eq!(
            store.record_process_event(&message).await,
            StoreOutcome::Duplicate
        );

        let health = store.health().await;
        let files = store.files().await;
        let channels = store.channels().await;

        assert_eq!(health.tracked_events, 2);
        assert_eq!(health.tracked_messages, 1);
        assert_eq!(health.tracked_files, 1);
        assert_eq!(health.tracked_channels, 1);
        assert_eq!(files[0].name, "brief.pdf");
        assert_eq!(channels[0].name.as_deref(), Some("announcements"));
    }

    #[tokio::test]
    async fn in_memory_store_tracks_reactions() {
        let store = InMemoryEventStore::new();
        let reaction = ProcessEventJob {
            event_id: "evt_reaction".to_owned(),
            team_id: "T123".to_owned(),
            event_time: 1,
            received_at: 2,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::ReactionAdded {
                user_id: "U123".to_owned(),
                reaction: "thumbsup".to_owned(),
                item_ts: "1700000000.000001".to_owned(),
            },
        };

        assert_eq!(
            store.record_process_event(&reaction).await,
            StoreOutcome::Inserted
        );

        let reactions = store.reactions().await;
        let health = store.health().await;

        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].name, "thumbsup");
        assert_eq!(health.tracked_reactions, 1);
    }

    #[tokio::test]
    async fn in_memory_store_refreshes_search_documents_for_threads() {
        let store = InMemoryEventStore::new();
        let reply = ProcessEventJob {
            event_id: "evt_reply".to_owned(),
            team_id: "T123".to_owned(),
            event_time: 5,
            received_at: 6,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U456".to_owned()),
                text: Some("reply details".to_owned()),
                ts: "1700000000.000002".to_owned(),
                thread_ts: Some("1700000000.000001".to_owned()),
                files: vec![],
            },
        };
        let root = ProcessEventJob {
            event_id: "evt_root".to_owned(),
            team_id: "T123".to_owned(),
            event_time: 7,
            received_at: 8,
            channel_id: "C123".to_owned(),
            channel_kind: ChannelKind::Public,
            payload: EventPayload::Message {
                user_id: Some("U123".to_owned()),
                text: Some("root summary".to_owned()),
                ts: "1700000000.000001".to_owned(),
                thread_ts: None,
                files: vec![],
            },
        };

        assert_eq!(store.record_process_event(&reply).await, StoreOutcome::Inserted);
        assert_eq!(store.record_process_event(&root).await, StoreOutcome::Inserted);

        let search_documents = store.search_documents().await;

        assert_eq!(search_documents.len(), 2);
        assert!(search_documents
            .iter()
            .all(|document| document.title.as_deref() == Some("root summary")));
    }
}
