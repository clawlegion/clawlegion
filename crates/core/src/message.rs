//! Message types for agent communication

use crate::{AgentId, CompanyId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique message identifier
pub type MessageId = Uuid;

/// Message content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    /// Plain text message
    Text(String),

    /// Reply to another message
    Reply {
        text: String,
        in_reply_to: MessageId,
    },

    /// Command to execute
    Command { command: String, args: Vec<String> },

    /// System notification
    Notification {
        title: String,
        body: String,
        level: NotificationLevel,
    },
}

/// Notification severity level
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Chat type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatType {
    /// One-on-one private chat
    Private { participant: AgentId },
}

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: MessageId,
    pub company_id: CompanyId,
    pub chat_type: ChatType,
    pub sender: AgentId,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub metadata: MessageMetadata,
}

/// Message metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Tags for categorization
    pub tags: Vec<String>,

    /// Whether this message should be stored in long-term memory
    pub important: bool,

    /// Optional expiration time for important messages.
    /// If None, the message never expires.
    /// If Some, the message will be marked for deletion after this time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,

    /// Reference to related task/issue
    pub related_issue_id: Option<Uuid>,

    /// Custom key-value pairs
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

impl MessageMetadata {
    /// Create metadata that never expires (default)
    pub fn new() -> Self {
        Self::default()
    }

    /// Create metadata that expires after a duration
    pub fn expiring_in(duration: chrono::Duration) -> Self {
        Self {
            expires_at: Some(Utc::now() + duration),
            ..Default::default()
        }
    }

    /// Create metadata with an explicit expiration time
    pub fn with_expiry(expires_at: DateTime<Utc>) -> Self {
        Self {
            expires_at: Some(expires_at),
            ..Default::default()
        }
    }

    /// Check if this metadata has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| exp < Utc::now())
    }
}

impl ChatMessage {
    /// Create a new private message
    pub fn private(
        company_id: CompanyId,
        sender: AgentId,
        recipient: AgentId,
        content: MessageContent,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id,
            chat_type: ChatType::Private {
                participant: recipient,
            },
            sender,
            content,
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        }
    }

    /// Create a reply message
    pub fn reply(
        company_id: CompanyId,
        chat_type: ChatType,
        sender: AgentId,
        text: String,
        in_reply_to: MessageId,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id,
            chat_type,
            sender,
            content: MessageContent::Reply { text, in_reply_to },
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_metadata_new_never_expires() {
        let metadata = MessageMetadata::new();
        assert!(!metadata.important);
        assert!(metadata.expires_at.is_none());
        assert!(!metadata.is_expired());
    }

    #[test]
    fn test_metadata_expiring_in_future() {
        let metadata = MessageMetadata::expiring_in(Duration::hours(1));
        assert!(metadata.expires_at.is_some());
        assert!(!metadata.is_expired());
    }

    #[test]
    fn test_metadata_expiring_in_past() {
        let metadata = MessageMetadata::expiring_in(Duration::seconds(-1));
        assert!(metadata.expires_at.is_some());
        assert!(metadata.is_expired());
    }

    #[test]
    fn test_metadata_with_expiry() {
        let future_time = Utc::now() + Duration::hours(24);
        let metadata = MessageMetadata::with_expiry(future_time);
        assert_eq!(metadata.expires_at, Some(future_time));
        assert!(!metadata.is_expired());
    }

    #[test]
    fn test_metadata_is_expired() {
        let past_time = Utc::now() - Duration::hours(1);
        let metadata = MessageMetadata::with_expiry(past_time);
        assert!(metadata.is_expired());
    }

    #[test]
    fn test_metadata_default_not_expired() {
        let metadata = MessageMetadata::default();
        assert!(metadata.expires_at.is_none());
        assert!(!metadata.is_expired());
    }
}
