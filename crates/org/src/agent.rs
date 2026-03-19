//! Agent within organization context

use clawlegion_core::{AgentId, AgentStatus, AgentTypeDef, CompanyId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Org Agent representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgAgent {
    /// Unique agent ID
    pub id: AgentId,

    /// Company ID this agent belongs to
    pub company_id: CompanyId,

    /// Agent name
    pub name: String,

    /// Agent role (e.g., "ceo", "engineer")
    pub role: String,

    /// Agent title (e.g., "CEO", "Engineer")
    pub title: String,

    /// Agent icon (emoji or URL)
    pub icon: Option<String>,

    /// Current status
    pub status: AgentStatus,

    /// ID of the manager agent (None for CEO/root)
    pub reports_to: Option<AgentId>,

    /// Agent capabilities description
    pub capabilities: String,

    /// Agent type
    pub agent_type: AgentTypeDef,

    /// Adapter type for running this agent
    pub adapter_type: String,

    /// Adapter-specific configuration
    pub adapter_config: HashMap<String, serde_json::Value>,

    /// Runtime configuration
    pub runtime_config: HashMap<String, serde_json::Value>,

    /// Permissions
    pub permissions: AgentPermissions,

    /// Last heartbeat timestamp
    pub last_heartbeat_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Created at timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Updated at timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl OrgAgent {
    /// Create a new agent
    pub fn new(
        company_id: CompanyId,
        name: impl Into<String>,
        role: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            company_id,
            name: name.into(),
            role: role.into(),
            title: title.into(),
            icon: None,
            status: AgentStatus::Initializing,
            reports_to: None,
            capabilities: String::new(),
            agent_type: AgentTypeDef::React,
            adapter_type: "default".to_string(),
            adapter_config: HashMap::new(),
            runtime_config: HashMap::new(),
            permissions: AgentPermissions::default(),
            last_heartbeat_at: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the manager/ supervisor for this agent
    pub fn set_reports_to(&mut self, manager_id: Option<AgentId>) {
        self.reports_to = manager_id;
        self.updated_at = chrono::Utc::now();
    }

    /// Set the adapter type
    pub fn set_adapter(
        &mut self,
        adapter_type: impl Into<String>,
        config: HashMap<String, serde_json::Value>,
    ) {
        self.adapter_type = adapter_type.into();
        self.adapter_config = config;
        self.updated_at = chrono::Utc::now();
    }

    /// Update status
    pub fn set_status(&mut self, status: AgentStatus) {
        self.status = status;
        self.updated_at = chrono::Utc::now();
    }

    /// Update last heartbeat
    pub fn heartbeat(&mut self) {
        self.last_heartbeat_at = Some(chrono::Utc::now());
        self.updated_at = chrono::Utc::now();
    }

    /// Check if this agent is the CEO (root of org tree)
    pub fn is_ceo(&self) -> bool {
        self.reports_to.is_none()
    }

    /// Check if this agent reports to another agent
    pub fn reports_to_agent(&self, manager_id: AgentId) -> bool {
        self.reports_to == Some(manager_id)
    }
}

/// Agent permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPermissions {
    /// Can hire new agents
    pub can_hire: bool,

    /// Can fire agents
    pub can_fire: bool,

    /// Can assign tasks
    pub can_assign_tasks: bool,

    /// Can approve tasks
    pub can_approve_tasks: bool,

    /// Can access company-wide data
    pub can_access_company_data: bool,

    /// Can create goals
    pub can_create_goals: bool,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            can_hire: false,
            can_fire: false,
            can_assign_tasks: true,
            can_approve_tasks: false,
            can_access_company_data: true,
            can_create_goals: false,
        }
    }
}

impl AgentPermissions {
    /// CEO permissions
    pub fn ceo() -> Self {
        Self {
            can_hire: true,
            can_fire: true,
            can_assign_tasks: true,
            can_approve_tasks: true,
            can_access_company_data: true,
            can_create_goals: true,
        }
    }

    /// Manager permissions
    pub fn manager() -> Self {
        Self {
            can_hire: true,
            can_fire: false,
            can_assign_tasks: true,
            can_approve_tasks: true,
            can_access_company_data: true,
            can_create_goals: true,
        }
    }

    /// Individual contributor permissions
    pub fn contributor() -> Self {
        Self {
            can_hire: false,
            can_fire: false,
            can_assign_tasks: false,
            can_approve_tasks: false,
            can_access_company_data: true,
            can_create_goals: false,
        }
    }
}
