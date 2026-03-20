use clawlegion_core::{LlmMessage, LlmOptions, LlmResponse, LlmUsage};

struct MockClient;

impl MockClient {
    fn model(&self) -> &str {
        "mock-model"
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
            usage: LlmUsage::default(),
        })
    }
}

#[tokio::test]
async fn test_llm_client_creation() {
    let client = MockClient;
    assert_eq!(client.model(), "mock-model");
}

#[tokio::test]
async fn test_llm_client_with_system_prompt() {
    let client = MockClient;
    let response = client
        .chat(Vec::new(), LlmOptions::default())
        .await
        .expect("mock response should succeed");

    assert_eq!(response.content.as_deref(), Some("Test response"));
    assert_eq!(response.finish_reason.as_deref(), Some("stop"));
    assert_eq!(response.usage.total_tokens, 0);
}
