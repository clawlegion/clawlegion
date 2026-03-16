//! Skill Loader - loads skills from directory or configuration

use clawlegion_core::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

use crate::skill::{
    config::parse_skill_directory, manager::SkillManager, metadata::SkillMetadata,
    registry::SkillRegistry, trait_def::Skill,
};

/// Load strategy
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoadStrategy {
    /// Only scan from directory
    DirectoryScan,
    /// Only from configuration file
    ConfigOnly,
    /// Directory scan + configuration override (higher priority)
    #[default]
    Hybrid,
}

/// Loader configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderConfig {
    /// Load strategy
    #[serde(default)]
    pub strategy: LoadStrategy,

    /// Skills directory
    pub skills_dir: Option<PathBuf>,

    /// Additional config file paths
    #[serde(default)]
    pub config_files: Vec<PathBuf>,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            strategy: LoadStrategy::Hybrid,
            skills_dir: default_skills_dir(),
            config_files: vec![],
        }
    }
}

fn default_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".clawlegion").join("skills"))
}

/// Skill Loader
pub struct SkillLoader {
    config: LoaderConfig,
    registry: Arc<SkillRegistry>,
    manager: Arc<SkillManager>,
}

impl SkillLoader {
    /// Create a new skill loader
    pub fn new(
        config: LoaderConfig,
        registry: Arc<SkillRegistry>,
        manager: Arc<SkillManager>,
    ) -> Self {
        Self {
            config,
            registry,
            manager,
        }
    }

    /// Create a new loader with default config
    pub fn with_defaults() -> (Self, Arc<SkillRegistry>, Arc<SkillManager>) {
        let registry = Arc::new(SkillRegistry::new());
        let manager = Arc::new(SkillManager::new(registry.clone()));
        let loader = Self::new(LoaderConfig::default(), registry.clone(), manager.clone());
        (loader, registry, manager)
    }

    /// Load all skills from configured sources
    pub fn load_all(&self) -> Result<Vec<SkillMetadata>> {
        let mut loaded = Vec::new();

        // Load from directory scan
        if self.config.strategy == LoadStrategy::DirectoryScan
            || self.config.strategy == LoadStrategy::Hybrid
        {
            if let Some(dir) = &self.config.skills_dir {
                let skills = self.load_from_directory(dir)?;
                loaded.extend(skills);
            }
        }

        // Load from config files
        if self.config.strategy == LoadStrategy::ConfigOnly
            || self.config.strategy == LoadStrategy::Hybrid
        {
            for config_path in &self.config.config_files {
                let skills = self.load_from_config_file(config_path)?;
                loaded.extend(skills);
            }
        }

        info!("Loaded {} skills", loaded.len());
        Ok(loaded)
    }

    /// Load skills from a directory
    pub fn load_from_directory(&self, dir: &Path) -> Result<Vec<SkillMetadata>> {
        let mut loaded = Vec::new();

        if !dir.exists() {
            info!("Skills directory does not exist: {}", dir.display());
            return Ok(loaded);
        }

        info!("Scanning skills directory: {}", dir.display());

        // Scan subdirectories for skill.toml files
        for entry in
            std::fs::read_dir(dir).map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to read directory {}: {}", dir.display(), e),
                ))
            })?
        {
            let entry = entry.map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to read directory entry: {}", e),
                ))
            })?;

            let path = entry.path();
            if path.is_dir() {
                match self.load_single_skill(&path) {
                    Ok(metadata) => {
                        info!("Loaded skill: {} v{}", metadata.name, metadata.version);
                        loaded.push(metadata);
                    }
                    Err(e) => {
                        warn!("Failed to load skill from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Load a single skill from a directory
    pub fn load_single_skill(&self, skill_dir: &Path) -> Result<SkillMetadata> {
        let (metadata, _config) = parse_skill_directory(skill_dir)?;

        // For now, we just parse metadata
        // In a full implementation, we would also load the skill implementation
        // from dynamic plugins or prompt files

        // Register the metadata
        // Note: This is a placeholder - actual skill loading requires the skill implementation
        // which would come from a dynamic plugin or built-in code

        Ok(metadata)
    }

    /// Load skills from a configuration file
    pub fn load_from_config_file(&self, config_path: &Path) -> Result<Vec<SkillMetadata>> {
        let content = std::fs::read_to_string(config_path).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to read config file {}: {}",
                config_path.display(),
                e
            )))
        })?;

        let config: SkillLoaderConfigFile = toml::from_str(&content).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to parse config file: {}",
                e
            )))
        })?;

        let mut loaded = Vec::new();

        for skill_entry in config.skills {
            // Load skill from the specified path
            let skill_path = PathBuf::from(&skill_entry.path);
            if skill_path.exists() {
                match self.load_single_skill(&skill_path) {
                    Ok(metadata) => {
                        info!(
                            "Loaded skill from config: {} v{}",
                            metadata.name, metadata.version
                        );
                        loaded.push(metadata);
                    }
                    Err(e) => {
                        warn!("Failed to load skill from {:?}: {}", skill_path, e);
                    }
                }
            } else {
                warn!("Skill path does not exist: {}", skill_entry.path);
            }
        }

        Ok(loaded)
    }

    /// Register a skill directly
    pub fn register_skill(&self, skill: Box<dyn Skill>) -> Result<()> {
        self.registry.register(skill)
    }

    /// Get the skill manager
    pub fn manager(&self) -> &Arc<SkillManager> {
        &self.manager
    }

    /// Get the skill registry
    pub fn registry(&self) -> &Arc<SkillRegistry> {
        &self.registry
    }

    /// Get the default skills directory
    pub fn skills_dir(&self) -> Option<&PathBuf> {
        self.config.skills_dir.as_ref()
    }
}

/// Configuration file format for skill loader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLoaderConfigFile {
    #[serde(default)]
    pub skills: Vec<SkillEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_skill_dir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        std::fs::create_dir(&skill_dir).unwrap();

        let toml_content = r#"
[skill]
name = "test-skill"
version = "1.0.0"
description = "A test skill"

[visibility]
type = "public"

[type]
mode = "code"
"#;

        let mut toml_file = std::fs::File::create(skill_dir.join("skill.toml")).unwrap();
        toml_file.write_all(toml_content.as_bytes()).unwrap();

        (temp_dir, skill_dir)
    }

    #[test]
    fn test_loader_creation() {
        let (_loader, registry, manager) = SkillLoader::with_defaults();
        assert_eq!(registry.len(), 0);
        assert_eq!(manager.active_instance_count(), 0);
    }

    #[test]
    fn test_load_from_directory() {
        let (_temp_dir, skill_dir) = create_test_skill_dir();

        let (_loader, registry, _manager) = SkillLoader::with_defaults();
        let loader = SkillLoader::new(
            LoaderConfig {
                strategy: LoadStrategy::DirectoryScan,
                skills_dir: Some(skill_dir.parent().unwrap().to_path_buf()),
                config_files: vec![],
            },
            registry.clone(),
            Arc::new(SkillManager::new(registry.clone())),
        );

        let skills = loader
            .load_from_directory(skill_dir.parent().unwrap())
            .unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");
    }
}
