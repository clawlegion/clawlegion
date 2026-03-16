//! Skill configuration parsing from TOML files

use clawlegion_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::skill::{
    metadata::SkillMetadata,
    types::{SkillType, Visibility},
};

/// Raw skill configuration from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawSkillConfig {
    /// Skill section
    #[serde(default)]
    pub skill: SkillSection,

    /// Visibility section
    #[serde(default)]
    pub visibility: VisibilitySection,

    /// Type section
    #[serde(default)]
    #[serde(rename = "type")]
    pub skill_type: TypeSection,

    /// Dependencies section
    #[serde(default)]
    pub dependencies: DependenciesSection,

    /// Config section
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,

    /// Lifecycle section
    #[serde(default)]
    pub lifecycle: LifecycleSection,

    /// Triggers section
    #[serde(default)]
    pub triggers: TriggersSection,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillSection {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VisibilitySection {
    #[serde(default)]
    pub r#type: String, // "public" or "private"
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeSection {
    #[serde(default)]
    pub mode: String, // "llm", "code", or "hybrid"
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependenciesSection {
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub mcps: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecycleSection {
    #[serde(default)]
    pub init_timeout_ms: Option<u64>,
    #[serde(default)]
    pub execute_timeout_ms: Option<u64>,
    #[serde(default)]
    pub shutdown_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggersSection {
    #[serde(default)]
    pub file_patterns: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
}

impl RawSkillConfig {
    /// Parse a TOML file into a RawSkillConfig
    pub fn from_toml(toml_content: &str) -> Result<Self> {
        let config: RawSkillConfig = toml::from_str(toml_content).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to parse TOML: {}",
                e
            )))
        })?;
        Ok(config)
    }

    /// Parse a TOML file from a path
    pub fn from_path(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to read config file {}: {}", path.display(), e),
                ))
            })?;
        Self::from_toml(&content)
    }

    /// Convert to SkillMetadata
    pub fn to_metadata(&self, config_path: Option<String>) -> SkillMetadata {
        let visibility = match self.visibility.r#type.to_lowercase().as_str() {
            "private" => Visibility::Private,
            _ => Visibility::Public,
        };

        let skill_type = match self.skill_type.mode.to_lowercase().as_str() {
            "llm" => SkillType::Llm,
            "hybrid" => SkillType::Hybrid,
            _ => SkillType::Code,
        };

        let mut metadata = SkillMetadata::new(
            self.skill
                .name
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            self.skill
                .version
                .clone()
                .unwrap_or_else(|| "1.0.0".to_string()),
            self.skill.description.clone().unwrap_or_default(),
        )
        .with_visibility(visibility)
        .with_skill_type(skill_type);

        if let Some(author) = &self.skill.author {
            metadata = metadata.with_author(author.clone());
        }

        metadata.required_tools = self.dependencies.tools.clone();
        metadata.required_mcps = self.dependencies.mcps.clone();
        metadata.dependencies = self.dependencies.skills.clone();
        metadata.config_path = config_path;

        metadata
    }
}

/// Parse a skill directory and return its metadata
pub fn parse_skill_directory(
    dir_path: &Path,
) -> Result<(SkillMetadata, HashMap<String, serde_json::Value>)> {
    let config_path = dir_path.join("skill.toml");

    if !config_path.exists() {
        return Err(clawlegion_core::Error::Capability(
            clawlegion_core::CapabilityError::NotFound(format!(
                "No skill.toml found in {}",
                dir_path.display()
            )),
        ));
    }

    let config = RawSkillConfig::from_path(&config_path)?;
    let metadata = config.to_metadata(Some(config_path.to_string_lossy().to_string()));
    let skill_config = config.config;

    Ok((metadata, skill_config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[skill]
name = "test-skill"
version = "1.0.0"
description = "A test skill"
"#;

        let config = RawSkillConfig::from_toml(toml).unwrap();
        assert_eq!(config.skill.name, Some("test-skill".to_string()));
        assert_eq!(config.skill.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[skill]
name = "code-reviewer"
version = "2.0.0"
description = "Code review skill"
author = "Test Author"

[visibility]
type = "private"

[type]
mode = "hybrid"

[dependencies]
skills = ["code-analyzer"]
tools = ["file-reader"]
mcps = ["github-api"]

[config]
max_file_size = 10000

[lifecycle]
init_timeout_ms = 5000
execute_timeout_ms = 60000

[triggers]
file_patterns = ["*.rs", "*.ts"]
events = ["file.save"]
"#;

        let config = RawSkillConfig::from_toml(toml).unwrap();
        assert_eq!(config.skill.name, Some("code-reviewer".to_string()));
        assert_eq!(config.visibility.r#type, "private");
        assert_eq!(config.skill_type.mode, "hybrid");
        assert_eq!(config.dependencies.skills, vec!["code-analyzer"]);
        assert_eq!(config.dependencies.tools, vec!["file-reader"]);
        assert_eq!(config.dependencies.mcps, vec!["github-api"]);
    }

    #[test]
    fn test_convert_to_metadata() {
        let toml = r#"
[skill]
name = "test-skill"
version = "1.0.0"
description = "A test skill"
author = "Test"

[visibility]
type = "public"

[type]
mode = "llm"

[dependencies]
skills = ["dep1"]
tools = ["tool1"]
mcps = ["mcp1"]
"#;

        let config = RawSkillConfig::from_toml(toml).unwrap();
        let metadata = config.to_metadata(Some("/path/to/skill.toml".to_string()));

        assert_eq!(metadata.name, "test-skill");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, "A test skill");
        assert_eq!(metadata.author, Some("Test".to_string()));
        assert_eq!(metadata.visibility, Visibility::Public);
        assert_eq!(metadata.skill_type, SkillType::Llm);
        assert_eq!(metadata.dependencies, vec!["dep1"]);
        assert_eq!(metadata.required_tools, vec!["tool1"]);
        assert_eq!(metadata.required_mcps, vec!["mcp1"]);
    }
}
