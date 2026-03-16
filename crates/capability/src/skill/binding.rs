//! Skill Binding - Tool/MCP binding and access control

use clawlegion_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Skill binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBinding {
    /// Skill name
    pub skill_name: String,

    /// Bound tool names
    #[serde(default)]
    pub bound_tools: Vec<String>,

    /// Bound MCP names
    #[serde(default)]
    pub bound_mcps: Vec<String>,

    /// Configuration overrides
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

impl SkillBinding {
    /// Create a new skill binding
    pub fn new(skill_name: impl Into<String>) -> Self {
        Self {
            skill_name: skill_name.into(),
            bound_tools: vec![],
            bound_mcps: vec![],
            config: HashMap::new(),
        }
    }

    /// Add a bound tool
    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.bound_tools.push(tool.into());
        self
    }

    /// Add a bound MCP
    pub fn with_mcp(mut self, mcp: impl Into<String>) -> Self {
        self.bound_mcps.push(mcp.into());
        self
    }

    /// Add a config value
    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }

    /// Check if a tool is bound
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.bound_tools.iter().any(|t| t == tool_name)
    }

    /// Check if an MCP is bound
    pub fn has_mcp(&self, mcp_name: &str) -> bool {
        self.bound_mcps.iter().any(|m| m == mcp_name)
    }
}

/// Tool proxy for skill access control
#[async_trait::async_trait]
pub trait ToolProxy: Send + Sync {
    /// Call a tool by name
    async fn call(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<crate::tool::ToolResult>;

    /// Call an MCP by name
    async fn call_mcp(
        &self,
        mcp_name: &str,
        args: serde_json::Value,
    ) -> Result<crate::mcp::McpResult>;

    /// Check if a tool is accessible
    fn can_access_tool(&self, tool_name: &str) -> bool;

    /// Check if an MCP is accessible
    fn can_access_mcp(&self, mcp_name: &str) -> bool;
}

/// Default tool proxy implementation
pub struct DefaultToolProxy {
    /// Skill binding
    binding: SkillBinding,
    /// Available tools (name -> accessible)
    available_tools: HashMap<String, bool>,
    /// Available MCPs (name -> accessible)
    available_mcps: HashMap<String, bool>,
}

impl DefaultToolProxy {
    /// Create a new default tool proxy
    pub fn new(binding: SkillBinding) -> Self {
        Self {
            binding,
            available_tools: HashMap::new(),
            available_mcps: HashMap::new(),
        }
    }

    /// Set available tools
    pub fn with_available_tools(mut self, tools: Vec<String>) -> Self {
        for tool in tools {
            let accessible = self.binding.has_tool(&tool);
            self.available_tools.insert(tool, accessible);
        }
        self
    }

    /// Set available MCPs
    pub fn with_available_mcps(mut self, mcps: Vec<String>) -> Self {
        for mcp in mcps {
            let accessible = self.binding.has_mcp(&mcp);
            self.available_mcps.insert(mcp, accessible);
        }
        self
    }
}

#[async_trait::async_trait]
impl ToolProxy for DefaultToolProxy {
    async fn call(
        &self,
        tool_name: &str,
        _args: serde_json::Value,
    ) -> Result<crate::tool::ToolResult> {
        if !self.can_access_tool(tool_name) {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Tool '{}' is not accessible to skill '{}'",
                    tool_name, self.binding.skill_name
                )),
            ));
        }

        // In a full implementation, this would actually call the tool
        // For now, return a placeholder error
        Err(clawlegion_core::Error::Capability(
            clawlegion_core::CapabilityError::NotFound(format!(
                "Tool '{}' call not implemented - requires tool registry integration",
                tool_name
            )),
        ))
    }

    async fn call_mcp(
        &self,
        mcp_name: &str,
        _args: serde_json::Value,
    ) -> Result<crate::mcp::McpResult> {
        if !self.can_access_mcp(mcp_name) {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "MCP '{}' is not accessible to skill '{}'",
                    mcp_name, self.binding.skill_name
                )),
            ));
        }

        // In a full implementation, this would actually call the MCP
        Err(clawlegion_core::Error::Capability(
            clawlegion_core::CapabilityError::NotFound(format!(
                "MCP '{}' call not implemented - requires MCP registry integration",
                mcp_name
            )),
        ))
    }

    fn can_access_tool(&self, tool_name: &str) -> bool {
        self.available_tools
            .get(tool_name)
            .copied()
            .unwrap_or(false)
    }

    fn can_access_mcp(&self, mcp_name: &str) -> bool {
        self.available_mcps.get(mcp_name).copied().unwrap_or(false)
    }
}

