//! PrivateMessage trigger - wakes up agent when private message is received

use super::{BuiltinWakeupTrigger, TriggerContext};
use async_trait::async_trait;
use clawlegion_core::{AgentId, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// PrivateMessage trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateMessageConfig {
    /// Filter by sender (None = any sender)
    pub from: Option<AgentId>,
}

/// PrivateMessage trigger
///
/// Triggers when a private message is received.
pub struct PrivateMessageTrigger {
    config: PrivateMessageConfig,
}

impl PrivateMessageTrigger {
    pub fn new() -> Self {
        Self {
            config: PrivateMessageConfig { from: None },
        }
    }

    pub fn with_sender_filter(sender: AgentId) -> Self {
        Self {
            config: PrivateMessageConfig { from: Some(sender) },
        }
    }

    pub fn config(&self) -> &PrivateMessageConfig {
        &self.config
    }
}

impl Default for PrivateMessageTrigger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BuiltinWakeupTrigger for PrivateMessageTrigger {
    fn trigger_type(&self) -> &str {
        "private_message"
    }

    async fn should_trigger(&self, context: &TriggerContext) -> Result<bool> {
        // Check if message_id is provided in context
        let has_message = context.get("message_id").is_some();

        if !has_message {
            return Ok(false);
        }

        // If sender filter is configured, check if sender matches
        if let Some(filter_sender) = self.config.from {
            if let Some(sender_value) = context.get("sender") {
                if let Some(sender_str) = sender_value.as_str() {
                    if let Ok(sender_id) = Uuid::parse_str(sender_str) {
                        return Ok(sender_id == filter_sender);
                    }
                }
            }
            return Ok(false);
        }

        Ok(true)
    }

    async fn wakeup(&self, agent_id: AgentId, data: serde_json::Value) -> Result<()> {
        tracing::info!(
            "PrivateMessageTrigger: Waking up agent {} with data: {}",
            agent_id,
            data
        );

        // In a real implementation, this would:
        // 1. Retrieve the actual message from storage
        // 2. Pass it to the agent for processing
        // 3. Update message status as "delivered"

        Ok(())
    }
}

/// Private message wakeup data
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateMessageWakeupData {
    /// Message ID
    pub message_id: Uuid,

    /// Sender ID
    pub sender: AgentId,

    /// Message content
    pub content: String,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PrivateMessageWakeupData {
    #[allow(dead_code)]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "message_id": self.message_id.to_string(),
            "sender": self.sender.to_string(),
            "content": self.content,
            "timestamp": self.timestamp.to_rfc3339()
        })
    }
}
