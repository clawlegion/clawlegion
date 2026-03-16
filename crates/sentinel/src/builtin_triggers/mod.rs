//! Built-in wakeup triggers for ClawLegion agents
//!
//! This module provides three built-in trigger types that are automatically
//! available to all agents:
//! - PrivateMessage: Triggered when a private message is received
//! - TaskAssigned: Triggered when a task is assigned
//! - ManagerAssigned: Triggered when a manager assigns a task

mod manager_assigned;
mod private_message;
mod task_assigned;

pub use manager_assigned::ManagerAssignedTrigger;
pub use private_message::PrivateMessageTrigger;
pub use task_assigned::TaskAssignedTrigger;

use async_trait::async_trait;
use clawlegion_core::{AgentId, Result};
use std::collections::HashMap;

/// Trigger context for builtin triggers
#[derive(Debug, Clone)]
pub struct TriggerContext {
    /// Additional context data
    pub data: HashMap<String, serde_json::Value>,
}

impl TriggerContext {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn with_data(mut self, key: &str, value: serde_json::Value) -> Self {
        self.data.insert(key.to_string(), value);
        self
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }
}

impl Default for TriggerContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in wakeup trigger trait
///
/// All builtin triggers implement this trait.
#[async_trait]
pub trait BuiltinWakeupTrigger: Send + Sync {
    /// Get trigger type identifier
    fn trigger_type(&self) -> &str;

    /// Check if the trigger should fire
    async fn should_trigger(&self, context: &TriggerContext) -> Result<bool>;

    /// Execute the wakeup
    async fn wakeup(&self, agent_id: AgentId, data: serde_json::Value) -> Result<()>;
}

/// Registry for builtin triggers
pub struct BuiltinTriggerRegistry {
    triggers: Vec<Box<dyn BuiltinWakeupTrigger>>,
}

impl BuiltinTriggerRegistry {
    pub fn new() -> Self {
        Self {
            triggers: Vec::new(),
        }
    }

    /// Register a builtin trigger
    pub fn register(&mut self, trigger: Box<dyn BuiltinWakeupTrigger>) {
        self.triggers.push(trigger);
    }

    /// Get all registered triggers
    pub fn list_triggers(&self) -> Vec<&dyn BuiltinWakeupTrigger> {
        self.triggers.iter().map(|t| t.as_ref()).collect()
    }

    /// Get a trigger by type
    pub fn get_by_type(&self, trigger_type: &str) -> Option<&dyn BuiltinWakeupTrigger> {
        self.triggers
            .iter()
            .find(|t| t.trigger_type() == trigger_type)
            .map(|t| t.as_ref())
    }

    /// Create a registry with all default builtin triggers
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Register all three builtin triggers
        registry.register(Box::new(PrivateMessageTrigger::new()));
        registry.register(Box::new(TaskAssignedTrigger::new()));
        registry.register(Box::new(ManagerAssignedTrigger::new()));

        registry
    }
}

impl Default for BuiltinTriggerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro for creating builtin trigger metadata
#[macro_export]
macro_rules! builtin_trigger {
    (
        name = $name:literal,
        version = $version:literal,
        description = $description:literal,
    ) => {
        pub struct TriggerDef;

        impl TriggerDef {
            pub fn metadata() -> serde_json::Value {
                serde_json::json!({
                    "name": $name,
                    "version": $version,
                    "description": $description,
                    "type": "builtin_trigger"
                })
            }
        }
    };
}
