//! Agent core traits and types

use crate::{CompanyId, MessageId, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{
    de::Error as DeError, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Agent identifier
pub type AgentId = Uuid;

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is idle, waiting for tasks
    Idle,

    /// Agent is currently executing a task
    Running,

    /// Agent is paused
    Paused,

    /// Agent is being initialized
    Initializing,

    /// Agent encountered an error
    Error,

    /// Agent is being shut down
    Stopping,
}

/// Agent type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTypeDef {
    /// ReAct pattern agent (reasoning + acting)
    React,

    /// Flow-based agent (predefined workflows)
    Flow,

    /// Normal agent (rule-based, no LLM)
    Normal,

    /// Codex CLI-backed agent
    Codex,

    /// Claude Code CLI-backed agent
    ClaudeCode,

    /// OpenCode CLI-backed agent
    OpenCode,

    /// Custom agent type (plugin-defined)
    Custom { type_name: String },
}

impl Serialize for AgentTypeDef {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::React => serializer.serialize_str("react"),
            Self::Flow => serializer.serialize_str("flow"),
            Self::Normal => serializer.serialize_str("normal"),
            Self::Codex => serializer.serialize_str("codex"),
            Self::ClaudeCode => serializer.serialize_str("claude_code"),
            Self::OpenCode => serializer.serialize_str("open_code"),
            Self::Custom { type_name } => {
                let mut state = serializer.serialize_struct("AgentTypeDef", 2)?;
                state.serialize_field("type", "custom")?;
                state.serialize_field("type_name", type_name)?;
                state.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for AgentTypeDef {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum AgentTypeDefWire {
            Simple(String),
            Custom {
                #[serde(rename = "type")]
                kind: String,
                type_name: String,
            },
        }

        match AgentTypeDefWire::deserialize(deserializer)? {
            AgentTypeDefWire::Simple(value) => match value.as_str() {
                "react" => Ok(Self::React),
                "flow" => Ok(Self::Flow),
                "normal" => Ok(Self::Normal),
                "codex" => Ok(Self::Codex),
                "claude_code" => Ok(Self::ClaudeCode),
                "open_code" => Ok(Self::OpenCode),
                other => Err(D::Error::custom(format!("unknown agent type '{}'", other))),
            },
            AgentTypeDefWire::Custom { kind, type_name } => {
                if kind == "custom" {
                    Ok(Self::Custom { type_name })
                } else {
                    Err(D::Error::custom(format!(
                        "unknown custom agent kind '{}'",
                        kind
                    )))
                }
            }
        }
    }
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique agent ID
    pub id: AgentId,

    /// Company ID this agent belongs to
    pub company_id: CompanyId,

    /// Agent name
    pub name: String,

    /// Agent role (e.g., "ceo", "engineer")
    pub role: String,

    /// Agent title (e.g., "首席执行官", "高级工程师")
    pub title: String,

    /// Agent type
    #[serde(rename = "agent_type")]
    pub agent_type: AgentTypeDef,

    /// Agent icon (emoji or URL)
    pub icon: Option<String>,

    /// ID of the manager agent (None for CEO/root)
    pub reports_to: Option<AgentId>,

    /// Agent capabilities description
    pub capabilities: String,

    /// Skills loaded by this agent
    pub skills: Vec<String>,

    /// Adapter type for running this agent
    pub adapter_type: String,

    /// Adapter-specific configuration
    pub adapter_config: HashMap<String, serde_json::Value>,

    /// Runtime configuration
    pub runtime_config: HashMap<String, serde_json::Value>,

    /// Agent-specific tags for categorization
    pub tags: Vec<String>,
}

/// Agent runtime information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub config: AgentConfig,
    pub status: AgentStatus,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AgentInfo {
    pub fn new(config: AgentConfig) -> Self {
        let now = Utc::now();
        Self {
            config,
            status: AgentStatus::Initializing,
            last_heartbeat_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Heartbeat trigger reason
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum HeartbeatTrigger {
    /// Scheduled heartbeat (cron)
    Scheduled,

    /// New private message received
    PrivateMessage { message_id: MessageId },

    /// Task assigned to this agent
    TaskAssigned { task_id: Uuid },

    /// Manager assigned task
    ManagerAssigned { task_id: Uuid, manager_id: AgentId },

    /// Custom trigger from Sentinel
    Custom {
        trigger_id: String,
        data: serde_json::Value,
    },
}

/// Heartbeat context
#[derive(Debug, Clone)]
pub struct HeartbeatContext {
    pub trigger: HeartbeatTrigger,
    pub timestamp: DateTime<Utc>,
}

/// Agent SPI (Service Provider Interface)
///
/// This trait defines the interface that all agents must implement.
/// Third-party developers can implement this trait to create custom agent types.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get agent ID
    fn id(&self) -> AgentId;

    /// Get agent info
    fn info(&self) -> AgentInfo;

    /// Update agent status
    fn set_status(&mut self, status: AgentStatus);

    /// Execute a heartbeat cycle
    async fn heartbeat(&mut self, ctx: HeartbeatContext) -> Result<HeartbeatResult>;

    /// Load a skill at runtime
    async fn load_skill(&mut self, skill_name: &str) -> Result<()>;

    /// Unload a skill at runtime
    async fn unload_skill(&mut self, skill_name: &str) -> Result<()>;

    /// Get loaded skills
    fn loaded_skills(&self) -> Vec<String>;

    /// Shutdown the agent
    async fn shutdown(&mut self) -> Result<()>;
}

/// Heartbeat execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResult {
    /// Whether the heartbeat completed successfully
    pub success: bool,

    /// Tasks completed during this heartbeat
    pub completed_tasks: Vec<Uuid>,

    /// New tasks created during this heartbeat
    pub created_tasks: Vec<Uuid>,

    /// Messages sent during this heartbeat
    pub sent_messages: Vec<MessageId>,

    /// Error message if failed
    pub error: Option<String>,
}

impl HeartbeatResult {
    pub fn success() -> Self {
        Self {
            success: true,
            completed_tasks: vec![],
            created_tasks: vec![],
            sent_messages: vec![],
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            completed_tasks: vec![],
            created_tasks: vec![],
            sent_messages: vec![],
            error: Some(message.into()),
        }
    }
}
