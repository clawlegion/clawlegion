//! 组织配置测试

use clawlegion_core::AgentTypeDef;
use clawlegion_org::OrgConfig;

#[test]
fn test_org_config_parses_codex_agent_type() {
    let toml = r#"
[company]
name = "Test Company"
issue_prefix = "TST"

[[agents]]
name = "Codex Agent"
role = "engineer"
title = "工程师"
agent_type = "codex"
capabilities = "Write and review code"
adapter_type = "codex"

[agents.adapter_config.llm_provider]
provider = "openai"
model = "gpt-5-codex"
api_key = "test-key"
api_base = "https://example.com/v1"
"#;

    let config: OrgConfig = toml::from_str(toml).unwrap();
    let company_id = uuid::Uuid::new_v4();
    let agents = config.to_agents(company_id).unwrap();

    assert_eq!(agents.len(), 1);
    assert!(matches!(config.agents[0].agent_type, AgentTypeDef::Codex));
    assert!(matches!(agents[0].agent_type, AgentTypeDef::Codex));
}

#[test]
fn test_org_config_parses_claude_code_agent_type() {
    let toml = r#"
[company]
name = "Test Company"
issue_prefix = "TST"

[[agents]]
name = "Claude Code Agent"
role = "engineer"
title = "工程师"
agent_type = "claude_code"
capabilities = "Write and review code"
adapter_type = "claude_code"

[agents.adapter_config.llm_provider]
provider = "anthropic"
model = "claude-sonnet-4.5"
api_key = "test-key"
api_base = "https://example.com/v1"
"#;

    let config: OrgConfig = toml::from_str(toml).unwrap();
    let company_id = uuid::Uuid::new_v4();
    let agents = config.to_agents(company_id).unwrap();

    assert_eq!(agents.len(), 1);
    assert!(matches!(
        config.agents[0].agent_type,
        AgentTypeDef::ClaudeCode
    ));
    assert!(matches!(agents[0].agent_type, AgentTypeDef::ClaudeCode));
    assert_eq!(
        agents[0]
            .adapter_config
            .get("llm_provider")
            .and_then(|value| value.get("model"))
            .and_then(|value| value.as_str()),
        Some("claude-sonnet-4.5")
    );
}

#[test]
fn test_org_config_parses_open_code_agent_type() {
    let toml = r#"
[company]
name = "Test Company"
issue_prefix = "TST"

[[agents]]
name = "Open Code Agent"
role = "engineer"
title = "工程师"
agent_type = "open_code"
capabilities = "Write and review code"
adapter_type = "open_code"

[agents.adapter_config.llm_provider]
provider = "openai"
model = "gpt-5-codex"
api_key = "test-key"
api_base = "https://example.com/v1"
"#;

    let config: OrgConfig = toml::from_str(toml).unwrap();
    let company_id = uuid::Uuid::new_v4();
    let agents = config.to_agents(company_id).unwrap();

    assert_eq!(agents.len(), 1);
    assert!(matches!(
        config.agents[0].agent_type,
        AgentTypeDef::OpenCode
    ));
    assert!(matches!(agents[0].agent_type, AgentTypeDef::OpenCode));
    assert_eq!(
        agents[0]
            .adapter_config
            .get("llm_provider")
            .and_then(|value| value.get("model"))
            .and_then(|value| value.as_str()),
        Some("gpt-5-codex")
    );
}
