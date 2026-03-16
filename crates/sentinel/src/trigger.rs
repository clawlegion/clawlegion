//! Trigger definitions for agent wakeup

use clawlegion_core::{AgentId, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trigger identifier
pub type TriggerId = String;

/// Wakeup trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeupTrigger {
    /// Unique trigger ID
    pub id: TriggerId,

    /// Agent ID to wake up
    pub agent_id: AgentId,

    /// Trigger condition
    pub condition: TriggerCondition,

    /// Wakeup method
    pub wakeup_method: WakeupMethod,

    /// Whether this trigger is enabled
    pub enabled: bool,

    /// Trigger priority (higher = checked first)
    pub priority: i32,

    /// Cooldown period in seconds (prevent rapid re-triggering)
    pub cooldown_secs: u64,

    /// Last triggered timestamp
    pub last_triggered_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl WakeupTrigger {
    /// Create a new trigger
    pub fn new(
        id: impl Into<String>,
        agent_id: AgentId,
        condition: TriggerCondition,
        wakeup_method: WakeupMethod,
    ) -> Self {
        Self {
            id: id.into(),
            agent_id,
            condition,
            wakeup_method,
            enabled: true,
            priority: 0,
            cooldown_secs: 60, // Default 1 minute cooldown
            last_triggered_at: None,
        }
    }

    /// Check if the trigger is in cooldown
    pub fn is_in_cooldown(&self) -> bool {
        if let Some(last_triggered) = self.last_triggered_at {
            let elapsed = chrono::Utc::now()
                .signed_duration_since(last_triggered)
                .num_seconds() as u64;
            elapsed < self.cooldown_secs
        } else {
            false
        }
    }

    /// Mark the trigger as fired
    pub fn mark_triggered(&mut self) {
        self.last_triggered_at = Some(chrono::Utc::now());
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set cooldown
    pub fn with_cooldown(mut self, cooldown_secs: u64) -> Self {
        self.cooldown_secs = cooldown_secs;
        self
    }
}

/// Trigger condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerCondition {
    /// Private message received
    PrivateMessage {
        /// Filter by sender (None = any sender)
        from: Option<AgentId>,
    },

    /// Task assigned
    TaskAssigned {
        /// Task ID
        task_id: Uuid,
    },

    /// Manager assigned task
    ManagerAssigned {
        /// Manager ID
        manager_id: AgentId,
    },

    /// Cron schedule
    Cron {
        /// Cron expression (e.g., "0 9 * * *" for 9 AM daily)
        expression: String,
    },

    /// Polling condition
    Polling {
        /// Poll interval in seconds
        interval_secs: u64,

        /// Poll endpoint or checker name
        checker: String,

        /// Poll parameters
        params: serde_json::Value,
    },

    /// Stream/WebSocket condition
    Stream {
        /// Stream URL
        url: String,

        /// Filter expression
        filter: Option<String>,
    },

    /// Webhook (external trigger)
    Webhook {
        /// Webhook secret for verification
        secret: Option<String>,

        /// Expected payload schema
        schema: Option<serde_json::Value>,
    },

    /// Custom condition (user-defined logic)
    Custom {
        /// Condition name/identifier
        name: String,

        /// Condition parameters
        params: serde_json::Value,
    },

    /// Compound condition (AND/OR of multiple conditions)
    Compound {
        /// Compound type
        op: CompoundOp,

        /// Child conditions
        conditions: Vec<TriggerCondition>,
    },

    /// Direct call (programmatic trigger)
    ///
    /// This condition is triggered when code directly calls the wakeup API.
    /// It bypasses the polling/evaluation cycle and immediately wakes up the agent.
    DirectCall {
        /// Caller identifier (e.g., tool name, function name)
        caller: String,

        /// Call parameters
        params: serde_json::Value,
    },
}

/// Compound operator
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompoundOp {
    /// All conditions must be true
    And,

    /// At least one condition must be true
    Or,

    /// Exactly one condition must be true
    Xor,
}

/// Wakeup method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WakeupMethod {
    /// Send a message to the agent
    SendMessage {
        /// Message content
        content: String,
    },

    /// Trigger a heartbeat
    Heartbeat,

    /// Execute a skill
    ExecuteSkill {
        /// Skill name
        skill: String,

        /// Skill input
        input: serde_json::Value,
    },

    /// Call a webhook
    CallWebhook {
        /// Webhook URL
        url: String,

        /// HTTP method
        method: String,

        /// Request body
        body: Option<serde_json::Value>,
    },

    /// Composite wakeup (multiple actions)
    Composite {
        /// Actions to execute in order
        actions: Vec<WakeupAction>,
    },
}

/// Wakeup action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WakeupAction {
    SendMessage {
        content: String,
    },
    Heartbeat,
    ExecuteSkill {
        skill: String,
        input: serde_json::Value,
    },
}

/// Trigger evaluation result
#[derive(Debug, Clone)]
pub struct TriggerEvaluation {
    /// Trigger ID
    pub trigger_id: TriggerId,

    /// Whether the condition is met
    pub condition_met: bool,

    /// Evaluation details
    pub details: Option<String>,

    /// Data associated with the trigger
    pub data: Option<serde_json::Value>,
}

impl TriggerEvaluation {
    pub fn met(trigger_id: impl Into<String>) -> Self {
        Self {
            trigger_id: trigger_id.into(),
            condition_met: true,
            details: None,
            data: None,
        }
    }

    pub fn not_met(trigger_id: impl Into<String>) -> Self {
        Self {
            trigger_id: trigger_id.into(),
            condition_met: false,
            details: None,
            data: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Trait for evaluating trigger conditions
#[async_trait::async_trait]
pub trait ConditionEvaluator: Send + Sync {
    /// Evaluate a condition
    async fn evaluate(&self, condition: &TriggerCondition) -> Result<TriggerEvaluation>;

    /// Register a custom condition handler
    fn register_handler(&mut self, name: &str, handler: Box<dyn CustomConditionHandler>);
}

/// Custom condition handler
#[async_trait::async_trait]
pub trait CustomConditionHandler: Send + Sync {
    /// Evaluate the custom condition
    async fn evaluate(&self, params: &serde_json::Value) -> Result<bool>;
}
