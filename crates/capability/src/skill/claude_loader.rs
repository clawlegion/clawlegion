//! Claude Skill Loader
//!
//! Scans and loads Claude Skills from ~/.claude/skills/ directory.

use crate::skill::claude_runner::ClaudeSkillRunner;
use clawlegion_core::{CapabilityError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

/// Claude Skill entry
pub struct ClaudeSkillEntry {
    /// Skill name
    pub name: String,

    /// Skill directory
    pub skill_dir: PathBuf,

    /// Pre-loaded runner
    pub runner: Option<ClaudeSkillRunner>,
}

impl std::fmt::Debug for ClaudeSkillEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeSkillEntry")
            .field("name", &self.name)
            .field("skill_dir", &self.skill_dir)
            .field("runner", &self.runner.as_ref().map(|_| "loaded"))
            .finish()
    }
}

/// Claude Skill Loader
pub struct ClaudeSkillLoader {
    /// Claude skills directory (~/.claude/skills/)
    skills_dir: PathBuf,

    /// Loaded skills
    skills: HashMap<String, ClaudeSkillEntry>,
}

impl ClaudeSkillLoader {
    /// Create a new loader with default skills directory
    pub fn new() -> Result<Self> {
        let skills_dir = default_claude_skills_dir();

        Ok(Self {
            skills_dir,
            skills: HashMap::new(),
        })
    }

    /// Create with custom skills directory
    pub fn with_dir(skills_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            skills: HashMap::new(),
        }
    }

    /// Get the skills directory
    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    /// Scan the skills directory and discover available skills
    pub fn scan(&mut self) -> Result<Vec<String>> {
        info!("Scanning Claude skills directory: {:?}", self.skills_dir);

        if !self.skills_dir.exists() {
            info!(
                "Claude skills directory does not exist: {:?}",
                self.skills_dir
            );
            return Ok(vec![]);
        }

        let mut discovered = vec![];

        let entries = std::fs::read_dir(&self.skills_dir).map_err(|e| {
            clawlegion_core::Error::Capability(CapabilityError::NotFound(format!(
                "Failed to read skills directory: {}",
                e
            )))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Check if it has a manifest.json
                let manifest_path = path.join("manifest.json");

                if manifest_path.exists() {
                    match self.load_skill_manifest(&path) {
                        Ok(name) => {
                            info!("Discovered Claude skill: {}", name);
                            discovered.push(name.clone());
                        }
                        Err(e) => {
                            warn!("Failed to load skill manifest from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        info!("Discovered {} Claude skills", discovered.len());
        Ok(discovered)
    }

    /// Load a skill manifest and register the skill
    fn load_skill_manifest(&mut self, skill_dir: &Path) -> Result<String> {
        let manifest = crate::skill::claude_runner::ClaudeManifest::from_directory(skill_dir)?;

        let name = manifest.name.clone();

        let entry = ClaudeSkillEntry {
            name: name.clone(),
            skill_dir: skill_dir.to_path_buf(),
            runner: None, // Lazy load the runner
        };

        self.skills.insert(name.clone(), entry);

        Ok(name)
    }

    /// Get a list of all discovered skills
    pub fn list_skills(&self) -> Vec<&str> {
        self.skills.keys().map(|s| s.as_str()).collect()
    }

    /// Get a skill by name
    pub fn get_skill(&self, name: &str) -> Option<&ClaudeSkillEntry> {
        self.skills.get(name)
    }

    /// Get a mutable skill entry
    pub fn get_skill_mut(&mut self, name: &str) -> Option<&mut ClaudeSkillEntry> {
        self.skills.get_mut(name)
    }

    /// Load a skill runner (lazy loading)
    pub fn load_runner(&mut self, name: &str) -> Result<&ClaudeSkillRunner> {
        let entry = self.skills.get_mut(name).ok_or_else(|| {
            clawlegion_core::Error::Capability(CapabilityError::NotFound(format!(
                "Skill '{}' not found",
                name
            )))
        })?;

        if entry.runner.is_none() {
            let runner = ClaudeSkillRunner::new(entry.skill_dir.clone())?;
            entry.runner = Some(runner);
        }

        Ok(entry.runner.as_ref().unwrap())
    }

    /// Execute a skill by name
    pub async fn execute(
        &mut self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let runner = self.load_runner(name)?;
        runner.execute(input).await
    }

    /// Unload a skill
    pub fn unload(&mut self, name: &str) -> Option<ClaudeSkillEntry> {
        self.skills.remove(name)
    }

    /// Check if a skill is loaded
    pub fn has_skill(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// Get the number of loaded skills
    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }
}

impl Default for ClaudeSkillLoader {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            error!("Failed to create default ClaudeSkillLoader: {}", e);
            Self {
                skills_dir: PathBuf::from("."),
                skills: HashMap::new(),
            }
        })
    }
}

/// Get the default Claude skills directory
fn default_claude_skills_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("skills")
}

/// Scan a directory for Claude Skills
pub fn scan_claude_skills(dir: &Path) -> Result<Vec<String>> {
    let mut loader = ClaudeSkillLoader::with_dir(dir.to_path_buf());
    loader.scan()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skill(dir: &Path, name: &str) {
        let skill_dir = dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();

        let manifest = serde_json::json!({
            "name": name,
            "version": "1.0.0",
            "description": format!("Test skill: {}", name),
            "main": "index.sh",
        });

        fs::write(skill_dir.join("manifest.json"), manifest.to_string()).unwrap();

        // Create a simple shell script
        fs::write(
            skill_dir.join("index.sh"),
            "#!/bin/bash\necho '{\"success\": true}'",
        )
        .unwrap();
    }

    #[test]
    fn test_loader_creation() {
        let loader = ClaudeSkillLoader::new().unwrap();
        assert_eq!(loader.skill_count(), 0);
    }

    #[test]
    fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create test skills
        create_test_skill(temp_dir.path(), "skill-a");
        create_test_skill(temp_dir.path(), "skill-b");
        create_test_skill(temp_dir.path(), "skill-c");

        let mut loader = ClaudeSkillLoader::with_dir(temp_dir.path().to_path_buf());
        let skills = loader.scan().unwrap();

        assert_eq!(skills.len(), 3);
        assert!(skills.iter().any(|s| s == "skill-a"));
        assert!(skills.iter().any(|s| s == "skill-b"));
        assert!(skills.iter().any(|s| s == "skill-c"));
    }

    #[test]
    fn test_list_skills() {
        let temp_dir = TempDir::new().unwrap();
        create_test_skill(temp_dir.path(), "test-skill");

        let mut loader = ClaudeSkillLoader::with_dir(temp_dir.path().to_path_buf());
        loader.scan().unwrap();

        let skills = loader.list_skills();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0], "test-skill");
    }

    #[tokio::test]
    async fn test_execute_skill() {
        let temp_dir = TempDir::new().unwrap();
        create_test_skill(temp_dir.path(), "exec-test");

        let mut loader = ClaudeSkillLoader::with_dir(temp_dir.path().to_path_buf());
        loader.scan().unwrap();

        let result = loader
            .execute("exec-test", serde_json::json!({}))
            .await
            .unwrap();

        assert_eq!(result["success"], true);
    }
}
