//! Agent Registry 测试

mod common;

use clawlegion_agent::{AgentFactory, AgentRegistry, BaseAgent};
use clawlegion_core::{AgentConfig, AgentTypeDef};
use std::sync::Arc;

#[test]
fn test_registry_creation() {
    let registry = AgentRegistry::new();
    assert_eq!(registry.count(), 0);
}

#[test]
fn test_registry_register_agent() {
    let registry = AgentRegistry::new();

    let config = create_test_agent_config("TestAgent");
    let capabilities = Arc::new(TestCapabilities);
    let agent = BaseAgent::new(config, capabilities);

    let result = registry.register(Box::new(agent));
    assert!(result.is_ok());
    assert_eq!(registry.count(), 1);
}

#[test]
fn test_registry_duplicate_agent_fails() {
    let registry = AgentRegistry::new();

    let config = create_test_agent_config("TestAgent");
    let capabilities = Arc::new(TestCapabilities);

    let agent1 = BaseAgent::new(config.clone(), capabilities.clone());
    let agent2 = BaseAgent::new(config, capabilities);

    // 第一次注册应该成功
    assert!(registry.register(Box::new(agent1)).is_ok());

    // 第二次注册相同 ID 应该失败
    let result = registry.register(Box::new(agent2));
    assert!(result.is_err());
}

#[test]
fn test_registry_get_agent() {
    let registry = AgentRegistry::new();

    let config = create_test_agent_config("TestAgent");
    let agent_id = config.id;
    let capabilities = Arc::new(TestCapabilities);
    let agent = BaseAgent::new(config, capabilities);

    registry.register(Box::new(agent)).unwrap();

    let retrieved = registry.get(agent_id);
    assert!(retrieved.is_some());
}

#[test]
fn test_registry_get_info() {
    let registry = AgentRegistry::new();

    let config = create_test_agent_config("TestAgent");
    let agent_id = config.id;
    let capabilities = Arc::new(TestCapabilities);
    let agent = BaseAgent::new(config, capabilities);

    registry.register(Box::new(agent)).unwrap();

    let info = registry.get_info(agent_id);
    assert!(info.is_some());
    assert_eq!(info.unwrap().config.name, "TestAgent");
}

#[test]
fn test_registry_list_agents() {
    let registry = AgentRegistry::new();

    // 添加多个 agent
    for i in 0..3 {
        let mut config = create_test_agent_config(&format!("Agent{}", i));
        config.id = uuid::Uuid::new_v4();
        let capabilities = Arc::new(TestCapabilities);
        let agent = BaseAgent::new(config, capabilities);
        registry.register(Box::new(agent)).unwrap();
    }

    let agents = registry.list_agents();
    assert_eq!(agents.len(), 3);
}

#[test]
fn test_registry_unregister() {
    let registry = AgentRegistry::new();

    let config = create_test_agent_config("TestAgent");
    let agent_id = config.id;
    let capabilities = Arc::new(TestCapabilities);
    let agent = BaseAgent::new(config, capabilities);

    registry.register(Box::new(agent)).unwrap();
    assert_eq!(registry.count(), 1);

    let result = registry.unregister(agent_id);
    assert!(result.is_ok());
    assert_eq!(registry.count(), 0);
}

#[test]
fn test_registry_has_agent() {
    let registry = AgentRegistry::new();

    let config = create_test_agent_config("TestAgent");
    let agent_id = config.id;
    let capabilities = Arc::new(TestCapabilities);
    let agent = BaseAgent::new(config, capabilities);

    registry.register(Box::new(agent)).unwrap();

    assert!(registry.has_agent(agent_id));

    registry.unregister(agent_id).unwrap();
    assert!(!registry.has_agent(agent_id));
}

