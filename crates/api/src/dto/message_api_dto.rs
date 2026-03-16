use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::services::message_service::{
    ConversationKind, ConversationRecord, ConversationSummary, MessageParticipant, MessageRecord,
    PollUpdate,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummaryResponse {
    pub conversation_id: String,
    pub kind: ConversationKind,
    pub participant_ids: Vec<String>,
    pub participant_names: Vec<String>,
    pub last_message_preview: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub unread_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListConversationsResponse {
    pub conversations: Vec<ConversationSummaryResponse>,
    pub next_cursor: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub conversation_id: String,
    pub kind: ConversationKind,
    pub participants: Vec<MessageParticipant>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
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
pub struct ListMessagesResponse {
    pub messages: Vec<MessageResponse>,
    pub next_cursor: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollResponse {
    pub conversations: Vec<ConversationSummaryResponse>,
    pub messages: Vec<MessageResponse>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub kind: ConversationKind,
    pub participant_ids: Vec<String>,
    pub participant_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub recipient_id: String,
    pub recipient_name: String,
    pub content: String,
    pub message_type: String,
    pub reply_to_id: Option<String>,
}

impl From<ConversationSummary> for ConversationSummaryResponse {
    fn from(value: ConversationSummary) -> Self {
        Self {
            conversation_id: value.conversation_id,
            kind: value.kind,
            participant_ids: value.participant_ids,
            participant_names: value.participant_names,
            last_message_preview: value.last_message_preview,
            last_message_at: value.last_message_at,
            unread_count: value.unread_count,
        }
    }
}

impl From<ConversationRecord> for ConversationResponse {
    fn from(value: ConversationRecord) -> Self {
        Self {
            conversation_id: value.conversation_id,
            kind: value.kind,
            participants: value.participants,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<MessageRecord> for MessageResponse {
    fn from(value: MessageRecord) -> Self {
        Self {
            message_id: value.message_id,
            conversation_id: value.conversation_id,
            sender_id: value.sender_id,
            sender_name: value.sender_name,
            recipient_id: value.recipient_id,
            recipient_name: value.recipient_name,
            content: value.content,
            timestamp: value.timestamp,
            message_type: value.message_type,
            reply_to_id: value.reply_to_id,
            read: value.read,
        }
    }
}

impl From<PollUpdate> for PollResponse {
    fn from(value: PollUpdate) -> Self {
        Self {
            conversations: value
                .conversations
                .into_iter()
                .map(ConversationSummaryResponse::from)
                .collect(),
            messages: value
                .messages
                .into_iter()
                .map(MessageResponse::from)
                .collect(),
            timestamp: value.timestamp,
        }
    }
}
