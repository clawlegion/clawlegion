use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConversationKind {
    AgentAgent,
    UserAgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageParticipant {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    pub conversation_id: String,
    pub kind: ConversationKind,
    pub participants: Vec<MessageParticipant>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub recipient_id: String,
    pub recipient_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: String,
    pub reply_to_id: Option<String>,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub conversation_id: String,
    pub kind: ConversationKind,
    pub participant_ids: Vec<String>,
    pub participant_names: Vec<String>,
    pub last_message_preview: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub unread_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollUpdate {
    pub conversations: Vec<ConversationSummary>,
    pub messages: Vec<MessageRecord>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MessageServiceConfig {
    pub max_conversations: usize,
    pub max_messages: usize,
    pub max_messages_per_conversation: usize,
    pub default_page_size: usize,
    pub max_page_size: usize,
}

impl Default for MessageServiceConfig {
    fn default() -> Self {
        Self {
            max_conversations: 1_000,
            max_messages: 50_000,
            max_messages_per_conversation: 2_000,
            default_page_size: 50,
            max_page_size: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageServiceStats {
    pub conversations: usize,
    pub messages: usize,
    pub unread_messages: usize,
    pub estimated_memory_bytes: u64,
    pub max_conversations: usize,
    pub max_messages: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageServiceError {
    ConversationNotFound(String),
    ConversationCapacityExceeded,
    InvalidParticipantSet,
    MessageCapacityExceeded,
    PageSizeTooLarge { requested: usize, max: usize },
}

impl fmt::Display for MessageServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConversationNotFound(id) => write!(f, "conversation not found: {id}"),
            Self::ConversationCapacityExceeded => {
                write!(f, "conversation capacity exceeded")
            }
            Self::InvalidParticipantSet => write!(f, "invalid participant set"),
            Self::MessageCapacityExceeded => write!(f, "message capacity exceeded"),
            Self::PageSizeTooLarge { requested, max } => {
                write!(f, "page size {requested} exceeds configured maximum {max}")
            }
        }
    }
}

impl std::error::Error for MessageServiceError {}

#[derive(Debug, Default)]
struct MessageStore {
    conversations: HashMap<String, ConversationRecord>,
    messages: HashMap<String, MessageRecord>,
    conversation_messages: HashMap<String, Vec<String>>,
}

pub struct MessageService {
    config: MessageServiceConfig,
    store: RwLock<MessageStore>,
}

impl Default for MessageService {
    fn default() -> Self {
        Self::new(MessageServiceConfig::default())
    }
}

impl MessageService {
    pub fn new(config: MessageServiceConfig) -> Self {
        Self {
            config,
            store: RwLock::new(MessageStore::default()),
        }
    }

    pub fn config(&self) -> MessageServiceConfig {
        self.config
    }

    pub async fn list_conversations(
        &self,
        cursor: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Vec<ConversationSummary> {
        let page_size = self
            .clamp_page_size(limit)
            .unwrap_or(self.config.default_page_size);
        let store = self.store.read().await;
        let mut items: Vec<_> = store
            .conversations
            .values()
            .filter_map(|conversation| {
                let related = store
                    .conversation_messages
                    .get(&conversation.conversation_id)
                    .into_iter()
                    .flat_map(|message_ids| message_ids.iter())
                    .filter_map(|message_id| store.messages.get(message_id))
                    .collect::<Vec<_>>();

                let last_message = related.last().copied();
                let last_activity = last_message
                    .map(|message| message.timestamp)
                    .unwrap_or(conversation.updated_at);

                if cursor.is_some_and(|value| last_activity >= value) {
                    return None;
                }

                let unread_count = related.iter().filter(|message| !message.read).count();

                Some(ConversationSummary {
                    conversation_id: conversation.conversation_id.clone(),
                    kind: conversation.kind.clone(),
                    participant_ids: conversation
                        .participants
                        .iter()
                        .map(|participant| participant.id.clone())
                        .collect(),
                    participant_names: conversation
                        .participants
                        .iter()
                        .map(|participant| participant.name.clone())
                        .collect(),
                    last_message_preview: last_message.map(|message| preview(&message.content)),
                    last_message_at: last_message.map(|message| message.timestamp),
                    unread_count,
                })
            })
            .collect();

        items.sort_by_key(|item| Reverse(item.last_message_at));
        items.truncate(page_size);
        items
    }

    pub async fn get_conversation(&self, conversation_id: &str) -> Option<ConversationRecord> {
        self.store
            .read()
            .await
            .conversations
            .get(conversation_id)
            .cloned()
    }

    pub async fn get_or_create_conversation(
        &self,
        kind: ConversationKind,
        participants: Vec<MessageParticipant>,
    ) -> Result<ConversationRecord, MessageServiceError> {
        if participants.is_empty() {
            return Err(MessageServiceError::InvalidParticipantSet);
        }

        let participant_ids: HashSet<_> =
            participants.iter().map(|item| item.id.as_str()).collect();
        if participant_ids.len() != participants.len() {
            return Err(MessageServiceError::InvalidParticipantSet);
        }

        let mut store = self.store.write().await;
        if let Some(existing) = store
            .conversations
            .values()
            .find(|conversation| {
                conversation.kind == kind
                    && conversation.participants.len() == participants.len()
                    && conversation
                        .participants
                        .iter()
                        .all(|participant| participant_ids.contains(participant.id.as_str()))
            })
            .cloned()
        {
            return Ok(existing);
        }

        self.evict_conversations_if_needed(&mut store)?;

        let now = Utc::now();
        let conversation = ConversationRecord {
            conversation_id: Uuid::new_v4().to_string(),
            kind,
            participants,
            created_at: now,
            updated_at: now,
        };
        store
            .conversation_messages
            .entry(conversation.conversation_id.clone())
            .or_default();
        store
            .conversations
            .insert(conversation.conversation_id.clone(), conversation.clone());

        Ok(conversation)
    }

    pub async fn list_messages(
        &self,
        conversation_id: &str,
        cursor: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Vec<MessageRecord> {
        let page_size = self
            .clamp_page_size(limit)
            .unwrap_or(self.config.default_page_size);
        let store = self.store.read().await;
        let mut messages: Vec<_> = store
            .conversation_messages
            .get(conversation_id)
            .into_iter()
            .flat_map(|message_ids| message_ids.iter())
            .filter_map(|message_id| store.messages.get(message_id))
            .filter(|message| {
                cursor
                    .map(|value| message.timestamp > value)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        messages.sort_by_key(|message| message.timestamp);
        if messages.len() > page_size {
            messages = messages.split_off(messages.len() - page_size);
        }
        messages
    }

    pub async fn send_message(
        &self,
        message: MessageRecord,
    ) -> Result<MessageRecord, MessageServiceError> {
        let mut store = self.store.write().await;
        let conversation = store
            .conversations
            .get_mut(&message.conversation_id)
            .ok_or_else(|| {
                MessageServiceError::ConversationNotFound(message.conversation_id.clone())
            })?;
        conversation.updated_at = message.timestamp;

        let message_id = message.message_id.clone();
        store.messages.insert(message_id.clone(), message.clone());
        store
            .conversation_messages
            .entry(message.conversation_id.clone())
            .or_default()
            .push(message_id);

        self.trim_conversation_messages(&mut store, &message.conversation_id);
        self.trim_global_messages(&mut store)?;

        Ok(message)
    }

    pub async fn poll_updates(
        &self,
        since: DateTime<Utc>,
        conversation_limit: Option<usize>,
        message_limit: Option<usize>,
    ) -> PollUpdate {
        let store = self.store.read().await;
        let conversation_page_size = self
            .clamp_page_size(conversation_limit)
            .unwrap_or(self.config.default_page_size);
        let mut conversations: Vec<_> = store
            .conversations
            .values()
            .filter_map(|conversation| {
                let related = store
                    .conversation_messages
                    .get(&conversation.conversation_id)
                    .into_iter()
                    .flat_map(|message_ids| message_ids.iter())
                    .filter_map(|message_id| store.messages.get(message_id))
                    .collect::<Vec<_>>();

                let last_message = related.last().copied();
                let last_activity = last_message
                    .map(|message| message.timestamp)
                    .unwrap_or(conversation.updated_at);
                if last_activity <= since {
                    return None;
                }

                Some(ConversationSummary {
                    conversation_id: conversation.conversation_id.clone(),
                    kind: conversation.kind.clone(),
                    participant_ids: conversation
                        .participants
                        .iter()
                        .map(|participant| participant.id.clone())
                        .collect(),
                    participant_names: conversation
                        .participants
                        .iter()
                        .map(|participant| participant.name.clone())
                        .collect(),
                    last_message_preview: last_message.map(|message| preview(&message.content)),
                    last_message_at: last_message.map(|message| message.timestamp),
                    unread_count: related.iter().filter(|message| !message.read).count(),
                })
            })
            .collect();
        conversations.sort_by_key(|item| Reverse(item.last_message_at));
        conversations.truncate(conversation_page_size);

        let page_size = self
            .clamp_page_size(message_limit)
            .unwrap_or(self.config.default_page_size);
        let mut messages: Vec<_> = store
            .messages
            .values()
            .filter(|message| message.timestamp > since)
            .cloned()
            .collect();
        messages.sort_by_key(|message| message.timestamp);
        if messages.len() > page_size {
            messages = messages.split_off(messages.len() - page_size);
        }

        PollUpdate {
            conversations,
            messages,
            timestamp: Utc::now(),
        }
    }

    pub async fn stats(&self) -> MessageServiceStats {
        let store = self.store.read().await;
        let estimated_memory_bytes = store
            .conversations
            .values()
            .map(estimate_conversation_size)
            .sum::<u64>()
            + store
                .messages
                .values()
                .map(estimate_message_size)
                .sum::<u64>();

        MessageServiceStats {
            conversations: store.conversations.len(),
            messages: store.messages.len(),
            unread_messages: store
                .messages
                .values()
                .filter(|message| !message.read)
                .count(),
            estimated_memory_bytes,
            max_conversations: self.config.max_conversations,
            max_messages: self.config.max_messages,
        }
    }

    fn clamp_page_size(&self, requested: Option<usize>) -> Result<usize, MessageServiceError> {
        let size = requested.unwrap_or(self.config.default_page_size);
        if size > self.config.max_page_size {
            return Err(MessageServiceError::PageSizeTooLarge {
                requested: size,
                max: self.config.max_page_size,
            });
        }
        Ok(size)
    }

    fn evict_conversations_if_needed(
        &self,
        store: &mut MessageStore,
    ) -> Result<(), MessageServiceError> {
        if store.conversations.len() < self.config.max_conversations {
            return Ok(());
        }

        let oldest_id = store
            .conversations
            .values()
            .min_by_key(|conversation| conversation.updated_at)
            .map(|conversation| conversation.conversation_id.clone())
            .ok_or(MessageServiceError::ConversationCapacityExceeded)?;

        self.remove_conversation(store, &oldest_id);
        Ok(())
    }

    fn trim_conversation_messages(&self, store: &mut MessageStore, conversation_id: &str) {
        while store
            .conversation_messages
            .get(conversation_id)
            .map(|message_ids| message_ids.len())
            .unwrap_or_default()
            > self.config.max_messages_per_conversation
        {
            if let Some(message_id) = store
                .conversation_messages
                .get_mut(conversation_id)
                .and_then(|message_ids| {
                    if message_ids.is_empty() {
                        None
                    } else {
                        Some(message_ids.remove(0))
                    }
                })
            {
                store.messages.remove(&message_id);
            }
        }
    }

    fn trim_global_messages(&self, store: &mut MessageStore) -> Result<(), MessageServiceError> {
        while store.messages.len() > self.config.max_messages {
            let oldest = store
                .messages
                .values()
                .min_by_key(|message| message.timestamp)
                .map(|message| (message.conversation_id.clone(), message.message_id.clone()))
                .ok_or(MessageServiceError::MessageCapacityExceeded)?;

            if let Some(message_ids) = store.conversation_messages.get_mut(&oldest.0) {
                message_ids.retain(|message_id| message_id != &oldest.1);
            }
            store.messages.remove(&oldest.1);
        }
        Ok(())
    }

    fn remove_conversation(&self, store: &mut MessageStore, conversation_id: &str) {
        if let Some(message_ids) = store.conversation_messages.remove(conversation_id) {
            for message_id in message_ids {
                store.messages.remove(&message_id);
            }
        }
        store.conversations.remove(conversation_id);
    }
}

fn preview(content: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 80;
    let mut chars = content.chars();
    let preview: String = chars.by_ref().take(MAX_PREVIEW_CHARS).collect();
    if chars.next().is_some() {
        format!("{preview}...")
    } else {
        preview
    }
}

fn estimate_conversation_size(conversation: &ConversationRecord) -> u64 {
    conversation.conversation_id.len() as u64
        + conversation
            .participants
            .iter()
            .map(|participant| (participant.id.len() + participant.name.len()) as u64)
            .sum::<u64>()
        + 64
}

fn estimate_message_size(message: &MessageRecord) -> u64 {
    (message.message_id.len()
        + message.conversation_id.len()
        + message.sender_id.len()
        + message.sender_name.len()
        + message.recipient_id.len()
        + message.recipient_name.len()
        + message.content.len()
        + message.message_type.len()
        + message
            .reply_to_id
            .as_ref()
            .map(|id| id.len())
            .unwrap_or_default()) as u64
        + 96
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_participants() -> Vec<MessageParticipant> {
        vec![
            MessageParticipant {
                id: "agent-a".to_string(),
                name: "Agent A".to_string(),
            },
            MessageParticipant {
                id: "agent-b".to_string(),
                name: "Agent B".to_string(),
            },
        ]
    }

    fn sample_message(conversation_id: &str, index: usize) -> MessageRecord {
        MessageRecord {
            message_id: format!("message-{index}"),
            conversation_id: conversation_id.to_string(),
            sender_id: "agent-a".to_string(),
            sender_name: "Agent A".to_string(),
            recipient_id: "agent-b".to_string(),
            recipient_name: "Agent B".to_string(),
            content: format!("message-{index}"),
            timestamp: Utc::now() + chrono::Duration::seconds(index as i64),
            message_type: "private".to_string(),
            reply_to_id: None,
            read: index % 2 == 0,
        }
    }

    #[tokio::test]
    async fn uses_indexed_message_storage_and_pagination() {
        let service = MessageService::new(MessageServiceConfig {
            max_messages_per_conversation: 3,
            max_messages: 10,
            ..MessageServiceConfig::default()
        });
        let conversation = service
            .get_or_create_conversation(ConversationKind::AgentAgent, sample_participants())
            .await
            .expect("conversation");

        for index in 0..5 {
            service
                .send_message(sample_message(&conversation.conversation_id, index))
                .await
                .expect("message");
        }

        let messages = service
            .list_messages(&conversation.conversation_id, None, Some(10))
            .await;
        assert_eq!(messages.len(), 3);
        assert_eq!(messages.first().unwrap().message_id, "message-2");
        assert_eq!(messages.last().unwrap().message_id, "message-4");
    }

    #[tokio::test]
    async fn evicts_oldest_conversation_when_capacity_is_reached() {
        let service = MessageService::new(MessageServiceConfig {
            max_conversations: 1,
            ..MessageServiceConfig::default()
        });

        let first = service
            .get_or_create_conversation(ConversationKind::AgentAgent, sample_participants())
            .await
            .expect("first");

        let second = service
            .get_or_create_conversation(
                ConversationKind::UserAgent,
                vec![MessageParticipant {
                    id: "user-1".to_string(),
                    name: "User".to_string(),
                }],
            )
            .await
            .expect("second");

        assert!(service
            .get_conversation(&first.conversation_id)
            .await
            .is_none());
        assert!(service
            .get_conversation(&second.conversation_id)
            .await
            .is_some());
    }
}
