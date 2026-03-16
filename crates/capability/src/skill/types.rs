//! Skill types - SkillType, ExecutionMode, Visibility, etc.

use serde::{Deserialize, Serialize};

/// Skill 类型标识
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillType {
    /// LLM-based: 仅使用系统提示词，通过 LLM 执行
    Llm,
    /// Code-based: 仅使用动态插件代码
    #[default]
    Code,
    /// Hybrid: 两者结合，先执行代码处理，再交给 LLM 总结
    Hybrid,
}

/// Skill 执行模式
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// 同步执行
    #[default]
    Sync,
    /// 异步执行 (后台运行，返回 task_id)
    Async,
    /// 流式执行 (LLM 流式输出)
    Stream,
}

/// Skill 可见性
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Public skills are available to all agents
    #[default]
    Public,
    /// Private skills must be explicitly loaded by agents
    Private,
}

/// Follow-up action requested by a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FollowUpAction {
    /// Call a tool
    CallTool {
        tool: String,
        args: serde_json::Value,
    },
    /// Call an MCP
    CallMcp {
        mcp: String,
        args: serde_json::Value,
    },
    /// Send a message
    SendMessage { recipient: String, content: String },
    /// Create a task
    CreateTask { title: String, description: String },
}
