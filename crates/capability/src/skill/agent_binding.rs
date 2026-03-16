//! Agent-Skill Binding - manages skill bindings for specific agents
//!
//! This module provides functionality for:
//! - Binding skills to specific agents
//! - Configuring per-agent skill settings
//! - Checking agent skill access permissions
//! - Managing agent-specific skill instances

use clawlegion_core::{AgentId, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::skill::metadata::SkillMetadata;

/// Agent-Skill binding configuration
#[derive(Clone)]
pub struct AgentSkillBinding {
    /// Agent identifier
    pub agent_id: AgentId,
    /// Bound skill names
    pub bound_skills: Vec<String>,
    /// Per-skill configuration overrides
    pub skill_configs: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl AgentSkillBinding {
    /// Create a new agent-skill binding
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            bound_skills: vec![],
            skill_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a bound skill
    pub fn with_skill(mut self, skill_name: impl Into<String>) -> Self {
        self.bound_skills.push(skill_name.into());
        self
    }

    /// Add a skill with configuration
    pub fn with_skill_config(
        mut self,
        skill_name: impl Into<String>,
        config: serde_json::Value,
    ) -> Self {
        let name = skill_name.into();
        self.bound_skills.push(name.clone());
        self.skill_configs.write().insert(name, config);
        self
    }

    /// Check if a skill is bound to this agent
    pub fn has_skill(&self, skill_name: &str) -> bool {
        self.bound_skills.iter().any(|s| s == skill_name)
    }

    /// Get skill configuration
    pub fn get_skill_config(&self, skill_name: &str) -> Option<serde_json::Value> {
        self.skill_configs.read().get(skill_name).cloned()
    }

    /// Remove a bound skill
    pub fn remove_skill(&mut self, skill_name: &str) {
        self.bound_skills.retain(|s| s != skill_name);
        self.skill_configs.write().remove(skill_name);
    }
}

impl std::fmt::Debug for AgentSkillBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentSkillBinding")
            .field("agent_id", &self.agent_id)
            .field("bound_skills", &self.bound_skills)
            .finish()
    }
}

/// Agent Skill Manager - manages skill bindings for all agents
pub struct AgentSkillManager {
    /// Agent-skill bindings
    bindings: DashMap<AgentId, AgentSkillBinding>,
    /// Global skill registry (for metadata lookup)
    skill_registry: Option<Arc<dyn SkillRegistryTrait>>,
}

/// Trait for skill registry (to avoid circular dependency)
pub trait SkillRegistryTrait: Send + Sync {
    fn get_metadata(&self, name: &str) -> Option<SkillMetadata>;
    fn contains(&self, name: &str) -> bool;
}

impl AgentSkillManager {
    /// Create a new agent skill manager
    pub fn new() -> Self {
        Self {
            bindings: DashMap::new(),
            skill_registry: None,
        }
    }

    /// Create with a skill registry
    pub fn with_registry(registry: Arc<dyn SkillRegistryTrait>) -> Self {
        Self {
            bindings: DashMap::new(),
            skill_registry: Some(registry),
        }
    }

    /// Bind a skill to an agent
    pub fn bind_skill(
        &self,
        agent_id: &AgentId,
        skill_name: &str,
        config: Option<serde_json::Value>,
    ) -> Result<()> {
        info!("Binding skill '{}' to agent {:?}", skill_name, agent_id);

        // Validate skill exists if registry is available
        if let Some(ref registry) = self.skill_registry {
            if !registry.contains(skill_name) {
                return Err(clawlegion_core::Error::Capability(
                    clawlegion_core::CapabilityError::NotFound(format!(
                        "Skill '{}' not found in registry",
                        skill_name
                    )),
                ));
            }
        }

        let mut binding = self
            .bindings
            .entry(*agent_id)
            .or_insert_with(|| AgentSkillBinding::new(*agent_id));

        if !binding.has_skill(skill_name) {
            binding.bound_skills.push(skill_name.to_string());
        }

        if let Some(cfg) = config {
            binding
                .skill_configs
                .write()
                .insert(skill_name.to_string(), cfg);
        }

        Ok(())
    }

