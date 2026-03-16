//! Agent Runtime - manages agent lifecycle and execution

use crate::{AgentFactory, AgentRegistry};
use clawlegion_core::{
    Agent, AgentConfig, AgentError, AgentId, AgentInfo, AgentStatus, Error, HeartbeatContext,
    HeartbeatTrigger, Result,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, Copy)]
pub struct AgentRuntimeConfig {
    pub shutdown_channel_capacity: usize,
    pub heartbeat_interval_secs: u64,
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            shutdown_channel_capacity: 100,
            heartbeat_interval_secs: 60,
        }
    }
}

pub struct AgentRuntime {
    registry: Arc<AgentRegistry>,
    configs: RwLock<HashMap<AgentId, AgentConfig>>,
    shutdown_tx: broadcast::Sender<AgentId>,
    running_agents: RwLock<HashMap<AgentId, tokio::task::JoinHandle<()>>>,
    config: AgentRuntimeConfig,
}

impl AgentRuntime {
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self::new_with_config(registry, AgentRuntimeConfig::default())
    }

    pub fn new_with_config(registry: Arc<AgentRegistry>, config: AgentRuntimeConfig) -> Self {
        let capacity = config.shutdown_channel_capacity.max(1);
        let (shutdown_tx, _) = broadcast::channel(capacity);

        Self {
            registry,
            configs: RwLock::new(HashMap::new()),
            shutdown_tx,
            running_agents: RwLock::new(HashMap::new()),
            config,
        }
    }

    pub fn with_heartbeat_interval(mut self, interval_secs: u64) -> Self {
        self.config.heartbeat_interval_secs = interval_secs.max(1);
        self
    }

    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    pub fn config(&self) -> AgentRuntimeConfig {
        self.config
    }

    pub async fn register_agent(
        &self,
        config: AgentConfig,
        agent: Box<dyn Agent>,
    ) -> Result<AgentId> {
        let id = config.id;
        self.configs.write().await.insert(id, config);
        self.registry.register(agent)?;
        self.start_heartbeat_loop(id).await;
        Ok(id)
    }

    pub async fn create_agent(
        &self,
        config: AgentConfig,
        capabilities: Arc<dyn crate::AgentCapabilities>,
    ) -> Result<AgentId> {
        let agent = AgentFactory::create_agent(&config, capabilities)?;
        self.register_agent(config, agent).await
    }

    pub fn get_agent(&self, id: AgentId) -> Option<Arc<RwLock<Box<dyn Agent>>>> {
        self.registry.get(id)
    }

    pub fn get_agent_info(&self, id: AgentId) -> Option<AgentInfo> {
        self.registry.get_info(id)
    }

    pub async fn start_agent(&self, id: AgentId) -> Result<()> {
        if self.running_agents.read().await.contains_key(&id) {
            return Ok(());
        }

        let agent = self
            .registry
            .get(id)
            .ok_or_else(|| Error::Agent(AgentError::NotFound(id.to_string())))?;
        agent.write().await.set_status(AgentStatus::Running);
        self.start_heartbeat_loop(id).await;
        Ok(())
    }

    pub async fn stop_agent(&self, id: AgentId) -> Result<()> {
        let _ = self.shutdown_tx.send(id);

        if let Some(handle) = self.running_agents.write().await.remove(&id) {
            handle.await.map_err(|error| {
                Error::Agent(AgentError::ExecutionFailed(format!(
                    "agent task join failed for {id}: {error}"
                )))
            })?;
        }

        if let Some(agent) = self.registry.get(id) {
            let mut agent_ref = agent.write().await;
            agent_ref.shutdown().await?;
            agent_ref.set_status(AgentStatus::Stopping);
        }

        Ok(())
    }

    pub async fn remove_agent(&self, id: AgentId) -> Result<()> {
        self.stop_agent(id).await?;
        self.registry.unregister(id)?;
        self.configs.write().await.remove(&id);
        Ok(())
    }

    pub async fn trigger_heartbeat(&self, id: AgentId, trigger: HeartbeatTrigger) -> Result<()> {
        let agent = self
            .registry
            .get(id)
            .ok_or_else(|| Error::Agent(AgentError::NotFound(id.to_string())))?;
        let ctx = HeartbeatContext {
            trigger,
            timestamp: chrono::Utc::now(),
        };

        let mut agent_ref = agent.write().await;
        let _ = agent_ref.heartbeat(ctx).await;
        self.registry.update_agent_info(id).await?;
        Ok(())
    }

    async fn start_heartbeat_loop(&self, agent_id: AgentId) {
        if self.running_agents.read().await.contains_key(&agent_id) {
            return;
        }

        let registry = self.registry.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let interval_secs = self.config.heartbeat_interval_secs.max(1);

        let handle = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Some(agent) = registry.get(agent_id) {
                            let ctx = HeartbeatContext {
                                trigger: HeartbeatTrigger::Scheduled,
                                timestamp: chrono::Utc::now(),
                            };

                            let mut agent_ref = agent.write().await;
                            let _ = agent_ref.heartbeat(ctx).await;
                            let _ = registry.update_agent_info(agent_id).await;
                        } else {
                            break;
                        }
                    }
                    Ok(shutdown_id) = shutdown_rx.recv() => {
                        if shutdown_id == agent_id {
                            break;
                        }
                    }
                }
            }
        });

        self.running_agents.write().await.insert(agent_id, handle);
    }

    pub async fn shutdown_all(&self) -> Result<()> {
        let agent_ids: Vec<AgentId> = self.running_agents.read().await.keys().cloned().collect();
        for id in agent_ids {
            let _ = self.stop_agent(id).await;
        }
        Ok(())
    }

    pub async fn get_stats(&self) -> RuntimeStats {
        RuntimeStats {
            total_agents: self.registry.count(),
            running_agents: self.running_agents.read().await.len(),
            registered_configs: self.configs.read().await.len(),
            heartbeat_interval_secs: self.config.heartbeat_interval_secs,
            shutdown_channel_capacity: self.config.shutdown_channel_capacity,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub total_agents: usize,
    pub running_agents: usize,
    pub registered_configs: usize,
    pub heartbeat_interval_secs: u64,
    pub shutdown_channel_capacity: usize,
}

pub type SharedAgentRuntime = Arc<AgentRuntime>;
