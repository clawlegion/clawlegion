//! Agent 系统测试

mod common;

use clawlegion_core::{
    AgentConfig, AgentInfo, AgentStatus, AgentTypeDef, HeartbeatContext, HeartbeatResult,
    HeartbeatTrigger,
};

#[test]
fn test_agent_config_creation() {
    let config = AgentConfig {
        id: common::test_agent_id(),
        company_id: common::test_company_id(),
        name: "TestAgent".to_string(),
        role: "engineer".to_string(),
        title: "高级工程师".to_string(),
        agent_type: AgentTypeDef::React,
        icon: Some("🤖".to_string()),
        reports_to: None,
        capabilities: "Coding and testing".to_string(),
        skills: vec!["rust".to_string()],
        adapter_type: "default".to_string(),
        adapter_config: Default::default(),
        runtime_config: Default::default(),
        tags: vec!["test".to_string()],
    };

    assert_eq!(config.name, "TestAgent");
    assert_eq!(config.role, "engineer");
    assert!(matches!(config.agent_type, AgentTypeDef::React));
}

#[test]
fn test_agent_info_creation() {
    let config = AgentConfig {
        id: common::test_agent_id(),
        company_id: common::test_company_id(),
        name: "TestAgent".to_string(),
        role: "manager".to_string(),
        title: "经理".to_string(),
        agent_type: AgentTypeDef::Flow,
        icon: None,
        reports_to: None,
        capabilities: "Management".to_string(),
        skills: vec![],
        adapter_type: "default".to_string(),
        adapter_config: Default::default(),
        runtime_config: Default::default(),
        tags: vec![],
    };

    let info = AgentInfo::new(config.clone());

    assert_eq!(info.config.name, "TestAgent");
    assert_eq!(info.status, AgentStatus::Initializing);
    assert!(info.last_heartbeat_at.is_none());
}

#[test]
fn test_agent_status_variants() {
    // 验证所有 AgentStatus 变体
    let statuses = [
        AgentStatus::Idle,
        AgentStatus::Running,
        AgentStatus::Paused,
        AgentStatus::Initializing,
        AgentStatus::Error,
        AgentStatus::Stopping,
    ];

    assert_eq!(statuses.len(), 6);
}

#[test]
fn test_agent_type_def_variants() {
    // 验证所有 AgentTypeDef 变体
    let react = AgentTypeDef::React;
    let flow = AgentTypeDef::Flow;
    let normal = AgentTypeDef::Normal;
    let codex = AgentTypeDef::Codex;
    let claude_code = AgentTypeDef::ClaudeCode;
    let open_code = AgentTypeDef::OpenCode;
    let custom = AgentTypeDef::Custom {
        type_name: "CustomType".to_string(),
    };

    assert!(matches!(react, AgentTypeDef::React));
    assert!(matches!(flow, AgentTypeDef::Flow));
    assert!(matches!(normal, AgentTypeDef::Normal));
    assert!(matches!(codex, AgentTypeDef::Codex));
    assert!(matches!(claude_code, AgentTypeDef::ClaudeCode));
    assert!(matches!(open_code, AgentTypeDef::OpenCode));
    assert!(matches!(custom, AgentTypeDef::Custom { .. }));
}

#[test]
fn test_agent_type_def_serde_for_builtin_and_custom() {
    let codex_json = serde_json::to_string(&AgentTypeDef::Codex).unwrap();
    assert_eq!(codex_json, "\"codex\"");

    let parsed_builtin: AgentTypeDef = serde_json::from_str("\"codex\"").unwrap();
    assert!(matches!(parsed_builtin, AgentTypeDef::Codex));

    let claude_code_json = serde_json::to_string(&AgentTypeDef::ClaudeCode).unwrap();
    assert_eq!(claude_code_json, "\"claude_code\"");
    let parsed_claude: AgentTypeDef = serde_json::from_str("\"claude_code\"").unwrap();
    assert!(matches!(parsed_claude, AgentTypeDef::ClaudeCode));

    let open_code_json = serde_json::to_string(&AgentTypeDef::OpenCode).unwrap();
    assert_eq!(open_code_json, "\"open_code\"");
    let parsed_open: AgentTypeDef = serde_json::from_str("\"open_code\"").unwrap();
    assert!(matches!(parsed_open, AgentTypeDef::OpenCode));

    let custom_json = serde_json::to_string(&AgentTypeDef::Custom {
        type_name: "plugin-backed".to_string(),
    })
    .unwrap();
    assert_eq!(
        custom_json,
        "{\"type\":\"custom\",\"type_name\":\"plugin-backed\"}"
    );

    let parsed_custom: AgentTypeDef =
        serde_json::from_str("{\"type\":\"custom\",\"type_name\":\"plugin-backed\"}").unwrap();
    assert!(matches!(
        parsed_custom,
        AgentTypeDef::Custom { ref type_name } if type_name == "plugin-backed"
    ));
}