    /// Unbind a skill from an agent
    pub fn unbind_skill(&self, agent_id: &AgentId, skill_name: &str) -> Result<()> {
        info!("Unbinding skill '{}' from agent {:?}", skill_name, agent_id);

        let binding = self.bindings.get(agent_id).ok_or_else(|| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "No bindings found for agent {:?}",
                agent_id
            )))
        })?;

        // Clone the skill list to avoid holding the lock
        let mut bound_skills = binding.bound_skills.clone();
        bound_skills.retain(|s| s != skill_name);

        // Update the binding
        drop(binding);
        if let Some(mut b) = self.bindings.get_mut(agent_id) {
            b.bound_skills = bound_skills;
            b.skill_configs.write().remove(skill_name);
        }

        Ok(())
    }

    /// Get all skills bound to an agent
    pub fn get_agent_skills(&self, agent_id: &AgentId) -> Vec<SkillMetadata> {
        let binding = self.bindings.get(agent_id);
        let binding = match binding {
            Some(b) => b,
            None => return vec![],
        };

        let mut skills = Vec::new();

        for skill_name in &binding.bound_skills {
            if let Some(ref registry) = self.skill_registry {
                if let Some(metadata) = registry.get_metadata(skill_name) {
                    skills.push(metadata);
                }
            } else {
                // Without registry, return just names
                skills.push(SkillMetadata::new(
                    skill_name,
                    "unknown",
                    "Skill bound to agent",
                ));
            }
        }

        skills
    }

    /// Get skill configuration for an agent
    pub fn get_agent_skill_config(
        &self,
        agent_id: &AgentId,
        skill_name: &str,
    ) -> Option<serde_json::Value> {
        self.bindings
            .get(agent_id)
            .and_then(|b| b.get_skill_config(skill_name))
    }

    /// Check if an agent has access to a skill
    pub fn has_access(&self, agent_id: &AgentId, skill_name: &str) -> bool {
        self.bindings
            .get(agent_id)
            .map(|b| b.has_skill(skill_name))
            .unwrap_or(false)
    }

    /// Get all agents that have a skill bound
    pub fn get_skill_agents(&self, skill_name: &str) -> Vec<AgentId> {
        self.bindings
            .iter()
            .filter(|entry| entry.value().has_skill(skill_name))
            .map(|entry| *entry.key())
            .collect()
    }

    /// Get all agent-skill bindings
    pub fn list_all_bindings(&self) -> Vec<AgentSkillBinding> {
        self.bindings.iter().map(|b| b.clone()).collect()
    }

    /// Remove all bindings for an agent
    pub fn remove_agent(&self, agent_id: &AgentId) -> Option<AgentSkillBinding> {
        self.bindings.remove(agent_id).map(|(_, b)| b)
    }

    /// Get the number of agents with bindings
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Check if there are any bindings
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Get binding statistics
    pub fn get_stats(&self) -> AgentSkillStats {
        let mut total_bindings = 0;
        let mut unique_skills = std::collections::HashSet::new();

        for entry in self.bindings.iter() {
            let binding = entry.value();
            total_bindings += binding.bound_skills.len();
            for skill in &binding.bound_skills {
                unique_skills.insert(skill.clone());
            }
        }

        AgentSkillStats {
            total_agents: self.bindings.len(),
            total_bindings,
            unique_skills: unique_skills.len(),
        }
    }
}

impl Default for AgentSkillManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent-Skill binding statistics
#[derive(Debug, Clone)]
pub struct AgentSkillStats {
    /// Number of agents with bindings
    pub total_agents: usize,
    /// Total number of skill bindings
    pub total_bindings: usize,
    /// Number of unique skills bound
    pub unique_skills: usize,
}

/// Builder for agent-skill bindings
pub struct AgentSkillBindingBuilder {
    agent_id: AgentId,
    skills: Vec<String>,
    configs: Vec<(String, serde_json::Value)>,
}

