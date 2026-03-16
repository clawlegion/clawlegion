//! Tool system - basic tool functions

use async_trait::async_trait;
use clawlegion_core::{AgentId, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Tool visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Public tools are available to all agents
    Public,

    /// Private tools must be bound to a Skill
    Private,
}

/// Tool metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Unique tool name
    pub name: String,

    /// Tool version (semver)
    pub version: String,

    /// Tool description
    pub description: String,

    /// Visibility
    pub visibility: Visibility,

    /// Tags for categorization and retrieval
    pub tags: Vec<String>,

    /// Input schema (JSON Schema)
    pub input_schema: serde_json::Value,

    /// Output schema (JSON Schema)
    pub output_schema: Option<serde_json::Value>,

    /// Whether this tool requires LLM
    pub requires_llm: bool,
}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Agent ID executing this tool
    pub agent_id: AgentId,

    /// Tool configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Execution timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

impl ToolContext {
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            config: HashMap::new(),
            timeout_ms: None,
        }
    }

    pub fn with_config(mut self, config: HashMap<String, serde_json::Value>) -> Self {
        self.config = config;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Success flag
    pub success: bool,

    /// Result data
    pub data: Option<serde_json::Value>,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ToolResult {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            execution_time_ms: 0,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
            execution_time_ms: 0,
        }
    }
}

/// Tool trait - defines basic tool functions
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get tool metadata
    fn metadata(&self) -> &ToolMetadata;

    /// Execute the tool with given arguments
    async fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> Result<ToolResult>;

    /// Get a description of what this tool does (for LLM prompts)
    fn description(&self) -> &str {
        &self.metadata().description
    }
}

/// Type-erased tool wrapper
pub type ToolBox = Arc<dyn Tool>;

/// Tool registry entry
pub struct ToolEntry {
    pub tool: ToolBox,
    pub bound_skills: Vec<String>, // Skills that have bound this private tool
}

impl ToolEntry {
    pub fn new(tool: ToolBox) -> Self {
        Self {
            tool,
            bound_skills: vec![],
        }
    }

    pub fn bind_to_skill(&mut self, skill_name: &str) {
        if !self.bound_skills.iter().any(|s| s == skill_name) {
            self.bound_skills.push(skill_name.to_string());
        }
    }

    pub fn is_accessible_by_skill(&self, skill_name: &str) -> bool {
        self.tool.metadata().visibility == Visibility::Public
            || self.bound_skills.iter().any(|s| s == skill_name)
    }
}

/// Macro for defining tools
#[macro_export]
macro_rules! define_tool {
    (
        name = $name:literal,
        version = $version:literal,
        description = $description:literal,
        visibility = $visibility:ident,
        tags = [$($tag:literal),*],
        input_schema = $input_schema:expr,
    ) => {
        pub struct ToolDef;

        impl ToolDef {
            pub fn metadata() -> $crate::ToolMetadata {
                $crate::ToolMetadata {
                    name: $name.to_string(),
                    version: $version.to_string(),
                    description: $description.to_string(),
                    visibility: $crate::Visibility::$visibility,
                    tags: vec![$($tag.to_string()),*],
                    input_schema: $input_schema,
                    output_schema: None,
                    requires_llm: false,
                }
            }
        }
    };
}
