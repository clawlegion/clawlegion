//! Agent Registry - manages agent registration and discovery

use clawlegion_core::{Agent, AgentError, AgentId, AgentInfo, Error, Result};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent Registry
///
/// Central registry for all agents in the system.
pub struct AgentRegistry {
    /// Registered agents
    agents: DashMap<AgentId, Arc<RwLock<Box<dyn Agent>>>>,

    /// Agent metadata cache
    agent_info: DashMap<AgentId, AgentInfo>,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
            agent_info: DashMap::new(),
        }
    }

    /// Register an agent
    pub fn register(&self, agent: Box<dyn Agent>) -> Result<()> {
        let id = agent.id();
        let info = agent.info();

        if self.agents.contains_key(&id) {
            return Err(Error::Agent(AgentError::AlreadyExists(id.to_string())));
        }

        self.agent_info.insert(id, info.clone());

        let arc_agent = Arc::new(RwLock::new(agent));
        self.agents.insert(id, arc_agent);

        Ok(())
    }

    /// Get an agent by ID
    pub fn get(&self, id: AgentId) -> Option<Arc<RwLock<Box<dyn Agent>>>> {
        self.agents.get(&id).map(|entry| entry.clone())
    }

    /// Get agent info
    pub fn get_info(&self, id: AgentId) -> Option<AgentInfo> {
        self.agent_info.get(&id).map(|entry| entry.clone())
    }

    /// Apply a function to an agent (preferred over get_mut for async safety)
    pub async fn with_agent<F, T>(&self, id: AgentId, f: F) -> Option<T>
    where
        F: FnOnce(&mut Box<dyn Agent>) -> T,
    {
        if let Some(entry) = self.agents.get(&id) {
            let mut guard = entry.write().await;
            Some(f(&mut guard))
        } else {
            None
        }
    }

    /// List all agents
    pub fn list_agents(&self) -> Vec<AgentInfo> {
        self.agent_info.iter().map(|entry| entry.clone()).collect()
    }

    /// List agents by status
    pub fn list_agents_by_status(&self, status: clawlegion_core::AgentStatus) -> Vec<AgentInfo> {
        self.agent_info
            .iter()
            .filter(|entry| entry.status == status)
            .map(|entry| entry.clone())
            .collect()
    }

    /// List agents by company
    pub fn list_agents_by_company(&self, company_id: clawlegion_core::CompanyId) -> Vec<AgentInfo> {
        self.agent_info
            .iter()
            .filter(|entry| entry.config.company_id == company_id)
            .map(|entry| entry.clone())
            .collect()
    }

    /// Check if an agent exists
    pub fn has_agent(&self, id: AgentId) -> bool {
        self.agents.contains_key(&id)
    }

    /// Unregister an agent
    pub fn unregister(&self, id: AgentId) -> Result<()> {
        self.agents
            .remove(&id)
            .ok_or_else(|| Error::Agent(AgentError::NotFound(id.to_string())))?;

        self.agent_info.remove(&id);

        Ok(())
    }

    /// Update agent info cache
    pub async fn update_agent_info(&self, id: AgentId) -> Result<()> {
        if let Some(agent) = self.get(id) {
            let info = agent.read().await.info();
            self.agent_info.insert(id, info);
            Ok(())
        } else {
            Err(Error::Agent(AgentError::NotFound(id.to_string())))
        }
    }

    /// Get agent count
    pub fn count(&self) -> usize {
        self.agents.len()
    }

    /// Clear all agents (for testing)
    pub fn clear(&self) {
        self.agents.clear();
        self.agent_info.clear();
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared agent registry type
pub type SharedAgentRegistry = Arc<AgentRegistry>;

/// Agent factory for creating agents
pub struct AgentFactory;

impl AgentFactory {
    /// Create an agent based on type definition
    pub fn create_agent(
        config: &clawlegion_core::AgentConfig,
        capabilities: Arc<dyn crate::AgentCapabilities>,
    ) -> Result<Box<dyn Agent>> {
        match config.agent_type {
            clawlegion_core::AgentTypeDef::React => {
                let agent = crate::ReactAgent::new(config.clone(), capabilities);
                Ok(Box::new(agent))
            }
            clawlegion_core::AgentTypeDef::Flow => {
                let agent = crate::FlowAgent::new(config.clone(), capabilities);
                Ok(Box::new(agent))
            }
            clawlegion_core::AgentTypeDef::Normal => {
                let agent = crate::NormalAgent::new(config.clone(), capabilities);
                Ok(Box::new(agent))
            }
            clawlegion_core::AgentTypeDef::Codex => {
                let agent = crate::CodexAgent::new(config.clone(), capabilities);
                Ok(Box::new(agent))
            }
            clawlegion_core::AgentTypeDef::ClaudeCode => {
                let agent = crate::ClaudeCodeAgent::new(config.clone(), capabilities);
                Ok(Box::new(agent))
            }
            clawlegion_core::AgentTypeDef::OpenCode => {
                let agent = crate::OpenCodeAgent::new(config.clone(), capabilities);
                Ok(Box::new(agent))
            }
            clawlegion_core::AgentTypeDef::Custom { .. } => {
                // For custom types, the caller should provide the agent implementation
                Err(Error::Agent(AgentError::ExecutionFailed(
                    "Custom agent types must be created with custom factory".to_string(),
                )))
            }
        }
    }
}
