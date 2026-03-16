//! Skill Manager - manages skill lifecycle and execution

use clawlegion_core::{AgentId, Result};
use std::sync::Arc;
use tracing::{info, warn};

use crate::skill::{
    context::SkillContext,
    metadata::{SkillInput, SkillMetadata, SkillOutput},
    registry::SkillRegistry,
    trait_def::{Skill, SkillInstance},
};

/// Skill Manager
///
/// Central manager for all skills in the system.
/// Handles lifecycle management, execution, and coordination.
pub struct SkillManager {
    /// Skill registry
    registry: Arc<SkillRegistry>,

    /// Active skill instances per agent
    agent_instances: Arc<dashmap::DashMap<AgentId, dashmap::DashMap<String, Arc<SkillInstance>>>>,
}

impl SkillManager {
    /// Create a new skill manager
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self {
            registry,
            agent_instances: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Create a new skill manager with a new registry
    pub fn with_new_registry() -> Self {
        Self::new(Arc::new(SkillRegistry::new()))
    }

    /// Get the skill registry
    pub fn registry(&self) -> &SkillRegistry {
        &self.registry
    }

    /// Get the skill registry (Arc)
    pub fn registry_arc(&self) -> Arc<SkillRegistry> {
        self.registry.clone()
    }

    /// Load a skill for an agent
    pub fn load_skill(&self, agent_id: AgentId, skill_name: &str) -> Result<bool> {
        // Get skill from registry
        let skill_arc = match self.registry.get(skill_name) {
            Some(skill) => skill,
            None => return Ok(false),
        };

        // Read the skill and create a wrapper
        let skill_guard = skill_arc.read();
        let metadata = skill_guard.metadata().clone();
        drop(skill_guard);

        // Create a wrapper skill that just holds metadata
        let skill_box = Box::new(MetadataOnlySkill::new(metadata));

        // Create instance
        let instance = Arc::new(SkillInstance::new(skill_box, agent_id));

        // Store in agent instances
        let agent_map = self.agent_instances.entry(agent_id).or_default();
        agent_map.insert(skill_name.to_string(), instance);

        info!("Loaded skill '{}' for agent {:?}", skill_name, agent_id);
        Ok(true)
    }

    /// Load a skill for an agent with a custom skill instance
    pub fn load_skill_instance(
        &self,
        agent_id: AgentId,
        skill_name: &str,
        skill: Box<dyn Skill>,
    ) -> Result<()> {
        let instance = Arc::new(SkillInstance::new(skill, agent_id));

        let agent_map = self.agent_instances.entry(agent_id).or_default();
        agent_map.insert(skill_name.to_string(), instance);

        info!(
            "Loaded skill instance '{}' for agent {:?}",
            skill_name, agent_id
        );
        Ok(())
    }

    /// Unload a skill from an agent
    pub fn unload_skill(&self, agent_id: AgentId, skill_name: &str) -> bool {
        if let Some(agent_map) = self.agent_instances.get(&agent_id) {
            if let Some((_, instance)) = agent_map.remove(skill_name) {
                // Shutdown the skill asynchronously (fire and forget)
                let skill_name_owned = skill_name.to_string();
                tokio::spawn(async move {
                    if let Err(e) = instance.shutdown().await {
                        warn!("Error shutting down skill '{}': {}", skill_name_owned, e);
                    }
                });
                info!("Unloaded skill '{}' from agent {:?}", skill_name, agent_id);
                return true;
            }
        }
        false
    }

    /// Get all skills loaded for an agent
    pub fn get_agent_skills(&self, agent_id: AgentId) -> Vec<SkillMetadata> {
        if let Some(agent_map) = self.agent_instances.get(&agent_id) {
            agent_map
                .iter()
                .map(|entry| entry.value().metadata().clone())
                .collect()
        } else {
            vec![]
        }
    }

    /// Check if an agent has a specific skill
    pub fn has_skill(&self, agent_id: AgentId, skill_name: &str) -> bool {
        if let Some(agent_map) = self.agent_instances.get(&agent_id) {
            agent_map.contains_key(skill_name)
        } else {
            false
        }
    }

    /// Execute a skill for an agent
    pub async fn execute_skill(
        &self,
        agent_id: AgentId,
        skill_name: &str,
        input: SkillInput,
    ) -> Result<SkillOutput> {
        let instance = self
            .get_skill_instance(agent_id, skill_name)
            .ok_or_else(|| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Skill '{}' not loaded for agent {:?}", skill_name, agent_id),
                ))
            })?;

        instance.execute(input).await
    }

    /// Get a skill instance for an agent
    fn get_skill_instance(
        &self,
        agent_id: AgentId,
        skill_name: &str,
    ) -> Option<Arc<SkillInstance>> {
        self.agent_instances
            .get(&agent_id)
            .and_then(|agent_map| agent_map.get(skill_name).map(|e| e.clone()))
    }

    /// Shutdown all skills for an agent
    pub async fn shutdown_agent_skills(&self, agent_id: AgentId) {
        if let Some((_, agent_map)) = self.agent_instances.remove(&agent_id) {
            for (_, instance) in agent_map.into_iter() {
                if let Err(e) = instance.shutdown().await {
                    warn!("Error shutting down skill: {}", e);
                }
            }
            info!("Shutdown all skills for agent {:?}", agent_id);
        }
    }

    /// Get the number of active skill instances
    pub fn active_instance_count(&self) -> usize {
        let mut count = 0;
        for entry in self.agent_instances.iter() {
            count += entry.value().len();
        }
        count
    }
}

/// Helper struct that holds only metadata
struct MetadataOnlySkill {
    metadata: SkillMetadata,
}

impl MetadataOnlySkill {
    fn new(metadata: SkillMetadata) -> Self {
        Self { metadata }
    }
}

#[async_trait::async_trait]
impl Skill for MetadataOnlySkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    async fn execute(&self, _ctx: &SkillContext, _input: SkillInput) -> Result<SkillOutput> {
        Err(clawlegion_core::Error::Capability(
            clawlegion_core::CapabilityError::NotFound(
                "MetadataOnlySkill cannot be executed - use load_skill_instance instead".into(),
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{types::Visibility, SkillContext};

    struct TestSkill;

    #[async_trait::async_trait]
    impl Skill for TestSkill {
        fn metadata(&self) -> &SkillMetadata {
            static METADATA: std::sync::OnceLock<SkillMetadata> = std::sync::OnceLock::new();
            METADATA.get_or_init(|| {
                SkillMetadata::new("test-skill", "1.0.0", "A test skill")
                    .with_visibility(Visibility::Public)
            })
        }

        async fn execute(&self, _ctx: &SkillContext, input: SkillInput) -> Result<SkillOutput> {
            Ok(SkillOutput::success(format!(
                "Executed with input: {:?}",
                input.text
            )))
        }
    }

    #[tokio::test]
    async fn test_manager_basic_operations() {
        let manager = SkillManager::with_new_registry();
        let agent_id = AgentId::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        // Initially no skills loaded
        assert!(manager.get_agent_skills(agent_id).is_empty());
        assert!(!manager.has_skill(agent_id, "test-skill"));

        // Register a skill
        let skill = Box::new(TestSkill);
        manager.registry().register(skill).unwrap();

        // Load skill for agent
        assert!(manager.load_skill(agent_id, "test-skill").unwrap());
        assert!(manager.has_skill(agent_id, "test-skill"));

        // Get skills for agent
        let skills = manager.get_agent_skills(agent_id);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");

        // Unload skill
        assert!(manager.unload_skill(agent_id, "test-skill"));
        assert!(!manager.has_skill(agent_id, "test-skill"));
    }
}
