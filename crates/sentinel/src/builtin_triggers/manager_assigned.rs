//! ManagerAssigned trigger - wakes up agent when a manager assigns a task

use super::{BuiltinWakeupTrigger, TriggerContext};
use async_trait::async_trait;
use clawlegion_core::{AgentId, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ManagerAssigned trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerAssignedConfig {
    /// Filter by specific manager (None = any manager)
    pub manager_id: Option<AgentId>,

    /// Filter by task priority (None = any priority)
    pub min_priority: Option<i32>,

    /// Filter by task type (None = any type)
    pub task_type: Option<String>,
}

/// ManagerAssigned trigger
///
/// Triggers when a manager (supervisor) assigns a task to the agent.
/// This is similar to TaskAssigned but specifically for tasks coming
/// from a manager in the organizational hierarchy.
pub struct ManagerAssignedTrigger {
    config: ManagerAssignedConfig,
}

impl ManagerAssignedTrigger {
    pub fn new() -> Self {
        Self {
            config: ManagerAssignedConfig {
                manager_id: None,
                min_priority: None,
                task_type: None,
            },
        }
    }

    pub fn with_manager_filter(manager_id: AgentId) -> Self {
        Self {
            config: ManagerAssignedConfig {
                manager_id: Some(manager_id),
                min_priority: None,
                task_type: None,
            },
        }
    }

    pub fn with_priority_filter(min_priority: i32) -> Self {
        Self {
            config: ManagerAssignedConfig {
                manager_id: None,
                min_priority: Some(min_priority),
                task_type: None,
            },
        }
    }

    pub fn with_type_filter(task_type: impl Into<String>) -> Self {
        Self {
            config: ManagerAssignedConfig {
                manager_id: None,
                min_priority: None,
                task_type: Some(task_type.into()),
            },
        }
    }

    pub fn with_filters(
        manager_id: Option<AgentId>,
        min_priority: Option<i32>,
        task_type: Option<String>,
    ) -> Self {
        Self {
            config: ManagerAssignedConfig {
                manager_id,
                min_priority,
                task_type,
            },
        }
    }

    pub fn config(&self) -> &ManagerAssignedConfig {
        &self.config
    }
}

impl Default for ManagerAssignedTrigger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BuiltinWakeupTrigger for ManagerAssignedTrigger {
    fn trigger_type(&self) -> &str {
        "manager_assigned"
    }

    async fn should_trigger(&self, context: &TriggerContext) -> Result<bool> {
        // Check if task_id and manager_id are provided
        let has_task = context.get("task_id").is_some();
        let has_manager = context.get("manager_id").is_some();

        if !has_task || !has_manager {
            return Ok(false);
        }

        // If manager filter is configured, check if manager matches
        if let Some(filter_manager_id) = self.config.manager_id {
            if let Some(manager_value) = context.get("manager_id") {
                if let Some(manager_str) = manager_value.as_str() {
                    if let Ok(manager_id) = Uuid::parse_str(manager_str) {
                        if manager_id != filter_manager_id {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }

        // If priority filter is configured, check if task meets priority
        if let Some(min_priority) = self.config.min_priority {
            if let Some(priority_value) = context.get("priority") {
                if let Some(priority) = priority_value.as_i64() {
                    if priority < min_priority as i64 {
                        return Ok(false);
                    }
                }
            }
        }

        // If task type filter is configured, check if type matches
        if let Some(ref filter_type) = self.config.task_type {
            if let Some(type_value) = context.get("task_type") {
                if let Some(type_str) = type_value.as_str() {
                    if type_str != filter_type {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    async fn wakeup(&self, agent_id: AgentId, data: serde_json::Value) -> Result<()> {
        tracing::info!(
            "ManagerAssignedTrigger: Waking up agent {} with data: {}",
            agent_id,
            data
        );

        // In a real implementation, this would:
        // 1. Retrieve the task details from storage
        // 2. Verify the manager has authority to assign tasks to this agent
        // 3. Pass the task to the agent for execution
        // 4. Update task status and notify the manager

        Ok(())
    }
}

/// Manager assigned wakeup data
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerAssignedWakeupData {
    /// Task ID
    pub task_id: Uuid,

    /// Manager ID who assigned the task
    pub manager_id: AgentId,

    /// Task title
    pub title: String,

    /// Task description
    pub description: String,

    /// Task priority (higher = more urgent)
    pub priority: i32,

    /// Task type
    pub task_type: String,

    /// Task deadline
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,

    /// Task parameters
    pub parameters: serde_json::Value,

    /// Additional instructions from manager
    pub manager_notes: Option<String>,
}

impl ManagerAssignedWakeupData {
    #[allow(dead_code)]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "task_id": self.task_id.to_string(),
            "manager_id": self.manager_id.to_string(),
            "title": self.title,
            "description": self.description,
            "priority": self.priority,
            "task_type": self.task_type,
            "deadline": self.deadline.map(|d| d.to_rfc3339()),
            "parameters": self.parameters,
            "manager_notes": self.manager_notes
        })
    }
}
