//! Capability Tree - hierarchical organization for Skills, MCPs, and Tools

use crate::{McpMetadata, SkillMetadata, ToolMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Capability tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityNode {
    /// Node name
    pub name: String,

    /// Node description
    pub description: Option<String>,

    /// Node type
    pub node_type: CapabilityNodeType,

    /// Child nodes
    pub children: Vec<CapabilityNode>,

    /// Capabilities in this node
    pub capabilities: Vec<CapabilityRef>,

    /// Tags for this node
    pub tags: Vec<String>,
}

/// Capability node type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityNodeType {
    /// Root node
    Root,

    /// Category node (e.g., "Communication", "DataAnalysis")
    Category,

    /// Sub-category node
    SubCategory,

    /// Leaf node (contains actual capabilities)
    Leaf,
}

/// Capability reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CapabilityRef {
    Skill { name: String, description: String },
    Tool { name: String, description: String },
    Mcp { name: String, description: String },
}

impl CapabilityNode {
    /// Create a root node
    pub fn root() -> Self {
        Self {
            name: "root".to_string(),
            description: Some("Root of the capability tree".to_string()),
            node_type: CapabilityNodeType::Root,
            children: vec![],
            capabilities: vec![],
            tags: vec![],
        }
    }

    /// Create a category node
    pub fn category(name: impl Into<String>, description: Option<String>) -> Self {
        Self {
            name: name.into(),
            description,
            node_type: CapabilityNodeType::Category,
            children: vec![],
            capabilities: vec![],
            tags: vec![],
        }
    }

    /// Create a leaf node
    pub fn leaf(name: impl Into<String>, description: Option<String>) -> Self {
        Self {
            name: name.into(),
            description,
            node_type: CapabilityNodeType::Leaf,
            children: vec![],
            capabilities: vec![],
            tags: vec![],
        }
    }

    /// Add a child node
    pub fn add_child(&mut self, child: CapabilityNode) {
        self.children.push(child);
    }

    /// Add a capability reference
    pub fn add_capability(&mut self, capability: CapabilityRef) {
        self.capabilities.push(capability);
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }

    /// Find a node by path (e.g., "Communication/Email")
    pub fn find_node(&self, path: &str) -> Option<&CapabilityNode> {
        let parts: Vec<&str> = path.split('/').collect();
        self.find_node_recursive(&parts)
    }

    fn find_node_recursive(&self, parts: &[&str]) -> Option<&CapabilityNode> {
        if parts.is_empty() {
            return Some(self);
        }

        let current_name = parts[0];
        let remaining = &parts[1..];

        for child in &self.children {
            if child.name == current_name {
                if remaining.is_empty() {
                    return Some(child);
                }
                return child.find_node_recursive(remaining);
            }
        }

        None
    }

    /// Find a node by path (mutable)
    pub fn find_node_mut(&mut self, path: &str) -> Option<&mut CapabilityNode> {
        let parts: Vec<&str> = path.split('/').collect();
        self.find_node_mut_recursive(&parts)
    }

    fn find_node_mut_recursive(&mut self, parts: &[&str]) -> Option<&mut CapabilityNode> {
        if parts.is_empty() {
            return Some(self);
        }

        let current_name = parts[0];
        let remaining = &parts[1..];

        for child in &mut self.children {
            if child.name == current_name {
                if remaining.is_empty() {
                    return Some(child);
                }
                return child.find_node_mut_recursive(remaining);
            }
        }

        None
    }

    /// Search for capabilities by keyword
    pub fn search(&self, keyword: &str) -> Vec<CapabilityRef> {
        let mut results = vec![];

        // Search in this node's capabilities
        for cap in &self.capabilities {
            if self.capability_matches(cap, keyword) {
                results.push(cap.clone());
            }
        }

        // Search in children
        for child in &self.children {
            results.extend(child.search(keyword));
        }

        results
    }

    fn capability_matches(&self, capability: &CapabilityRef, keyword: &str) -> bool {
        let (name, description) = match capability {
            CapabilityRef::Skill { name, description } => (name, description),
            CapabilityRef::Tool { name, description } => (name, description),
            CapabilityRef::Mcp { name, description } => (name, description),
        };

        name.to_lowercase().contains(&keyword.to_lowercase())
            || description.to_lowercase().contains(&keyword.to_lowercase())
    }

    /// Search by tags
    pub fn search_by_tag(&self, tag: &str) -> Vec<&CapabilityNode> {
        let mut results = vec![];
        self.search_by_tag_recursive(tag, &mut results);
        results
    }

    fn search_by_tag_recursive<'a>(&'a self, tag: &str, results: &mut Vec<&'a CapabilityNode>) {
        if self.tags.iter().any(|t| t == tag) {
            results.push(self);
        }