impl AgentSkillBindingBuilder {
    /// Create a new builder
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            skills: vec![],
            configs: vec![],
        }
    }

    /// Add a skill
    pub fn skill(mut self, skill_name: impl Into<String>) -> Self {
        self.skills.push(skill_name.into());
        self
    }

    /// Add a skill with configuration
    pub fn skill_with_config(
        mut self,
        skill_name: impl Into<String>,
        config: serde_json::Value,
    ) -> Self {
        let name = skill_name.into();
        self.skills.push(name.clone());
        self.configs.push((name, config));
        self
    }

    /// Build and apply to manager
    pub fn apply(self, manager: &AgentSkillManager) -> Result<()> {
        for skill in &self.skills {
            let config = self
                .configs
                .iter()
                .find(|(name, _)| name == skill)
                .map(|(_, c)| c.clone());
            manager.bind_skill(&self.agent_id, skill, config)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRegistry {
        skills: DashMap<String, SkillMetadata>,
    }

    impl MockRegistry {
        fn new() -> Self {
            let registry = Self {
                skills: DashMap::new(),
            };
            registry.skills.insert(
                "test-skill".to_string(),
                SkillMetadata::new("test-skill", "1.0.0", "A test skill"),
            );
            registry
        }
    }

    impl SkillRegistryTrait for MockRegistry {
        fn get_metadata(&self, name: &str) -> Option<SkillMetadata> {
            self.skills.get(name).map(|s| s.clone())
        }

        fn contains(&self, name: &str) -> bool {
            self.skills.contains_key(name)
        }
    }

    #[test]
    fn test_agent_skill_binding_creation() {
        let agent_id = AgentId::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let binding = AgentSkillBinding::new(agent_id)
            .with_skill("skill1")
            .with_skill_config("skill2", serde_json::json!({"key": "value"}));

        assert_eq!(
            binding.agent_id.to_string(),
            "00000000-0000-0000-0000-000000000001"
        );
        assert!(binding.has_skill("skill1"));
        assert!(binding.has_skill("skill2"));
        assert!(!binding.has_skill("skill3"));
        assert!(binding.get_skill_config("skill2").is_some());
    }

    #[test]
    fn test_agent_skill_binding_clone() {
        let agent_id = AgentId::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let binding1 = AgentSkillBinding::new(agent_id).with_skill("skill1");
        let binding2 = binding1.clone();

        assert_eq!(
            binding2.agent_id.to_string(),
            "00000000-0000-0000-0000-000000000001"
        );
        assert!(binding2.has_skill("skill1"));
    }

    #[test]
    fn test_agent_skill_manager() {
        let registry = Arc::new(MockRegistry::new());
        let manager = AgentSkillManager::with_registry(registry);

        let agent_id = AgentId::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        // Bind a skill
        manager.bind_skill(&agent_id, "test-skill", None).unwrap();

        // Check access
        assert!(manager.has_access(&agent_id, "test-skill"));
        assert!(!manager.has_access(&agent_id, "other-skill"));

        // Get agent skills
        let skills = manager.get_agent_skills(&agent_id);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");

        // Unbind
        manager.unbind_skill(&agent_id, "test-skill").unwrap();
        assert!(!manager.has_access(&agent_id, "test-skill"));
    }

    #[test]
    fn test_agent_skill_stats() {
        let manager = AgentSkillManager::new();
        let agent1 = AgentId::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let agent2 = AgentId::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        manager.bind_skill(&agent1, "skill1", None).unwrap();
        manager.bind_skill(&agent1, "skill2", None).unwrap();
        manager.bind_skill(&agent2, "skill1", None).unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_agents, 2);
        assert_eq!(stats.total_bindings, 3);
        assert_eq!(stats.unique_skills, 2);
    }

    #[test]
    fn test_builder_pattern() {
        let manager = AgentSkillManager::new();
        let agent_id = AgentId::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        AgentSkillBindingBuilder::new(agent_id)
            .skill("skill1")
            .skill_with_config("skill2", serde_json::json!({"key": "value"}))
            .apply(&manager)
            .unwrap();

        assert!(manager.has_access(&agent_id, "skill1"));
        assert!(manager.has_access(&agent_id, "skill2"));
        assert!(manager
            .get_agent_skill_config(&agent_id, "skill2")
            .is_some());
    }
}
