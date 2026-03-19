//! OpenAI Provider Plugin for ClawLegion
//!
//! This plugin provides OpenAI-compatible API integration.

use async_trait::async_trait;
use clawlegion_core::{
    Error, LlmError, LlmMessage, LlmOptions, LlmProvider, LlmProviderConfig, LlmResponse, Result,
    StreamChunk,
};
use clawlegion_plugin_sdk::{plugin, LlmProviderPlugin, Plugin, PluginMetadata};
use std::sync::Arc;

/// OpenAI provider implementation
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    api_base: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    fn new(api_key: String, model: String, api_base: Option<String>) -> Self {
        Self {
            api_key,
            model,
            api_base: api_base.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            client: reqwest::Client::new(),
        }
    }

    fn compatible(api_key: String, model: String, api_base: String) -> Self {
        Self::new(api_key, model, Some(api_base))
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat(&self, messages: Vec<LlmMessage>, options: LlmOptions) -> Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.api_base);

        let mut request_body = serde_json::json!({
            "model": self.model,
            "messages": messages,
        });

        if let Some(temp) = options.temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }
        if let Some(tools) = options.tools {
            request_body["tools"] = serde_json::to_value(tools).map_err(|e| {
                Error::Llm(LlmError::RequestFailed(format!(
                    "Failed to serialize tools: {}",
                    e
                )))
            })?;
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
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

        // Parse response
        let choices = response_body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .ok_or_else(|| {
                Error::Llm(LlmError::RequestFailed(
                    "No choices in response".to_string(),
                ))
            })?;

        let content = choices
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .map(String::from);

        Ok(LlmResponse {
            content,
            tool_calls: vec![],
            finish_reason: choices
                .get("finish_reason")
                .and_then(|f| f.as_str())
                .map(String::from),
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

/// OpenAI Provider Plugin
pub struct OpenAiProviderPlugin {
    metadata: PluginMetadata,
}

impl OpenAiProviderPlugin {
    pub fn new() -> Self {
        Self {
            metadata: clawlegion_plugin_sdk::PluginBuilder::new("openai-provider", "0.1.0")
                .description("OpenAI-compatible LLM provider plugin")
                .author("ClawLegion Team")
                .tag("llm")
                .tag("openai")
                .build(),
        }
    }

    pub fn default_metadata() -> PluginMetadata {
        clawlegion_plugin_sdk::PluginBuilder::new("openai-provider", "0.1.0")
            .description("OpenAI-compatible LLM provider plugin")
            .author("ClawLegion Team")
            .tag("llm")
            .tag("openai")
            .build()
    }
}

impl Default for OpenAiProviderPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for OpenAiProviderPlugin {
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

impl LlmProviderPlugin for OpenAiProviderPlugin {
    fn provider_type(&self) -> &str {
        "openai"
    }

    fn create_provider(
        &self,
        config: &LlmProviderConfig,
    ) -> clawlegion_plugin_sdk::Result<Arc<dyn LlmProvider>> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| {
                Error::Llm(LlmError::RequestFailed(
                    "API key not provided for OpenAI provider".to_string(),
                ))
            })?;

        let provider = OpenAiProvider::compatible(
            api_key,
            config.model.clone(),
            config
                .api_base
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        );

        Ok(Arc::new(provider))
    }
}

// Register the plugin
plugin!(OpenAiProviderPlugin);
