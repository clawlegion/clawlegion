//! Organization configuration - TOML-based org config

use crate::{AgentPermissions, Company, OrgAgent};
use clawlegion_core::{AgentTypeDef, CompanyId, Error, OrgError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Organization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgConfig {
    /// Company configuration
    pub company: CompanyConfig,

    /// Agent configurations
    #[serde(default)]
    pub agents: Vec<AgentConfigEntry>,
}

/// Company configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConfig {
    /// Company name
    pub name: String,

    /// Company description
    pub description: Option<String>,

    /// Issue prefix
    pub issue_prefix: String,

    /// Require approval for new agents
    #[serde(default = "default_true")]
    pub require_approval_for_new_agents: bool,

    /// Brand color
    pub brand_color: Option<String>,
}

/// Agent configuration entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigEntry {
    /// Agent ID (optional, will be generated if not provided)
    pub id: Option<String>,

    /// Agent name
    pub name: String,

    /// Agent role
    pub role: String,

    /// Agent title
    pub title: String,

    /// Agent icon
    pub icon: Option<String>,

    /// Manager ID (who this agent reports to)
    pub reports_to: Option<String>,

    /// Capabilities description
    pub capabilities: Option<String>,

    /// Agent type
    #[serde(default = "default_agent_type")]
    pub agent_type: AgentTypeDef,

    /// Adapter type
    #[serde(default = "default_adapter_type")]
    pub adapter_type: String,

    /// Adapter configuration
    #[serde(default)]
    pub adapter_config: HashMap<String, serde_json::Value>,

    /// Runtime configuration
    #[serde(default)]
    pub runtime_config: HashMap<String, serde_json::Value>,

    /// Permissions
    #[serde(default)]
    pub permissions: AgentPermissionsConfig,

    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Agent permissions configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentPermissionsConfig {
    #[serde(default)]
    pub can_hire: bool,

    #[serde(default)]
    pub can_fire: bool,

    #[serde(default)]
    pub can_assign_tasks: bool,

    #[serde(default)]
    pub can_approve_tasks: bool,

    #[serde(default)]
    pub can_access_company_data: bool,

    #[serde(default)]
    pub can_create_goals: bool,
}

impl From<AgentPermissionsConfig> for AgentPermissions {
    fn from(config: AgentPermissionsConfig) -> Self {
        Self {
            can_hire: config.can_hire,
            can_fire: config.can_fire,
            can_assign_tasks: config.can_assign_tasks,
            can_approve_tasks: config.can_approve_tasks,
            can_access_company_data: config.can_access_company_data,
            can_create_goals: config.can_create_goals,
        }
    }
}

impl OrgConfig {
    /// Load organization configuration from a TOML file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::Org(OrgError::CompanyNotFound(format!(
                "Failed to read org config file: {}",
                e
            )))
        })?;

        let config: OrgConfig = toml::from_str(&content).map_err(|e| {
            Error::Org(OrgError::InvalidStructure(format!(
                "Failed to parse org config: {}",
                e
            )))
        })?;

        Ok(config)
    }

    /// Save organization configuration to a TOML file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            Error::Org(OrgError::InvalidStructure(format!(
                "Failed to serialize org config: {}",
                e
            )))
        })?;

        std::fs::write(path, content).map_err(|e| {
            Error::Org(OrgError::CompanyNotFound(format!(
                "Failed to write org config file: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Convert to Company
    pub fn to_company(&self) -> Company {
        let mut company = Company::new(&self.company.name, &self.company.issue_prefix);

        if let Some(ref desc) = self.company.description {
            company.description = Some(desc.clone());
        }

        company.require_approval_for_new_agents = self.company.require_approval_for_new_agents;
        company.brand_color = self.company.brand_color.clone();

        company
    }

    /// Convert to OrgAgents
    pub fn to_agents(&self, company_id: CompanyId) -> Result<Vec<OrgAgent>> {
        let mut agents = vec![];
        let mut id_map: HashMap<String, clawlegion_core::AgentId> = HashMap::new();

        // First pass: create all agents
        for agent_config in &self.agents {
            let mut agent = OrgAgent::new(
                company_id,
                &agent_config.name,
                &agent_config.role,
                &agent_config.title,
            );

            // Set or generate ID
            let agent_id = if let Some(ref id_str) = agent_config.id {
                // Use provided ID (deterministic based on string)
                use uuid::Uuid;
                let bytes = id_str.as_bytes();
                let mut id_bytes = [0u8; 16];
                for (i, &b) in bytes.iter().enumerate() {
                    id_bytes[i % 16] = b;
                }
                Uuid::from_bytes(id_bytes)
            } else {
                clawlegion_core::AgentId::new_v4()
            };

            agent.id = agent_id;
            id_map.insert(agent_config.name.clone(), agent_id);

            // Set other fields
            agent.icon = agent_config.icon.clone();
            agent.capabilities = agent_config.capabilities.clone().unwrap_or_default();
            agent.agent_type = agent_config.agent_type.clone();
            agent.adapter_type = agent_config.adapter_type.clone();
            agent.adapter_config = agent_config.adapter_config.clone();
            agent.runtime_config = agent_config.runtime_config.clone();
            agent.permissions = agent_config.permissions.clone().into();

            agents.push(agent);
        }

        // Second pass: set reporting relationships
        for (i, agent_config) in self.agents.iter().enumerate() {
            if let Some(ref manager_name) = agent_config.reports_to {
                if let Some(&manager_id) = id_map.get(manager_name) {
                    agents[i].reports_to = Some(manager_id);
                } else {
                    return Err(Error::Org(OrgError::InvalidStructure(format!(
                        "Manager '{}' not found for agent '{}'",
                        manager_name, agent_config.name
                    ))));
                }
            }
        }

        Ok(agents)
    }
}

fn default_true() -> bool {
    true
}

fn default_adapter_type() -> String {
    "default".to_string()
}

fn default_agent_type() -> AgentTypeDef {
    AgentTypeDef::React
}