#[test]
fn test_heartbeat_trigger_variants() {
    // Scheduled trigger
    let scheduled = HeartbeatTrigger::Scheduled;
    assert!(matches!(scheduled, HeartbeatTrigger::Scheduled));

    // PrivateMessage trigger
    let msg_id = common::test_message_id();
    let private_msg = HeartbeatTrigger::PrivateMessage { message_id: msg_id };
    assert!(matches!(
        private_msg,
        HeartbeatTrigger::PrivateMessage { .. }
    ));

    // TaskAssigned trigger
    let task_id = common::test_message_id();
    let task_assigned = HeartbeatTrigger::TaskAssigned { task_id };
    assert!(matches!(
        task_assigned,
        HeartbeatTrigger::TaskAssigned { .. }
    ));

    // ManagerAssigned trigger
    let task_id = common::test_message_id();
    let manager_id = common::test_agent_id();
    let manager_assigned = HeartbeatTrigger::ManagerAssigned {
        task_id,
        manager_id,
    };
    assert!(matches!(
        manager_assigned,
        HeartbeatTrigger::ManagerAssigned { .. }
    ));

    // Custom trigger
    let custom = HeartbeatTrigger::Custom {
        trigger_id: "custom_trigger".to_string(),
        data: serde_json::json!({"key": "value"}),
    };
    assert!(matches!(custom, HeartbeatTrigger::Custom { .. }));
}

#[test]
fn test_heartbeat_context() {
    let trigger = HeartbeatTrigger::Scheduled;
    let ctx = HeartbeatContext {
        trigger,
        timestamp: chrono::Utc::now(),
    };

    assert!(matches!(ctx.trigger, HeartbeatTrigger::Scheduled));
}

#[test]
fn test_heartbeat_result_success() {
    let result = HeartbeatResult::success();

    assert!(result.success);
    assert!(result.completed_tasks.is_empty());
    assert!(result.created_tasks.is_empty());
    assert!(result.sent_messages.is_empty());
    assert!(result.error.is_none());
}

#[test]
fn test_heartbeat_result_error() {
    let result = HeartbeatResult::error("Test error message");

    assert!(!result.success);
    assert_eq!(result.error, Some("Test error message".to_string()));
}

#[test]
fn test_heartbeat_result_with_data() {
    let task_id = common::test_message_id();
    let msg_id = common::test_message_id();

    let mut result = HeartbeatResult::success();
    result.completed_tasks.push(task_id);
    result.sent_messages.push(msg_id);

    assert_eq!(result.completed_tasks.len(), 1);
    assert_eq!(result.sent_messages.len(), 1);
}

#[test]
fn test_agent_config_with_reports_to() {
    let manager_id = common::test_agent_id();

    let config = AgentConfig {
        id: common::test_agent_id(),
        company_id: common::test_company_id(),
        name: "Subordinate".to_string(),
        role: "junior".to_string(),
        title: "初级工程师".to_string(),
        agent_type: AgentTypeDef::Normal,
        icon: None,
        reports_to: Some(manager_id),
        capabilities: "Learning".to_string(),
        skills: vec![],
        adapter_type: "default".to_string(),
        adapter_config: Default::default(),
        runtime_config: Default::default(),
        tags: vec![],
    };

    assert_eq!(config.reports_to, Some(manager_id));
}
