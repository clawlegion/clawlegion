//! Anthropic Provider Plugin for ClawLegion
//!
//! This plugin provides Anthropic Claude API integration.

use async_trait::async_trait;
use clawlegion_core::{
    Error, LlmError, LlmMessage, LlmOptions, LlmProvider, LlmProviderConfig, LlmResponse, Result,
    StreamChunk, TokenUsage,
};
use clawlegion_plugin_sdk::{plugin, LlmProviderPlugin, Plugin, PluginMetadata};
use std::sync::Arc;

/// Anthropic Claude provider implementation
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    api_base: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    fn new(api_key: String, model: String, api_base: Option<String>) -> Self {
        Self {
            api_key,
            model,
            api_base: api_base.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat(&self, messages: Vec<LlmMessage>, options: LlmOptions) -> Result<LlmResponse> {
        let url = format!("{}/v1/messages", self.api_base);

        // Convert messages to Anthropic format
        let system_message = messages
            .iter()
            .find(|m| m.role == clawlegion_core::MessageRole::System)
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let non_system_messages: Vec<_> = messages
            .into_iter()
            .filter(|m| m.role != clawlegion_core::MessageRole::System)
            .collect();

        let mut request_body = serde_json::json!({
            "model": self.model,
            "max_tokens": options.max_tokens.unwrap_or(1024),
            "system": system_message,
            "messages": non_system_messages,
        });

        if let Some(temp) = options.temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }

        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                Error::Llm(LlmError::RequestFailed(format!(
                    "HTTP request failed: {}",
                    e
                )))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::Llm(LlmError::RequestFailed(format!(
                "API error ({}): {}",
                status, error_text
            ))));
        }

        let response_body: serde_json::Value = response.json().await.map_err(|e| {
            Error::Llm(LlmError::RequestFailed(format!(
                "Failed to parse response: {}",
                e
            )))
        })?;

        let content = response_body
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .map(String::from);

        let usage = response_body
            .get("usage")
            .map(|u| parse_usage(u))
            .unwrap_or_default();

        Ok(LlmResponse {
            content,
            tool_calls: vec![],
            finish_reason: response_body
                .get("stop_reason")
                .and_then(|r| r.as_str())
                .map(String::from),
            usage,
        })
    }

    async fn stream(
        &self,
        _messages: Vec<LlmMessage>,
        _options: LlmOptions,
    ) -> Result<Box<dyn futures_core::Stream<Item = Result<StreamChunk>> + Send + Unpin>> {
        Err(Error::Llm(LlmError::RequestFailed(
            "Streaming not implemented".to_string(),
        )))
    }
}

/// Parse token usage from JSON
fn parse_usage(usage: &serde_json::Value) -> TokenUsage {
    TokenUsage {
        prompt_tokens: usage
            .get("prompt_tokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        completion_tokens: usage
            .get("completion_tokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        total_tokens: usage
            .get("total_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    }
}

/// Anthropic Provider Plugin
pub struct AnthropicProviderPlugin {
    metadata: PluginMetadata,
}

impl AnthropicProviderPlugin {
    pub fn new() -> Self {
        Self {
            metadata: clawlegion_plugin_sdk::PluginBuilder::new("anthropic-provider", "0.1.0")
                .description("Anthropic Claude LLM provider plugin")
                .author("ClawLegion Team")
                .tag("llm")
                .tag("anthropic")
                .build(),
        }
    }

    pub fn default_metadata() -> PluginMetadata {
        clawlegion_plugin_sdk::PluginBuilder::new("anthropic-provider", "0.1.0")
            .description("Anthropic Claude LLM provider plugin")
            .author("ClawLegion Team")
            .tag("llm")
            .tag("anthropic")
            .build()
    }
}

impl Default for AnthropicProviderPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for AnthropicProviderPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

async fn init(
    &mut self,
    _ctx: clawlegion_plugin_sdk::PluginContext,
) -> anyhow::Result<()> {
    Ok(())
}

async fn shutdown(&mut self) -> anyhow::Result<()> {
    Ok(())
}
}

impl LlmProviderPlugin for AnthropicProviderPlugin {
    fn provider_type(&self) -> &str {
        "anthropic"
    }

    fn create_provider(
        &self,
        config: &LlmProviderConfig,
    ) -> clawlegion_plugin_sdk::Result<Arc<dyn LlmProvider>> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| {
                Error::Llm(LlmError::RequestFailed(
                    "API key not provided for Anthropic provider".to_string(),
                ))
            })?;

        let provider =
            AnthropicProvider::new(api_key, config.model.clone(), config.api_base.clone());

        Ok(Arc::new(provider))
    }
}

// Register the plugin
plugin!(AnthropicProviderPlugin);
