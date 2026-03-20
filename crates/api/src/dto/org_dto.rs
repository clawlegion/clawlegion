//! Organization DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Company response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyResponse {
    pub company_id: String,
    pub company_name: String,
    pub issue_prefix: String,
    pub agent_count: usize,
    pub created_at: Option<DateTime<Utc>>,
}

/// Org tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgNodeResponse {
    pub node_id: String,
    pub name: String,
    pub role: String,
    pub title: String,
    pub icon: Option<String>,
    pub depth: u32,
    pub children: Vec<OrgNodeResponse>,
}

/// Org tree response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgTreeResponse {
    pub root: Option<OrgNodeResponse>,
}

/// Flat agent list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgAgentResponse {
    pub id: String,
    pub name: String,
    pub role: String,
    pub title: String,
    pub depth: u32,
    pub parent_id: Option<String>,
    pub direct_reports_count: usize,
}

/// List org agents response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOrgAgentsResponse {
    pub agents: Vec<OrgAgentResponse>,
}
