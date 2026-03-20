//! API State - shared application state

use std::sync::Arc;

use clawlegion_agent::AgentRegistry;
use clawlegion_org::OrgConfig;
use clawlegion_org::OrgTree;
use clawlegion_plugin::SharedPluginManager;

use crate::services::{
    application_service::AppServices,
    message_service::{MessageService, MessageServiceConfig},
};

#[derive(Clone)]
pub struct ApiState {
    pub agent_registry: Arc<AgentRegistry>,
    pub org_tree: Arc<OrgTree>,
    pub org_config: Arc<OrgConfig>,
    pub plugin_manager: SharedPluginManager,
    pub message_service: Arc<MessageService>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub services: Arc<AppServices>,
}

impl ApiState {
    pub fn new(
        agent_registry: Arc<AgentRegistry>,
        org_tree: Arc<OrgTree>,
        org_config: Arc<OrgConfig>,
        plugin_manager: SharedPluginManager,
    ) -> Self {
        Self::with_message_service_config(
            agent_registry,
            org_tree,
            org_config,
            plugin_manager,
            MessageServiceConfig::default(),
        )
    }

    pub fn with_message_service_config(
        agent_registry: Arc<AgentRegistry>,
        org_tree: Arc<OrgTree>,
        org_config: Arc<OrgConfig>,
        plugin_manager: SharedPluginManager,
        message_config: MessageServiceConfig,
    ) -> Self {
        let message_service = Arc::new(MessageService::new(message_config));
        let start_time = chrono::Utc::now();
        let services = Arc::new(AppServices::new(
            Arc::clone(&agent_registry),
            Arc::clone(&message_service),
            Some(Arc::clone(&plugin_manager)),
            start_time,
        ));

        Self {
            agent_registry,
            org_tree,
            org_config,
            plugin_manager,
            message_service,
            start_time,
            services,
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        (chrono::Utc::now() - self.start_time).num_seconds().max(0) as u64
    }
}
