//! Agent DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub role: String,
    pub title: String,
    pub status: String,
    pub icon: Option<String>,
    pub reports_to: Option<String>,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub budget_remaining_cents: Option<u64>,
    pub token_usage_total: Option<u64>,
    pub cost_total_cents: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDetailResponse {
    pub id: String,
    pub name: String,
    pub role: String,
    pub title: String,
    pub status: String,
    pub icon: Option<String>,
    pub reports_to: Option<String>,
    pub capabilities: String,
    pub skills: Vec<String>,
    pub budget_remaining_cents: Option<u64>,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub tasks_completed: Option<u64>,
    pub tasks_pending: Option<u64>,
    pub token_usage_total: Option<u64>,
    pub cost_total_cents: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusResponse {
    pub agent_id: String,
    pub status: String,
    pub current_task: Option<String>,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub heartbeat_interval_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillResponse {
    pub name: String,
    pub version: String,
    pub description: String,
    pub execution_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAgentSkillsResponse {
    pub agent_id: String,
    pub skills: Vec<AgentSkillResponse>,
}

impl AgentResponse {
    pub fn from_agent_info(info: &clawlegion_core::AgentInfo) -> Self {
        Self {
            id: info.config.id.to_string(),
            name: info.config.name.clone(),
            role: info.config.role.clone(),
            title: info.config.title.clone(),
            status: format!("{:?}", info.status).to_lowercase(),
            icon: info.config.icon.clone(),
            reports_to: info.config.reports_to.map(|id| id.to_string()),
            last_heartbeat: info.last_heartbeat_at,
            budget_remaining_cents: info.config.budget_monthly_cents,
            token_usage_total: None,
            cost_total_cents: None,
        }
    }
}

impl AgentDetailResponse {
    pub fn from_agent_info(info: &clawlegion_core::AgentInfo) -> Self {
        Self {
            id: info.config.id.to_string(),
            name: info.config.name.clone(),
            role: info.config.role.clone(),
            title: info.config.title.clone(),
            status: format!("{:?}", info.status).to_lowercase(),
            icon: info.config.icon.clone(),
            reports_to: info.config.reports_to.map(|id| id.to_string()),
            capabilities: info.config.capabilities.clone(),
            skills: info.config.skills.clone(),
            budget_remaining_cents: info.config.budget_monthly_cents,
            last_heartbeat: info.last_heartbeat_at,
            tasks_completed: None,
            tasks_pending: None,
            token_usage_total: None,
            cost_total_cents: None,
        }
    }
}

impl AgentStatusResponse {
    pub fn from_agent_info(
        info: &clawlegion_core::AgentInfo,
        heartbeat_interval_secs: Option<u64>,
    ) -> Self {
        Self {
            agent_id: info.config.id.to_string(),
            status: format!("{:?}", info.status).to_lowercase(),
            current_task: None,
            last_heartbeat: info.last_heartbeat_at,
            heartbeat_interval_secs,
        }
    }
}
