use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use clawlegion_core::{
    AgentError, Error as CoreError, HeartbeatResult, LlmError, LlmMessage, LlmOptions, LlmResponse,
    MessageRole, Result as CoreResult, StreamChunk, TokenUsage,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmExecuteRequest {
    pub model: String,
    pub messages: Vec<LlmProtocolMessage>,
    pub options: LlmProtocolOptions,
}

impl LlmExecuteRequest {
    pub fn new(model: String, messages: Vec<LlmMessage>, options: LlmOptions) -> CoreResult<Self> {
        if model.trim().is_empty() {
            return Err(CoreError::Llm(LlmError::RequestFailed(
                "llm request model must not be empty".to_string(),
            )));
        }

        if messages.is_empty() {
            return Err(CoreError::Llm(LlmError::RequestFailed(
                "llm request messages must not be empty".to_string(),
            )));
        }

        Ok(Self {
            model,
            messages: messages.into_iter().map(LlmProtocolMessage::from).collect(),
            options: LlmProtocolOptions::from(options),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProtocolMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl From<LlmMessage> for LlmProtocolMessage {
    fn from(value: LlmMessage) -> Self {
        Self {
            role: match value.role {
                MessageRole::System => "system".to_string(),
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::Tool => "tool".to_string(),
            },
            content: value.content,
            name: value.name,
            tool_call_id: value.tool_call_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmProtocolOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

impl From<LlmOptions> for LlmProtocolOptions {
    fn from(value: LlmOptions) -> Self {
        Self {
            temperature: value.temperature,
            max_tokens: value.max_tokens,
            top_p: value.top_p,
            frequency_penalty: value.frequency_penalty,
            presence_penalty: value.presence_penalty,
            stop: value.stop,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmExecuteResponse {
    pub text: String,
    pub usage: TokenUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

impl LlmExecuteResponse {
    pub fn validate(&self) -> CoreResult<()> {
        if self.text.is_empty() {
            return Err(CoreError::Llm(LlmError::RequestFailed(
                "llm response text must not be empty".to_string(),
            )));
        }
        if self.usage.total_tokens < self.usage.prompt_tokens + self.usage.completion_tokens {
            return Err(CoreError::Llm(LlmError::RequestFailed(
                "llm response usage.total_tokens is inconsistent".to_string(),
            )));
        }
        Ok(())
    }

    pub fn into_llm_response(self) -> LlmResponse {
        LlmResponse {
            content: Some(self.text),
            tool_calls: Vec::new(),
            finish_reason: self.finish_reason,
            usage: self.usage,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStreamChunk {
    pub delta: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

impl LlmStreamChunk {
    pub fn validate(&self) -> CoreResult<()> {
        if self.delta.is_empty() && self.finish_reason.is_none() {
            return Err(CoreError::Llm(LlmError::RequestFailed(
                "stream chunk requires delta or finish_reason".to_string(),
            )));
        }
        Ok(())
    }

    pub fn into_stream_chunk(self) -> StreamChunk {
        StreamChunk {
            delta: self.delta,
            finish_reason: self.finish_reason,
            tool_call_delta: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentAction {
    Heartbeat {
        trigger: String,
        timestamp_rfc3339: String,
    },
    LoadSkill {
        skill_name: String,
    },
    UnloadSkill {
        skill_name: String,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActionRequest {
    pub plugin_id: String,
    pub capability_id: String,
    pub agent_id: Uuid,
    #[serde(flatten)]
    pub action: AgentAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActionResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heartbeat: Option<HeartbeatResult>,
    #[serde(default)]
    pub loaded_skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentActionResponse {
    pub fn validate(&self) -> CoreResult<()> {
        if !self.success && self.error.is_none() {
            return Err(CoreError::Agent(AgentError::ExecutionFailed(
                "agent response must include error when success=false".to_string(),
            )));
        }
        Ok(())
    }

    pub fn into_heartbeat_result(self) -> HeartbeatResult {
        self.heartbeat.unwrap_or_else(|| {
            if self.success {
                HeartbeatResult::success()
            } else {
                HeartbeatResult::error(
                    self.error
                        .unwrap_or_else(|| "plugin agent heartbeat failed".to_string()),
                )
            }
        })
    }
}