#[test]
fn test_registry_clear() {
    let registry = AgentRegistry::new();

    // 添加多个 agent
    for i in 0..3 {
        let mut config = create_test_agent_config(&format!("Agent{}", i));
        config.id = uuid::Uuid::new_v4();
        let capabilities = Arc::new(TestCapabilities);
        let agent = BaseAgent::new(config, capabilities);
        registry.register(Box::new(agent)).unwrap();
    }

    assert_eq!(registry.count(), 3);

    registry.clear();
    assert_eq!(registry.count(), 0);
}

#[test]
fn test_factory_creates_codex_agent() {
    let mut config = create_test_agent_config("CodexAgent");
    config.agent_type = AgentTypeDef::Codex;
    config.adapter_config = serde_json::from_value(serde_json::json!({
        "llm_provider": {
            "provider": "openai",
            "model": "gpt-5-codex",
            "api_key": "test-key",
            "api_base": "https://example.com/v1",
            "organization": null,
            "timeout_secs": null,
            "extra": {
                "env_key": "OPENAI_API_KEY"
            }
        }
    }))
    .unwrap();

    let capabilities = Arc::new(TestCapabilities);
    let agent = AgentFactory::create_agent(&config, capabilities).unwrap();

    assert_eq!(agent.id(), config.id);
    assert!(matches!(
        agent.info().config.agent_type,
        AgentTypeDef::Codex
    ));
}

#[test]
fn test_factory_creates_claude_code_agent() {
    let mut config = create_test_agent_config("ClaudeCodeAgent");
    config.agent_type = AgentTypeDef::ClaudeCode;
    config.adapter_config = serde_json::from_value(serde_json::json!({
        "llm_provider": {
            "provider": "anthropic",
            "model": "claude-sonnet-4.5",
            "api_key": "test-key",
            "api_base": "https://example.com/v1",
            "organization": null,
            "timeout_secs": null,
            "extra": {
                "env_key": "ANTHROPIC_API_KEY"
            }
        }
    }))
    .unwrap();

    let capabilities = Arc::new(TestCapabilities);
    let agent = AgentFactory::create_agent(&config, capabilities).unwrap();

    assert_eq!(agent.id(), config.id);
    assert!(matches!(
        agent.info().config.agent_type,
        AgentTypeDef::ClaudeCode
    ));
}

#[test]
fn test_factory_creates_open_code_agent() {
    let mut config = create_test_agent_config("OpenCodeAgent");
    config.agent_type = AgentTypeDef::OpenCode;
    config.adapter_config = serde_json::from_value(serde_json::json!({
        "llm_provider": {
            "provider": "openai",
            "model": "gpt-5-codex",
            "api_key": "test-key",
            "api_base": "https://example.com/v1",
            "organization": null,
            "timeout_secs": null,
            "extra": {
                "env_key": "OPENAI_API_KEY"
            }
        }
    }))
    .unwrap();

    let capabilities = Arc::new(TestCapabilities);
    let agent = AgentFactory::create_agent(&config, capabilities).unwrap();

    assert_eq!(agent.id(), config.id);
    assert!(matches!(
        agent.info().config.agent_type,
        AgentTypeDef::OpenCode
    ));
}

fn create_test_agent_config(name: &str) -> AgentConfig {
    AgentConfig {
        id: uuid::Uuid::new_v4(),
        company_id: uuid::Uuid::new_v4(),
        name: name.to_string(),
        role: "test".to_string(),
        title: "测试工程师".to_string(),
        agent_type: AgentTypeDef::React,
        icon: None,
        reports_to: None,
        capabilities: "Testing".to_string(),
        skills: vec![],
        adapter_type: "default".to_string(),
        adapter_config: Default::default(),
        runtime_config: Default::default(),
        tags: vec![],
    }
}

// 测试用 Capabilities 实现
struct TestCapabilities;

#[async_trait::async_trait]
impl clawlegion_agent::AgentCapabilities for TestCapabilities {
    async fn execute_heartbeat(
        &self,
        _agent: &BaseAgent,
        _ctx: &clawlegion_core::HeartbeatContext,
    ) -> clawlegion_core::Result<clawlegion_core::HeartbeatResult> {
        Ok(clawlegion_core::HeartbeatResult::success())
    }
}
