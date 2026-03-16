//! Skill Registry - manages skill registration and lookup

use clawlegion_core::{Error, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::skill::{Skill, SkillMetadata};

/// Skill Registry
///
/// Central registry for all skills in the system.
/// Handles registration, discovery, and access control.
pub struct SkillRegistry {
    /// Registered skills
    skills: DashMap<String, Arc<RwLock<Box<dyn Skill>>>>,

    /// Skill metadata cache (for quick lookups without locking)
    metadata_cache: DashMap<String, SkillMetadata>,
}

impl SkillRegistry {
    /// Create a new skill registry
    pub fn new() -> Self {
        Self {
            skills: DashMap::new(),
            metadata_cache: DashMap::new(),
        }
    }

    /// Register a skill
    pub fn register(&self, skill: Box<dyn Skill>) -> Result<()> {
        let metadata = skill.metadata().clone();
        let name = metadata.name.clone();

        // Check for duplicates
        if self.skills.contains_key(&name) {
            return Err(Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Skill '{}' already registered",
                    name
                )),
            ));
        }

        // Store metadata cache
        self.metadata_cache.insert(name.clone(), metadata.clone());

        // Store skill
        let arc_skill = Arc::new(RwLock::new(skill));
        self.skills.insert(name, arc_skill);

        Ok(())
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<Arc<RwLock<Box<dyn Skill>>>> {
        self.skills.get(name).map(|entry| entry.clone())
    }

    /// Get skill metadata
    pub fn get_metadata(&self, name: &str) -> Option<SkillMetadata> {
        self.metadata_cache.get(name).map(|entry| entry.clone())
    }

    /// List all skills
    pub fn list(&self) -> Vec<SkillMetadata> {
        self.metadata_cache
            .iter()
            .map(|entry| entry.clone())
            .collect()
    }

    /// List public skills
    pub fn list_public(&self) -> Vec<SkillMetadata> {
        self.metadata_cache
            .iter()
            .filter(|entry| entry.visibility == crate::skill::Visibility::Public)
            .map(|entry| entry.clone())
            .collect()
    }

    /// List skills by tag
    pub fn list_by_tag(&self, tag: &str) -> Vec<SkillMetadata> {
        self.metadata_cache
            .iter()
            .filter(|entry| entry.tags.iter().any(|t| t == tag))
            .map(|entry| entry.clone())
            .collect()
    }

    /// Check if a skill exists
    pub fn contains(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// Unregister a skill
    pub fn unregister(&self, name: &str) -> Result<()> {
        self.skills.remove(name).ok_or_else(|| {
            Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Skill '{}' not found",
                name
            )))
        })?;

        self.metadata_cache.remove(name);

        Ok(())
    }

    /// Get the number of registered skills
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{types::Visibility, SkillContext, SkillInput, SkillOutput};

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

        async fn execute(&self, _ctx: &SkillContext, _input: SkillInput) -> Result<SkillOutput> {
            Ok(SkillOutput::success("Test completed"))
        }
    }

    #[test]
    fn test_registry_basic_operations() {
        let registry = SkillRegistry::new();

        // Initially empty
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        // Register a skill
        let skill = Box::new(TestSkill);
        assert!(registry.register(skill).is_ok());

        // Check existence
        assert!(registry.contains("test-skill"));
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);

        // Get metadata
        let metadata = registry.get_metadata("test-skill").unwrap();
        assert_eq!(metadata.name, "test-skill");
        assert_eq!(metadata.version, "1.0.0");

        // List skills
        let skills = registry.list();
        assert_eq!(skills.len(), 1);

        // List public skills
        let public_skills = registry.list_public();
        assert_eq!(public_skills.len(), 1);

        // Unregister
        assert!(registry.unregister("test-skill").is_ok());
        assert!(!registry.contains("test-skill"));
    }
}
