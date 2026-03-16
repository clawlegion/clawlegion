//! ClawLegion Capability System
//!
//! Provides Skill, MCP, and Tool abstractions with visibility control.

pub mod skill;
pub mod tools;

mod mcp;
mod tool;
mod tool_registry;
mod tree;

pub use mcp::{
    Mcp, McpBox, McpContext, McpEntry, McpMetadata, McpPrompt, McpResource, McpResult,
    McpServerType, Visibility as McpVisibility,
};
pub use skill::{
    AgentSkillBinding, AgentSkillBindingBuilder, AgentSkillManager, AgentSkillStats,
    BindingManager, DefaultToolProxy, ExecutionMode, FollowUpAction, InstallationStatus,
    InstallerConfig, MarketplaceCategory, MarketplaceClient, MarketplaceConfig,
    MarketplaceSearchResponse, MarketplaceSkill, Skill, SkillBinding, SkillContext, SkillInput,
    SkillInstance, SkillManager, SkillMetadata, SkillOutput, SkillRegistry, SkillRegistryTrait,
    SkillType, ToolProxy, ToolProxyBuilder, Visibility,
};
pub use tool::{
    Tool, ToolBox, ToolContext, ToolEntry, ToolMetadata, ToolResult, Visibility as ToolVisibility,
};
pub use tool_registry::ToolRegistry;
pub use tools::{CommandInput, CommandOutput, SandboxCommandTool, SandboxConfig};
pub use tree::*;
