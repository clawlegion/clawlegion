use std::sync::Arc;

use chrono::{DateTime, Utc};
use clawlegion_agent::AgentRegistry;
use clawlegion_core::AgentStatus;
use clawlegion_plugin::{PluginManagerStats, SharedPluginManager};

use crate::services::message_service::{
    ConversationKind, ConversationRecord, ConversationSummary, MessageParticipant, MessageRecord,
    MessageService, MessageServiceConfig, MessageServiceError, MessageServiceStats, PollUpdate,
};

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub status: String,
    pub uptime_secs: u64,
    pub version: String,
    pub agents_total: usize,
    pub agents_active: usize,
    pub plugins_loaded: Option<usize>,
    pub plugins_active: Option<usize>,
    pub plugin_runtime: Option<PluginManagerStats>,
    pub message_service: MessageServiceStats,
}

#[derive(Clone)]
pub struct AppServices {
    agent_registry: Arc<AgentRegistry>,
    message_service: Arc<MessageService>,
    plugin_manager: Option<SharedPluginManager>,
    start_time: DateTime<Utc>,
    version: String,
}

impl AppServices {
    pub fn new(
        agent_registry: Arc<AgentRegistry>,
        message_service: Arc<MessageService>,
        plugin_manager: Option<SharedPluginManager>,
        start_time: DateTime<Utc>,
    ) -> Self {
        Self {
            agent_registry,
            message_service,
            plugin_manager,
            start_time,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn message_service(&self) -> Arc<MessageService> {
        Arc::clone(&self.message_service)
    }

    pub fn plugin_manager(&self) -> Option<SharedPluginManager> {
        self.plugin_manager.as_ref().map(Arc::clone)
    }

    pub fn message_service_config(&self) -> MessageServiceConfig {
        self.message_service.config()
    }

    pub async fn system_snapshot(&self) -> SystemSnapshot {
        let now = Utc::now();
        let plugin_runtime = self
            .plugin_manager
            .as_ref()
            .map(|manager| manager.read().stats());
        let message_service = self.message_service.stats().await;

        SystemSnapshot {
            status: "ok".to_string(),
            uptime_secs: (now - self.start_time).num_seconds().max(0) as u64,
            version: self.version.clone(),
            agents_total: self.agent_registry.count(),
            agents_active: self
                .agent_registry
                .list_agents_by_status(AgentStatus::Running)
                .len(),
            plugins_loaded: plugin_runtime.as_ref().map(|stats| stats.total_plugins),
            plugins_active: plugin_runtime.as_ref().map(|stats| stats.active_plugins),
            plugin_runtime,
            message_service,
        }
    }

    pub async fn list_conversations(
        &self,
        cursor: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Vec<ConversationSummary> {
        self.message_service.list_conversations(cursor, limit).await
    }

    pub async fn get_conversation(&self, conversation_id: &str) -> Option<ConversationRecord> {
        self.message_service.get_conversation(conversation_id).await
    }

    pub async fn create_conversation(
        &self,
        kind: ConversationKind,
        participants: Vec<MessageParticipant>,
    ) -> Result<ConversationRecord, MessageServiceError> {
        self.message_service
            .get_or_create_conversation(kind, participants)
            .await
    }

    pub async fn list_messages(
        &self,
        conversation_id: &str,
        cursor: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Vec<MessageRecord> {
        self.message_service
            .list_messages(conversation_id, cursor, limit)
            .await
    }

    pub async fn send_message(
        &self,
        message: MessageRecord,
    ) -> Result<MessageRecord, MessageServiceError> {
        self.message_service.send_message(message).await
    }

    pub async fn poll_updates(
        &self,
        since: DateTime<Utc>,
        conversation_limit: Option<usize>,
        message_limit: Option<usize>,
    ) -> PollUpdate {
        self.message_service
            .poll_updates(since, conversation_limit, message_limit)
            .await
    }
}
