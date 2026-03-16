//! MCP (Model Context Protocol) system

use async_trait::async_trait;
use clawlegion_core::{AgentId, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// MCP visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Public MCPs are available to all agents
    Public,

    /// Private MCPs must be bound to a Skill
    Private,
}

/// MCP metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMetadata {
    /// Unique MCP name
    pub name: String,

    /// MCP version (semver)
    pub version: String,

    /// MCP description
    pub description: String,

    /// Visibility
    pub visibility: Visibility,

    /// Tags for categorization and retrieval
    pub tags: Vec<String>,

    /// MCP server type
    pub server_type: McpServerType,

    /// Connection configuration
    pub connection_config: serde_json::Value,
}

/// MCP server type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpServerType {
    /// Local stdio server
    Stdio { command: String, args: Vec<String> },

    /// Remote HTTP server
    Http { url: String },

    /// Remote WebSocket server
    WebSocket { url: String },

    /// In-process server
    InProcess { module_path: String },
}

/// MCP resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Resource URI
    pub uri: String,

    /// Resource name
    pub name: String,

    /// Resource description
    pub description: Option<String>,

    /// MIME type
    pub mime_type: Option<String>,
}

/// MCP prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    /// Prompt name
    pub name: String,

    /// Prompt description
    pub description: String,

    /// Prompt arguments
    pub arguments: Vec<PromptArgument>,
}

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

/// MCP tool (exposed by MCP server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Input schema
    pub input_schema: serde_json::Value,
}

/// MCP execution context
#[derive(Debug, Clone)]
pub struct McpContext {
    /// Agent ID executing this MCP
    pub agent_id: AgentId,

    /// MCP configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Execution timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

impl McpContext {
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
}

/// MCP execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResult {
    /// Success flag
    pub success: bool,

    /// Result content
    pub content: Vec<McpContent>,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// MCP content type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpContent {
    Text {
        text: String,
    },
    Image {
        data: String,
        mime_type: String,
    },
    Resource {
        uri: String,
        data: serde_json::Value,
    },
}

impl McpResult {
    pub fn success(content: Vec<McpContent>) -> Self {
        Self {
            success: true,
            content,
            error: None,
            execution_time_ms: 0,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            content: vec![],
            error: Some(message.into()),
            execution_time_ms: 0,
        }
    }
}

/// MCP trait - defines Model Context Protocol capabilities
#[async_trait]
pub trait Mcp: Send + Sync {
    /// Get MCP metadata
    fn metadata(&self) -> &McpMetadata;

    /// Connect to the MCP server
    async fn connect(&self, ctx: &McpContext) -> Result<()>;

    /// Disconnect from the MCP server
    async fn disconnect(&self, ctx: &McpContext) -> Result<()>;

    /// List available resources
    async fn list_resources(&self, ctx: &McpContext) -> Result<Vec<McpResource>>;

    /// Read a resource
    async fn read_resource(&self, ctx: &McpContext, uri: &str) -> Result<String>;

    /// List available prompts
    async fn list_prompts(&self, ctx: &McpContext) -> Result<Vec<McpPrompt>>;

    /// Get a prompt
    async fn get_prompt(
        &self,
        ctx: &McpContext,
        name: &str,
        args: HashMap<String, String>,
    ) -> Result<String>;

    /// List available tools
    async fn list_tools(&self, ctx: &McpContext) -> Result<Vec<McpTool>>;

    /// Call a tool
    async fn call_tool(
        &self,
        ctx: &McpContext,
        name: &str,
        args: serde_json::Value,
    ) -> Result<McpResult>;

    /// Check if the MCP is connected
    async fn is_connected(&self) -> bool;
}

/// Type-erased MCP wrapper
pub type McpBox = Arc<dyn Mcp>;

/// MCP registry entry
pub struct McpEntry {
    pub mcp: McpBox,
    pub bound_skills: Vec<String>,
    pub connected: bool,
}

impl McpEntry {
    pub fn new(mcp: McpBox) -> Self {
        Self {
            mcp,
            bound_skills: vec![],
            connected: false,
        }
    }

    pub fn bind_to_skill(&mut self, skill_name: &str) {
        if !self.bound_skills.iter().any(|s| s == skill_name) {
            self.bound_skills.push(skill_name.to_string());
        }
    }

    pub fn is_accessible_by_skill(&self, skill_name: &str) -> bool {
        self.mcp.metadata().visibility == Visibility::Public
            || self.bound_skills.iter().any(|s| s == skill_name)
    }
}
