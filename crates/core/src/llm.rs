//! LLM (Large Language Model) abstraction layer

use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM provider identifier
pub type ProviderId = String;

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
    pub tool_call_id: Option<String>,
}

impl LlmMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            name: None,
            tool_call_id: None,
        }
    }
}

/// Tool definition for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool call request from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub result: String,
    pub is_error: bool,
}

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// Generated content
    pub content: Option<String>,

    /// Tool calls requested by the model
    pub tool_calls: Vec<ToolCall>,

    /// Finish reason
    pub finish_reason: Option<String>,

    /// Token usage
    pub usage: TokenUsage,
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// LLM request options
#[derive(Debug, Clone, Default)]
pub struct LlmOptions {
    /// Temperature for sampling (0.0 - 2.0)
    pub temperature: Option<f64>,

    /// Maximum tokens to generate
    pub max_tokens: Option<u64>,

    /// Top-p sampling
    pub top_p: Option<f64>,

    /// Frequency penalty
    pub frequency_penalty: Option<f64>,

    /// Presence penalty
    pub presence_penalty: Option<f64>,

    /// Stop sequences
    pub stop: Option<Vec<String>>,

    /// Tools available for this request
    pub tools: Option<Vec<ToolDefinition>>,

    /// Force the model to call a specific tool
    pub tool_choice: Option<String>,
}

/// Stream chunk for streaming responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub delta: String,
    pub finish_reason: Option<String>,
    pub tool_call_delta: Option<ToolCallDelta>,
}

/// Partial tool call delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: Option<String>,
}

/// LLM provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Get provider model
    fn model(&self) -> &str;

    /// Send a chat completion request
    async fn chat(&self, messages: Vec<LlmMessage>, options: LlmOptions) -> Result<LlmResponse>;

    /// Send a streaming chat completion request
    async fn stream(
        &self,
        messages: Vec<LlmMessage>,
        options: LlmOptions,
    ) -> Result<Box<dyn futures_core::Stream<Item = Result<StreamChunk>> + Send + Unpin>>;

    /// Count tokens for a message (optional, provider-specific)
    async fn count_tokens(&self, messages: &[LlmMessage]) -> Result<u64> {
        // Default implementation returns 0
        // Providers should override this for accurate token counting
        Ok(messages.iter().map(|m| (m.content.len() / 4) as u64).sum())
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider: ProviderId,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub timeout_secs: Option<u64>,
    pub extra: HashMap<String, serde_json::Value>,
}
