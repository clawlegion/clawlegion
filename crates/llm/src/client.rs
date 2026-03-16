//! LLM Client - high-level interface for LLM interactions

use clawlegion_core::{LlmMessage, LlmOptions, LlmResponse, Result};
use std::sync::Arc;

type ToolHandler = dyn Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync;
type ToolHandlerMap = std::collections::HashMap<String, Box<ToolHandler>>;

/// LLM Client
///
/// High-level client for interacting with LLM providers.
/// Provides convenient methods for common LLM operations.
pub struct LlmClient {
    provider: Arc<dyn clawlegion_core::LlmProvider>,
    default_options: LlmOptions,
    system_prompt: Option<String>,
    conversation_history: Vec<LlmMessage>,
    max_history: usize,
}

impl LlmClient {
    /// Create a new LLM client
    pub fn new(provider: Arc<dyn clawlegion_core::LlmProvider>) -> Self {
        Self {
            provider,
            default_options: LlmOptions::default(),
            system_prompt: None,
            conversation_history: Vec::new(),
            max_history: 20,
        }
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set default options
    pub fn with_options(mut self, options: LlmOptions) -> Self {
        self.default_options = options;
        self
    }

    /// Set max conversation history
    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    /// Get the model name
    pub fn model_name(&self) -> &str {
        self.provider.model()
    }

    /// Send a simple message
    pub async fn ask(&self, message: impl Into<String>) -> Result<LlmResponse> {
        let messages = self.build_messages(vec![LlmMessage::user(message)]);
        self.provider
            .chat(messages, self.default_options.clone())
            .await
    }

    /// Send a message with context
    pub async fn ask_with_context(
        &self,
        message: impl Into<String>,
        context: &[String],
    ) -> Result<LlmResponse> {
        let mut messages = Vec::new();

        // Add context as system message
        if !context.is_empty() {
            let context_text = context.join("\n\n");
            messages.push(LlmMessage::system(format!(
                "Context information:\n{}",
                context_text
            )));
        }

        // Add system prompt if set
        if let Some(ref prompt) = self.system_prompt {
            if messages.is_empty() {
                messages.push(LlmMessage::system(prompt.clone()));
            }
        }

        messages.push(LlmMessage::user(message));

        let messages = self.build_messages(messages);
        self.provider
            .chat(messages, self.default_options.clone())
            .await
    }

    /// Send messages in a conversation (maintains history)
    pub async fn chat(&mut self, message: impl Into<String>) -> Result<LlmResponse> {
        // Add user message to history
        self.conversation_history.push(LlmMessage::user(message));

        // Build messages with history
        let messages = self.build_messages(self.conversation_history.clone());

        // Get response
        let response = self
            .provider
            .chat(messages, self.default_options.clone())
            .await?;

        // Add assistant response to history
        if let Some(ref content) = response.content {
            self.conversation_history
                .push(LlmMessage::assistant(content.clone()));

            // Trim history if needed
            while self.conversation_history.len() > self.max_history {
                // Remove oldest non-system message
                if let Some(pos) = self
                    .conversation_history
                    .iter()
                    .position(|m| m.role != clawlegion_core::MessageRole::System)
                {
                    self.conversation_history.remove(pos);
                }
            }
        }

        Ok(response)
    }

    /// Clear conversation history
    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }

    /// Get conversation history
    pub fn history(&self) -> &[LlmMessage] {
        &self.conversation_history
    }

    /// Send a message with tools
    pub async fn chat_with_tools(
        &self,
        message: impl Into<String>,
        tools: Vec<clawlegion_core::ToolDefinition>,
    ) -> Result<LlmResponse> {
        let mut options = self.default_options.clone();
        options.tools = Some(tools);

        let messages = self.build_messages(vec![LlmMessage::user(message)]);
        self.provider.chat(messages, options).await
    }

    /// Execute a tool call loop
    pub async fn execute_tool_loop(
        &self,
        message: impl Into<String>,
        tools: &ToolHandlerMap,
    ) -> Result<LlmResponse> {
        let mut messages = self.build_messages(vec![LlmMessage::user(message)]);

        let mut max_iterations = 10;

        loop {
            if max_iterations == 0 {
                return Err(clawlegion_core::Error::Llm(
                    clawlegion_core::LlmError::RequestFailed(
                        "Max tool call iterations reached".to_string(),
                    ),
                ));
            }
            max_iterations -= 1;

            // Build tool definitions
            let tool_defs: Vec<clawlegion_core::ToolDefinition> = tools
                .keys()
                .map(|name| clawlegion_core::ToolDefinition {
                    name: name.clone(),
                    description: "Tool".to_string(),
                    parameters: serde_json::json!({"type": "object"}),
                })
                .collect();

            let mut options = self.default_options.clone();
            options.tools = Some(tool_defs);

            let response = self.provider.chat(messages.clone(), options).await?;

            // Check if there are tool calls
            if response.tool_calls.is_empty() {
                return Ok(response);
            }

            // Execute tool calls
            for tool_call in &response.tool_calls {
                if let Some(tool_fn) = tools.get(&tool_call.name) {
                    let result = tool_fn(serde_json::to_value(&tool_call.arguments).unwrap())?;

                    messages.push(LlmMessage {
                        role: clawlegion_core::MessageRole::Assistant,
                        content: format!("Calling tool: {}", tool_call.name),
                        name: None,
                        tool_call_id: None,
                    });

                    messages.push(LlmMessage {
                        role: clawlegion_core::MessageRole::Tool,
                        content: serde_json::to_string(&result).unwrap_or_default(),
                        name: Some(tool_call.name.clone()),
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
            }
        }
    }

    /// Count tokens for a message
    pub async fn count_tokens(&self, message: &str) -> Result<u64> {
        let messages = vec![LlmMessage::user(message)];
        self.provider.count_tokens(&messages).await
    }

    /// Build messages with system prompt
    fn build_messages(&self, mut messages: Vec<LlmMessage>) -> Vec<LlmMessage> {
        // Add system prompt if set and no system message exists
        if let Some(ref prompt) = self.system_prompt {
            if !messages
                .iter()
                .any(|m| m.role == clawlegion_core::MessageRole::System)
            {
                messages.insert(0, LlmMessage::system(prompt.clone()));
            }
        }

        messages
    }
}

/// Builder for LLM clients
pub struct LlmClientBuilder {
    provider: Option<Arc<dyn clawlegion_core::LlmProvider>>,
    system_prompt: Option<String>,
    options: LlmOptions,
    max_history: usize,
}

impl LlmClientBuilder {
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            options: LlmOptions::default(),
            max_history: 20,
        }
    }

    pub fn provider(mut self, provider: Arc<dyn clawlegion_core::LlmProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn options(mut self, options: LlmOptions) -> Self {
        self.options = options;
        self
    }

    pub fn max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    pub fn build(self) -> Option<LlmClient> {
        self.provider.map(|provider| {
            let mut client = LlmClient::new(provider);
            client.system_prompt = self.system_prompt;
            client.default_options = self.options;
            client.max_history = self.max_history;
            client
        })
    }
}

impl Default for LlmClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
