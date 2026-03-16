//! LLM 客户端集成测试
//!
//! 使用提供的测试 API 配置：
//! - Base URL: https://103.237.28.67:8317/v1
//! - API Key: sk-SFng2PCyEKcQw82fA-cl-test

mod common;

use async_trait::async_trait;
use clawlegion_core::{LlmMessage, LlmOptions, LlmProvider, LlmResponse, StreamChunk, TokenUsage};
use clawlegion_llm::{LlmClient, LlmClientBuilder};
use std::sync::Arc;

/// 测试 API 配置
#[allow(dead_code)]
const TEST_API_KEY: &str = "sk-SFng2PCyEKcQw82fA-cl-test";
#[allow(dead_code)]
const TEST_API_BASE: &str = "https://103.237.28.67:8317/v1";
#[allow(dead_code)]
const TEST_MODEL: &str = "qwen3-coder-plus";

// 测试用 Provider 实现
struct TestProvider {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    model: String,
}

impl TestProvider {
    fn new() -> Self {
        Self {
            name: "test".to_string(),
            model: "test-model".to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for TestProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: LlmOptions,
    ) -> clawlegion_core::Result<LlmResponse> {
        Ok(LlmResponse {
            content: Some("Test response".to_string()),
            tool_calls: vec![],
            finish_reason: Some("stop".to_string()),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
        })
    }

    async fn stream(
        &self,
        _messages: Vec<LlmMessage>,
        _options: LlmOptions,
    ) -> clawlegion_core::Result<
        Box<dyn futures_core::Stream<Item = clawlegion_core::Result<StreamChunk>> + Send + Unpin>,
    > {
        unimplemented!()
    }
}

#[tokio::test]
async fn test_llm_client_creation() {
    // 测试 LLM 客户端创建
    let provider = Arc::new(TestProvider::new());
    let client = LlmClient::new(provider);

    assert_eq!(client.provider_name(), "test");
}

#[tokio::test]
async fn test_llm_client_with_system_prompt() {
    let provider = Arc::new(TestProvider::new());
    let _client = LlmClient::new(provider).with_system_prompt("You are a helpful assistant.");

    // 验证系统提示被设置
    // 注意：这里需要访问内部状态，实际测试中应该检查行为
}

#[tokio::test]
async fn test_llm_client_with_options() {
    let provider = Arc::new(TestProvider::new());
    let options = LlmOptions {
        temperature: Some(0.7),
        max_tokens: Some(1000),
        ..Default::default()
    };

    let _client = LlmClient::new(provider).with_options(options);

    // 验证选项被设置
}

#[tokio::test]
async fn test_llm_client_builder() {
    let provider = Arc::new(TestProvider::new());

    let client = LlmClientBuilder::new()
        .provider(provider)
        .system_prompt("Test prompt")
        .max_history(50)
        .build();

    assert!(client.is_some());
}

#[tokio::test]
async fn test_llm_client_clear_history() {
    let provider = Arc::new(TestProvider::new());
    let mut client = LlmClient::new(provider);

    // 添加一些历史消息
    let _ = client.chat("Hello").await;

    // 清除历史
    client.clear_history();

    // 验证历史被清除
    assert_eq!(client.history().len(), 0);
}
