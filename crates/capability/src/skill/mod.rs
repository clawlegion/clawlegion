//! Skill system - high-level capability compositions
//!
//! This module provides the new Skill system for ClawLegion, supporting:
//! - LLM-based, Code-based, and Hybrid skills
//! - Dynamic loading from directory or marketplace
//! - Tool and MCP binding
//! - Agent-specific skill bindings
//! - Skill dependencies and composition

pub mod types;
pub use types::{ExecutionMode, FollowUpAction, SkillType, Visibility};

pub mod metadata;
pub use metadata::{SkillEvent, SkillInput, SkillMetadata, SkillOutput};

pub mod context;
pub use context::SkillContext;

pub mod trait_def;
pub use trait_def::{Skill, SkillBox, SkillInstance};

pub mod registry;
pub use registry::SkillRegistry;

pub mod manager;
pub use manager::SkillManager;

pub mod config;
pub use config::RawSkillConfig;

pub mod loader;
pub use loader::{LoadStrategy, LoaderConfig, SkillLoader};

pub mod dependency;
pub use dependency::SkillDependencyGraph;

pub mod executor;
pub use executor::{ExecutionResult, ExecutionStats, ExecutorConfig, SkillExecutor};

pub mod binding;
pub use binding::{BindingManager, DefaultToolProxy, SkillBinding, ToolProxy, ToolProxyBuilder};

pub mod marketplace;
pub use marketplace::{
    MarketplaceCategory, MarketplaceClient, MarketplaceConfig, MarketplaceSearchResponse,
    MarketplaceSkill,
};

pub mod installer;
pub use installer::{InstallationStatus, InstallerConfig, SkillInstaller};

pub mod agent_binding;
pub use agent_binding::{
    AgentSkillBinding, AgentSkillBindingBuilder, AgentSkillManager, AgentSkillStats,
    SkillRegistryTrait,
};

// Claude Skills compatibility
pub mod claude_runner;
pub use claude_runner::{ClaudeManifest, ClaudeSkillRunner};

pub mod claude_loader;
pub use claude_loader::{ClaudeSkillEntry, ClaudeSkillLoader};

// Re-export for backward compatibility
pub use types::Visibility as SkillVisibility;
