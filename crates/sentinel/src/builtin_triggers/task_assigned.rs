//! TaskAssigned trigger - wakes up agent when a task is assigned

use super::{BuiltinWakeupTrigger, TriggerContext};
use async_trait::async_trait;
use clawlegion_core::{AgentId, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// TaskAssigned trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignedConfig {
    /// Filter by task priority (None = any priority)
    pub min_priority: Option<i32>,

    /// Filter by task type (None = any type)
    pub task_type: Option<String>,
}

/// TaskAssigned trigger
///
/// Triggers when a task is assigned to the agent.
pub struct TaskAssignedTrigger {
    config: TaskAssignedConfig,
}

impl TaskAssignedTrigger {
    pub fn new() -> Self {
        Self {
            config: TaskAssignedConfig {
                min_priority: None,
                task_type: None,
            },
        }
    }

    pub fn with_priority_filter(min_priority: i32) -> Self {
        Self {
            config: TaskAssignedConfig {
                min_priority: Some(min_priority),
                task_type: None,
            },
        }
    }

    pub fn with_type_filter(task_type: impl Into<String>) -> Self {
        Self {
            config: TaskAssignedConfig {
                min_priority: None,
                task_type: Some(task_type.into()),
            },
        }
    }

    pub fn with_filters(min_priority: Option<i32>, task_type: Option<String>) -> Self {
        Self {
            config: TaskAssignedConfig {
                min_priority,
                task_type,
            },
        }
    }

    pub fn config(&self) -> &TaskAssignedConfig {
        &self.config
    }
}

impl Default for TaskAssignedTrigger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BuiltinWakeupTrigger for TaskAssignedTrigger {
    fn trigger_type(&self) -> &str {
        "task_assigned"
    }

    async fn should_trigger(&self, context: &TriggerContext) -> Result<bool> {
        // Check if task_id is provided
        let has_task = context.get("task_id").is_some();

        if !has_task {
            return Ok(false);
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
            // If no priority provided but filter is set, still trigger
            // (the task might have default priority)
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
            // If no type provided but filter is set, still trigger
            // (the task type might be checked at processing time)
        }

        Ok(true)
    }

    async fn wakeup(&self, agent_id: AgentId, data: serde_json::Value) -> Result<()> {
        tracing::info!(
            "TaskAssignedTrigger: Waking up agent {} with data: {}",
            agent_id,
            data
        );

        // In a real implementation, this would:
        // 1. Retrieve the actual task details from storage
        // 2. Pass the task to the agent for execution
        // 3. Update task status to "in_progress"

        Ok(())
    }
}

/// Task assigned wakeup data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TaskAssignedWakeupData {
    /// Task ID
    pub task_id: Uuid,

    /// Task title
    pub title: String,

    /// Task description
    pub description: String,

    /// Task priority (higher = more urgent)
    pub priority: i32,

    /// Task type
    pub task_type: String,

    /// Assigned by (who created/assigned this task)
    pub assigned_by: Option<AgentId>,

    /// Task deadline
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,

    /// Task parameters
    pub parameters: serde_json::Value,
}

impl TaskAssignedWakeupData {
    #[allow(dead_code)]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "task_id": self.task_id.to_string(),
            "title": self.title,
            "description": self.description,
            "priority": self.priority,
            "task_type": self.task_type,
            "assigned_by": self.assigned_by.map(|id| id.to_string()),
            "deadline": self.deadline.map(|d| d.to_rfc3339()),
            "parameters": self.parameters
        })
    }
}
