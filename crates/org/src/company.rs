//! Company management

use clawlegion_core::CompanyId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Company representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    /// Unique company ID
    pub id: CompanyId,

    /// Company name
    pub name: String,

    /// Company description
    pub description: Option<String>,

    /// Issue prefix (e.g., "ACME" for ACME-123)
    pub issue_prefix: String,

    /// Issue counter for generating unique issue numbers
    pub issue_counter: u64,

    /// Require board approval for new agents
    pub require_approval_for_new_agents: bool,

    /// Brand color (hex)
    pub brand_color: Option<String>,

    /// Created at timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Updated at timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Company {
    /// Create a new company
    pub fn new(name: impl Into<String>, issue_prefix: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            issue_prefix: issue_prefix.into(),
            issue_counter: 0,
            require_approval_for_new_agents: true,
            brand_color: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a company with ID
    pub fn with_id(
        id: CompanyId,
        name: impl Into<String>,
        issue_prefix: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name: name.into(),
            description: None,
            issue_prefix: issue_prefix.into(),
            issue_counter: 0,
            require_approval_for_new_agents: true,
            brand_color: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate the next issue number for this company
    pub fn next_issue_number(&mut self) -> u64 {
        self.issue_counter += 1;
        self.issue_counter
    }

    /// Generate a full issue identifier (e.g., "ACME-123")
    pub fn generate_issue_id(&mut self) -> String {
        let number = self.next_issue_number();
        format!("{}-{}", self.issue_prefix, number)
    }

    /// Reset monthly counters (called at the start of each month)
    pub fn reset_monthly_counters(&mut self) {
        self.issue_counter = 0;
        self.updated_at = chrono::Utc::now();
    }
}

/// Company status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanyStatus {
    /// Company is active
    Active,

    /// Company is paused (no new agents/tasks)
    Paused,

    /// Company is being dissolved
    Dissolving,

    /// Company has been dissolved
    Dissolved,
}
