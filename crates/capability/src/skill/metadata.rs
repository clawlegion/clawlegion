//! Skill metadata definitions

use crate::skill::types::{ExecutionMode, SkillType, Visibility};
use serde::{Deserialize, Serialize};

/// Skill Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Unique skill name
    pub name: String,

    /// Skill version (semver)
    pub version: String,

    /// Skill description
    pub description: String,

    /// Skill author
    pub author: Option<String>,

    /// Visibility
    pub visibility: Visibility,

    /// Skill type (LLM/Code/Hybrid)
    #[serde(default)]
    pub skill_type: SkillType,

    /// Execution mode
    #[serde(default)]
    pub execution_mode: ExecutionMode,

    /// Tags for categorization and retrieval
    pub tags: Vec<String>,

    /// Required tools
    pub required_tools: Vec<String>,

    /// Required MCPs
    pub required_mcps: Vec<String>,

    /// Dependencies on other skills
    pub dependencies: Vec<String>,

    /// Configuration directory path (for loaded skills)
    #[serde(skip)]
    pub config_path: Option<String>,
}

impl SkillMetadata {
    /// Create a new SkillMetadata with defaults
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            author: None,
            visibility: Visibility::Public,
            skill_type: SkillType::default(),
            execution_mode: ExecutionMode::default(),
            tags: vec![],
            required_tools: vec![],
            required_mcps: vec![],
            dependencies: vec![],
            config_path: None,
        }
    }

    /// Set the author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set visibility
    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set skill type
    pub fn with_skill_type(mut self, skill_type: SkillType) -> Self {
        self.skill_type = skill_type;
        self
    }

    /// Set execution mode
    pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add required tool
    pub fn with_required_tool(mut self, tool: impl Into<String>) -> Self {
        self.required_tools.push(tool.into());
        self
    }

    /// Add required MCP
    pub fn with_required_mcp(mut self, mcp: impl Into<String>) -> Self {
        self.required_mcps.push(mcp.into());
        self
    }

    /// Add dependency
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }
}

/// Skill input
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillInput {
    /// Text input
    pub text: Option<String>,

    /// Structured data
    pub data: serde_json::Value,

    /// Attached file paths (if any)
    pub attachments: Vec<String>,
}

impl SkillInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    pub fn data(data: serde_json::Value) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

    pub fn with_attachment(mut self, path: impl Into<String>) -> Self {
        self.attachments.push(path.into());
        self
    }
}

/// Skill output
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillOutput {
    /// Text result
    pub text: Option<String>,

    /// Structured result
    pub data: Option<serde_json::Value>,

    /// Whether the skill completed successfully
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,

    /// Follow-up actions requested
    pub follow_ups: Vec<crate::skill::types::FollowUpAction>,
}

impl SkillOutput {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            success: true,
            ..Default::default()
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(message.into()),
            ..Default::default()
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_follow_up(mut self, action: crate::skill::types::FollowUpAction) -> Self {
        self.follow_ups.push(action);
        self
    }
}

/// Skill event for event-driven execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SkillEvent {
    /// File system event
    FileChanged { path: String, event_type: String },
    /// Agent message received
    MessageReceived { from: String, content: String },
    /// Timer event
    Timer { id: String, interval_ms: u64 },
    /// Custom event
    Custom {
        name: String,
        data: serde_json::Value,
    },
}