/// Tool proxy builder
pub struct ToolProxyBuilder {
    binding: SkillBinding,
    available_tools: Vec<String>,
    available_mcps: Vec<String>,
}

impl ToolProxyBuilder {
    /// Create a new tool proxy builder
    pub fn new(skill_name: impl Into<String>) -> Self {
        Self {
            binding: SkillBinding::new(skill_name),
            available_tools: vec![],
            available_mcps: vec![],
        }
    }

    /// Bind a tool
    pub fn bind_tool(mut self, tool: impl Into<String>) -> Self {
        self.binding = self.binding.with_tool(tool);
        self
    }

    /// Bind an MCP
    pub fn bind_mcp(mut self, mcp: impl Into<String>) -> Self {
        self.binding = self.binding.with_mcp(mcp);
        self
    }

    /// Set available tools
    pub fn with_available_tools(mut self, tools: Vec<String>) -> Self {
        self.available_tools = tools;
        self
    }

    /// Set available MCPs
    pub fn with_available_mcps(mut self, mcps: Vec<String>) -> Self {
        self.available_mcps = mcps;
        self
    }

    /// Build the tool proxy
    pub fn build(self) -> DefaultToolProxy {
        DefaultToolProxy::new(self.binding)
            .with_available_tools(self.available_tools)
            .with_available_mcps(self.available_mcps)
    }
}

/// Binding manager for managing multiple skill bindings
pub struct BindingManager {
    bindings: dashmap::DashMap<String, SkillBinding>,
}

impl BindingManager {
    /// Create a new binding manager
    pub fn new() -> Self {
        Self {
            bindings: dashmap::DashMap::new(),
        }
    }

    /// Add a skill binding
    pub fn add_binding(&self, binding: SkillBinding) {
        self.bindings.insert(binding.skill_name.clone(), binding);
    }

    /// Get a skill binding
    pub fn get_binding(&self, skill_name: &str) -> Option<SkillBinding> {
        self.bindings.get(skill_name).map(|b| b.clone())
    }

    /// Remove a skill binding
    pub fn remove_binding(&self, skill_name: &str) -> Option<SkillBinding> {
        self.bindings.remove(skill_name).map(|(_, b)| b)
    }

    /// Check if a skill has access to a tool
    pub fn skill_has_tool_access(&self, skill_name: &str, tool_name: &str) -> bool {
        self.bindings
            .get(skill_name)
            .map(|b| b.has_tool(tool_name))
            .unwrap_or(false)
    }

    /// Check if a skill has access to an MCP
    pub fn skill_has_mcp_access(&self, skill_name: &str, mcp_name: &str) -> bool {
        self.bindings
            .get(skill_name)
            .map(|b| b.has_mcp(mcp_name))
            .unwrap_or(false)
    }

    /// Create a tool proxy for a skill
    pub fn create_proxy(&self, skill_name: &str) -> Option<DefaultToolProxy> {
        let binding = self.get_binding(skill_name)?;
        Some(DefaultToolProxy::new(binding))
    }

    /// Get all bindings
    pub fn list_bindings(&self) -> Vec<SkillBinding> {
        self.bindings.iter().map(|b| b.clone()).collect()
    }
}

impl Default for BindingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_binding_creation() {
        let binding = SkillBinding::new("test-skill")
            .with_tool("tool1")
            .with_tool("tool2")
            .with_mcp("mcp1")
            .with_config("key", serde_json::json!("value"));

        assert_eq!(binding.skill_name, "test-skill");
        assert!(binding.has_tool("tool1"));
        assert!(binding.has_tool("tool2"));
        assert!(!binding.has_tool("tool3"));
        assert!(binding.has_mcp("mcp1"));
        assert!(!binding.has_mcp("mcp2"));
    }

    #[test]
    fn test_binding_manager() {
        let manager = BindingManager::new();

        let binding = SkillBinding::new("test-skill").with_tool("tool1");
        manager.add_binding(binding);

        assert!(manager.skill_has_tool_access("test-skill", "tool1"));
        assert!(!manager.skill_has_tool_access("test-skill", "tool2"));
        assert!(!manager.skill_has_tool_access("other-skill", "tool1"));
    }

    #[test]
    fn test_tool_proxy_access() {
        let binding = SkillBinding::new("test-skill").with_tool("tool1");
        let proxy = DefaultToolProxy::new(binding)
            .with_available_tools(vec!["tool1".to_string(), "tool2".to_string()]);

        assert!(proxy.can_access_tool("tool1"));
        assert!(!proxy.can_access_tool("tool2"));
        assert!(!proxy.can_access_tool("tool3"));
    }
}