        for child in &self.children {
            child.search_by_tag_recursive(tag, results);
        }
    }

    /// Get all capabilities in this tree
    pub fn all_capabilities(&self) -> Vec<CapabilityRef> {
        let mut results = self.capabilities.clone();

        for child in &self.children {
            results.extend(child.all_capabilities());
        }

        results
    }

    /// Build a tree from a list of skills with tags
    pub fn from_skills(skills: &[SkillMetadata]) -> Self {
        let mut root = CapabilityNode::root();

        // Group skills by their first tag (category)
        let mut categories: HashMap<String, Vec<&SkillMetadata>> = HashMap::new();

        for skill in skills {
            let category = skill
                .tags
                .first()
                .cloned()
                .unwrap_or_else(|| "uncategorized".to_string());

            categories.entry(category).or_default().push(skill);
        }

        // Create category nodes
        for (category_name, category_skills) in categories {
            let mut category_node = CapabilityNode::category(&category_name, None);

            for skill in category_skills {
                category_node.add_capability(CapabilityRef::Skill {
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                });
            }

            root.add_child(category_node);
        }

        root
    }

    /// Build a tree from a list of tools with tags
    pub fn from_tools(tools: &[ToolMetadata]) -> Self {
        let mut root = CapabilityNode::root();

        // Group tools by their first tag (category)
        let mut categories: HashMap<String, Vec<&ToolMetadata>> = HashMap::new();

        for tool in tools {
            let category = tool
                .tags
                .first()
                .cloned()
                .unwrap_or_else(|| "uncategorized".to_string());

            categories.entry(category).or_default().push(tool);
        }

        // Create category nodes
        for (category_name, category_tools) in categories {
            let mut category_node = CapabilityNode::category(&category_name, None);

            for tool in category_tools {
                category_node.add_capability(CapabilityRef::Tool {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                });
            }

            root.add_child(category_node);
        }

        root
    }

    /// Build a tree from a list of MCPs with tags
    pub fn from_mcps(mcps: &[McpMetadata]) -> Self {
        let mut root = CapabilityNode::root();

        // Group MCPs by their first tag (category)
        let mut categories: HashMap<String, Vec<&McpMetadata>> = HashMap::new();

        for mcp in mcps {
            let category = mcp
                .tags
                .first()
                .cloned()
                .unwrap_or_else(|| "uncategorized".to_string());

            categories.entry(category).or_default().push(mcp);
        }

        // Create category nodes
        for (category_name, category_mcps) in categories {
            let mut category_node = CapabilityNode::category(&category_name, None);

            for mcp in category_mcps {
                category_node.add_capability(CapabilityRef::Mcp {
                    name: mcp.name.clone(),
                    description: mcp.description.clone(),
                });
            }

            root.add_child(category_node);
        }

        root
    }
}

/// Capability tree with fast lookup
pub struct CapabilityTree {
    root: CapabilityNode,
    name_to_path: HashMap<String, String>, // Fast lookup by name
}

impl CapabilityTree {
    pub fn new(root: CapabilityNode) -> Self {
        let mut name_to_path = HashMap::new();
        Self::build_index(&root, "".to_string(), &mut name_to_path);

        Self { root, name_to_path }
    }

    fn build_index(
        node: &CapabilityNode,
        parent_path: String,
        index: &mut HashMap<String, String>,
    ) {
        let current_path = if parent_path.is_empty() {
            node.name.clone()
        } else {
            format!("{}/{}", parent_path, node.name)
        };

        // Index capabilities in this node
        for cap in &node.capabilities {
            let name = match cap {
                CapabilityRef::Skill { name, .. } => name.clone(),
                CapabilityRef::Tool { name, .. } => name.clone(),
                CapabilityRef::Mcp { name, .. } => name.clone(),
            };
            index.insert(name, current_path.clone());
        }

        // Recurse into children
        for child in &node.children {
            Self::build_index(child, current_path.clone(), index);
        }
    }

    /// Get the path for a capability by name
    pub fn get_path(&self, name: &str) -> Option<&str> {
        self.name_to_path.get(name).map(|s| s.as_str())
    }

    /// Get a node by path
    pub fn get_node(&self, path: &str) -> Option<&CapabilityNode> {
        self.root.find_node(path)
    }

    /// Search for capabilities
    pub fn search(&self, keyword: &str) -> Vec<CapabilityRef> {
        self.root.search(keyword)
    }

    /// Get the root node
    pub fn root(&self) -> &CapabilityNode {
        &self.root
    }

    /// Print the tree as ASCII art
    pub fn print_ascii(&self) -> String {
        let mut output = String::new();
        self.print_node_ascii(&self.root, 0, &mut output);
        output
    }

    fn print_node_ascii(&self, node: &CapabilityNode, depth: usize, output: &mut String) {
        let indent = "  ".repeat(depth);

        if depth == 0 {
            output.push_str(&format!("{}\n", node.name));
        } else {
            output.push_str(&format!("{}├── {}\n", indent, node.name));
        }

        // Print capabilities
        for cap in &node.capabilities {
            let name = match cap {
                CapabilityRef::Skill { name, .. } => format!("[Skill] {}", name),
                CapabilityRef::Tool { name, .. } => format!("[Tool] {}", name),
                CapabilityRef::Mcp { name, .. } => format!("[MCP] {}", name),
            };
            output.push_str(&format!("{}    ├── {}\n", indent, name));
        }

        // Print children
        for child in &node.children {
            self.print_node_ascii(child, depth + 1, output);
        }
    }
}
