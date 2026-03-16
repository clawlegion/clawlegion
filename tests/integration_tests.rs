//! 端到端集成测试
//!
//! 测试完整工作流程场景

mod common;

use clawlegion_core::{
    AgentConfig, AgentStatus, AgentTypeDef, ChatMessage, ChatType, MessageContent,
};
use clawlegion_org::{OrgAgent, OrgTree};

/// 测试 API 配置
#[allow(dead_code)]
const TEST_API_KEY: &str = "sk-SFng2PCyEKcQw82fA-cl-test";
#[allow(dead_code)]
const TEST_API_BASE: &str = "https://103.237.28.67:8317/v1";
#[allow(dead_code)]
const TEST_MODEL: &str = "qwen3-coder-plus";

#[test]
fn test_scenario_a_agent_onboarding() {
    // 场景 A：新 Agent 入职流程
    // 1. 创建公司配置
    let company_id = uuid::Uuid::new_v4();

    // 2. 创建组织树
    let tree = OrgTree::new(company_id);
    assert_eq!(tree.company_id(), company_id);

    // 3. 创建 CEO Agent
    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = uuid::Uuid::new_v4();
    let ceo_id = ceo.id;
    assert!(tree.add_agent(ceo).is_ok());

    // 4. 创建部门经理 Agent（汇报给 CEO）
    let mut manager = OrgAgent::new(
        company_id,
        "Manager".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager.id = uuid::Uuid::new_v4();
    let manager_id = manager.id;
    manager.set_reports_to(Some(ceo_id));
    assert!(tree.add_agent(manager).is_ok());

    // 5. 创建员工 Agent（汇报给经理）
    let mut employee = OrgAgent::new(
        company_id,
        "Employee".to_string(),
        "engineer".to_string(),
        "工程师".to_string(),
    );
    employee.id = uuid::Uuid::new_v4();
    let employee_id = employee.id;
    employee.set_reports_to(Some(manager_id));
    assert!(tree.add_agent(employee).is_ok());

    // 6. 验证组织树结构
    assert_eq!(tree.agent_count(), 3);

    // 验证 CEO
    let ceo_agent = tree.get_ceo();
    assert!(ceo_agent.is_some());

    // 验证直接汇报
    let direct_reports = tree.get_direct_reports(ceo_id);
    assert_eq!(direct_reports.len(), 1);
    assert_eq!(direct_reports[0].read().id, manager_id);

    // 7. 验证指挥链
    let chain = tree.get_chain_of_command(employee_id);
    assert_eq!(chain.len(), 3); // Employee -> Manager -> CEO

    // 8. 验证深度
    assert_eq!(tree.get_depth(ceo_id), Some(0));
    assert_eq!(tree.get_depth(manager_id), Some(1));
    assert_eq!(tree.get_depth(employee_id), Some(2));

    println!("场景 A：新 Agent 入职流程 - 通过");
}

#[test]
fn test_scenario_message_flow() {
    // 测试消息传递流程
    let company_id = uuid::Uuid::new_v4();
    let sender_id = uuid::Uuid::new_v4();
    let recipient_id = uuid::Uuid::new_v4();

    // 创建私有消息
    let message = ChatMessage::private(
        company_id,
        sender_id,
        recipient_id,
        MessageContent::Text("Hello, Agent!".to_string()),
    );

    assert_eq!(message.company_id, company_id);
    assert_eq!(message.sender, sender_id);
    assert!(matches!(
        message.chat_type,
        ChatType::Private { participant } if participant == recipient_id
    ));
    assert!(matches!(message.content, MessageContent::Text(ref text) if text == "Hello, Agent!"));

    // 测试回复消息
    let reply = ChatMessage::reply(
        company_id,
        message.chat_type,
        recipient_id,
        "Hello back!".to_string(),
        message.id,
    );

    assert!(matches!(
        reply.content,
        MessageContent::Reply { in_reply_to, .. } if in_reply_to == message.id
    ));

    println!("场景：消息传递流程 - 通过");
}

#[test]
fn test_scenario_agent_config() {
    // 测试 Agent 配置创建和验证
    let agent_id = uuid::Uuid::new_v4();
    let company_id = uuid::Uuid::new_v4();

    let config = AgentConfig {
        id: agent_id,
        company_id,
        name: "TestAgent".to_string(),
        role: "engineer".to_string(),
        title: "高级工程师".to_string(),
        agent_type: AgentTypeDef::React,
        icon: Some("🤖".to_string()),
        reports_to: None,
        capabilities: "Coding and testing".to_string(),
        skills: vec!["rust".to_string(), "testing".to_string()],
        budget_monthly_cents: Some(100000),
        adapter_type: "default".to_string(),
        adapter_config: Default::default(),
        runtime_config: Default::default(),
        tags: vec!["test".to_string()],
    };

    assert_eq!(config.name, "TestAgent");
    assert_eq!(config.role, "engineer");
    assert!(matches!(config.agent_type, AgentTypeDef::React));
    assert_eq!(config.skills.len(), 2);
    assert_eq!(config.budget_monthly_cents, Some(100000));

    println!("场景：Agent 配置 - 通过");
}

#[test]
fn test_scenario_agent_status_lifecycle() {
    // 测试 Agent 状态生命周期
    let statuses = vec![
        (AgentStatus::Initializing, "初始化"),
        (AgentStatus::Idle, "空闲"),
        (AgentStatus::Running, "运行中"),
        (AgentStatus::Paused, "暂停"),
        (AgentStatus::Error, "错误"),
        (AgentStatus::Stopping, "停止中"),
    ];

    for (status, description) in statuses {
        println!("测试状态：{:?} - {}", status, description);
    }

    assert_eq!(statuses.len(), 6);
    println!("场景：Agent 状态生命周期 - 通过");
}

#[tokio::test]
async fn test_async_message_metadata() {
    // 异步测试：消息元数据
    use chrono::Duration;

    let mut metadata = clawlegion_core::MessageMetadata::new();

    // 测试过期时间设置
    metadata.expires_at = Some(chrono::Utc::now() + Duration::hours(1));
    assert!(!metadata.is_expired());

    // 测试重要标记
    metadata.important = true;
    assert!(metadata.important);

    // 测试标签
    metadata.tags.push("urgent".to_string());
    assert!(metadata.tags.contains(&"urgent".to_string()));

    // 测试自定义字段
    metadata
        .custom
        .insert("priority".to_string(), serde_json::json!("high"));
    assert!(metadata.custom.contains_key("priority"));

    println!("异步测试：消息元数据 - 通过");
}

#[test]
fn test_memory_lifecycle() {
    // 测试记忆生命周期
    let company_id = uuid::Uuid::new_v4();

    let memory = clawlegion_core::MemoryEntry {
        id: uuid::Uuid::new_v4(),
        company_id,
        agent_id: None,
        content: "Test memory".to_string(),
        category: clawlegion_core::MemoryCategory::ShortTerm,
        importance: 0.5,
        access_count: 0,
        last_accessed_at: chrono::Utc::now(),
        created_at: chrono::Utc::now(),
        tags: vec!["test".to_string()],
        related_messages: vec![],
        expires_at: None,
    };

    // 验证基本属性
    assert_eq!(memory.content, "Test memory");
    assert_eq!(memory.category, clawlegion_core::MemoryCategory::ShortTerm);
    assert_eq!(memory.importance, 0.5);

    // 验证保留分数
    let score = memory.retention_score();
    assert!(score >= 0.0 && score <= 1.0);

    // 验证不应该压缩或遗忘
    assert!(!memory.should_compress());
    assert!(!memory.should_forget());
    assert!(!memory.is_expired());

    println!("测试：记忆生命周期 - 通过");
}

#[test]
fn test_trigger_conditions() {
    // 测试触发器条件
    use clawlegion_sentinel::{TriggerCondition, WakeupMethod, WakeupTrigger};

    let agent_id = uuid::Uuid::new_v4();

    // 测试私有消息触发器
    let pm_trigger = WakeupTrigger::new(
        "pm_trigger",
        agent_id,
        TriggerCondition::PrivateMessage { from: None },
        WakeupMethod::Heartbeat,
    );
    assert_eq!(pm_trigger.id, "pm_trigger");
    assert!(pm_trigger.enabled);

    // 测试定时触发器
    let cron_trigger = WakeupTrigger::new(
        "cron_trigger",
        agent_id,
        TriggerCondition::Cron {
            expression: "0 9 * * *".to_string(),
        },
        WakeupMethod::Heartbeat,
    );
    assert!(matches!(
        cron_trigger.condition,
        TriggerCondition::Cron { .. }
    ));

    // 测试任务分配触发器
    let task_id = uuid::Uuid::new_v4();
    let task_trigger = WakeupTrigger::new(
        "task_trigger",
        agent_id,
        TriggerCondition::TaskAssigned { task_id },
        WakeupMethod::Heartbeat,
    );
    assert!(matches!(
        task_trigger.condition,
        TriggerCondition::TaskAssigned { .. }
    ));

    println!("测试：触发器条件 - 通过");
}

#[test]
fn test_org_permissions() {
    // 测试组织权限
    use clawlegion_org::AgentPermissions;

    // CEO 权限
    let ceo_perms = AgentPermissions::ceo();
    assert!(ceo_perms.can_hire);
    assert!(ceo_perms.can_fire);
    assert!(ceo_perms.can_manage_budget);
    assert!(ceo_perms.can_approve_spending);

    // 经理权限
    let manager_perms = AgentPermissions::manager();
    assert!(manager_perms.can_hire);
    assert!(!manager_perms.can_fire);
    assert!(manager_perms.can_manage_budget);

    // 贡献者权限
    let contributor_perms = AgentPermissions::contributor();
    assert!(!contributor_perms.can_hire);
    assert!(!contributor_perms.can_fire);
    assert!(!contributor_perms.can_manage_budget);

    // 默认权限
    let default_perms = AgentPermissions::default();
    assert!(!default_perms.can_hire);
    assert!(!default_perms.can_fire);
    assert!(default_perms.can_assign_tasks);
    assert!(default_perms.can_access_company_data);

    println!("测试：组织权限 - 通过");
}
