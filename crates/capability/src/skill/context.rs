//! Skill context

use clawlegion_core::AgentId;
use std::collections::HashMap;

/// Skill context provided during execution
#[derive(Debug, Clone)]
pub struct SkillContext {
    /// Agent ID executing this skill
    pub agent_id: AgentId,

    /// Shared state across skill invocations
    pub state: HashMap<String, serde_json::Value>,

    /// Skill configuration
    pub config: HashMap<String, serde_json::Value>,
}

impl SkillContext {
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            state: HashMap::new(),
            config: HashMap::new(),
        }
    }

    pub fn with_config(mut self, config: HashMap<String, serde_json::Value>) -> Self {
        self.config = config;
        self
    }

    pub fn with_state(mut self, state: HashMap<String, serde_json::Value>) -> Self {
        self.state = state;
        self
    }

    /// Get a config value
    pub fn get_config(&self, key: &str) -> Option<&serde_json::Value> {
        self.config.get(key)
    }

    /// Get a config value with default
    pub fn get_config_or<T: serde::de::DeserializeOwned>(&self, key: &str, default: T) -> T {
        self.config
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(default)
    }

    /// Get a state value
    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Set a state value
    pub fn set_state(&mut self, key: String, value: serde_json::Value) {
        self.state.insert(key, value);
    }

    /// Remove a state value
    pub fn remove_state(&mut self, key: &str) -> Option<serde_json::Value> {
        self.state.remove(key)
    }

    /// Get agent ID
    pub fn agent_id(&self) -> &AgentId {
        &self.agent_id
    }
}
