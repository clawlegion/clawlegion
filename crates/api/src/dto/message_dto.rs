//! Message DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Message type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Private,
    Group,
    System,
}

/// Message response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message_id: String,
    pub conversation_id: String,
    pub from_agent_id: String,
    pub from_agent_name: String,
    pub to_agent_id: Option<String>,
    pub to_agent_name: Option<String>,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
    pub in_reply_to: Option<String>,
    pub read: bool,
}

/// Conversation summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub conversation_id: String,
    pub participants: Vec<ConversationParticipant>,
    pub last_message_at: DateTime<Utc>,
    pub last_message_preview: String,
    pub message_count: usize,
    pub unread_count: usize,
}

/// Conversation participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationParticipant {
    pub agent_id: String,
    pub agent_name: String,
}

/// List messages response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMessagesResponse {
    pub messages: Vec<MessageResponse>,
    pub has_more: bool,
}

/// List conversations response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListConversationsResponse {
    pub conversations: Vec<ConversationResponse>,
}
