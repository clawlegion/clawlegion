//! Sentinel 触发器测试

mod common;

use clawlegion_sentinel::{
    CompoundOp, TriggerCondition, TriggerEvaluation, WakeupMethod, WakeupTrigger,
};

#[test]
fn test_wakeup_trigger_creation() {
    let agent_id = common::test_agent_id();
    let condition = TriggerCondition::Cron {
        expression: "0 * * * *".to_string(),
    };

    let trigger = WakeupTrigger::new("test_trigger", agent_id, condition, WakeupMethod::Heartbeat);

    assert_eq!(trigger.id, "test_trigger");
    assert_eq!(trigger.agent_id, agent_id);
    assert!(trigger.enabled);
    assert_eq!(trigger.cooldown_secs, 60); // 默认值
}

#[test]
fn test_wakeup_trigger_with_priority() {
    let agent_id = common::test_agent_id();

    let trigger = WakeupTrigger::new(
        "priority_trigger",
        agent_id,
        TriggerCondition::Cron {
            expression: "0 * * * *".to_string(),
        },
        WakeupMethod::Heartbeat,
    )
    .with_priority(100);

    assert_eq!(trigger.priority, 100);
}

#[test]
fn test_wakeup_trigger_with_cooldown() {
    let agent_id = common::test_agent_id();

    let trigger = WakeupTrigger::new(
        "cooldown_trigger",
        agent_id,
        TriggerCondition::Cron {
            expression: "0 * * * *".to_string(),
        },
        WakeupMethod::Heartbeat,
    )
    .with_cooldown(300);

    assert_eq!(trigger.cooldown_secs, 300);
}

#[test]
fn test_trigger_is_in_cooldown() {
    let agent_id = common::test_agent_id();
    let mut trigger = WakeupTrigger::new(
        "test",
        agent_id,
        TriggerCondition::Cron {
            expression: "0 * * * *".to_string(),
        },
        WakeupMethod::Heartbeat,
    )
    .with_cooldown(60);

    // 未触发时不在冷却中
    assert!(!trigger.is_in_cooldown());

    // 标记为已触发
    trigger.mark_triggered();

    // 现在应该在冷却中
    assert!(trigger.is_in_cooldown());
}

#[test]
fn test_trigger_cooldown_elapsed() {
    let agent_id = common::test_agent_id();
    let mut trigger = WakeupTrigger::new(
        "test",
        agent_id,
        TriggerCondition::Cron {
            expression: "0 * * * *".to_string(),
        },
        WakeupMethod::Heartbeat,
    )
    .with_cooldown(1); // 1 秒冷却

    trigger.mark_triggered();

    // 等待超过冷却时间
    std::thread::sleep(std::time::Duration::from_secs(2));

    assert!(!trigger.is_in_cooldown());
}

#[test]
fn test_trigger_condition_private_message() {
    let from_agent = common::test_agent_id();

    let condition = TriggerCondition::PrivateMessage {
        from: Some(from_agent),
    };

    assert!(matches!(
        condition,
        TriggerCondition::PrivateMessage { from: Some(id) } if id == from_agent
    ));
}

#[test]
fn test_trigger_condition_task_assigned() {
    let task_id = common::test_message_id();

    let condition = TriggerCondition::TaskAssigned { task_id };

    assert!(matches!(
        condition,
        TriggerCondition::TaskAssigned { task_id: id } if id == task_id
    ));
}

#[test]
fn test_trigger_condition_manager_assigned() {
    let manager_id = common::test_agent_id();
    let condition = TriggerCondition::ManagerAssigned { manager_id };

    assert!(matches!(
        condition,
        TriggerCondition::ManagerAssigned { manager_id: id } if id == manager_id
    ));
}

#[test]
fn test_trigger_condition_cron() {
    let condition = TriggerCondition::Cron {
        expression: "0 9 * * *".to_string(),
    };

    assert!(matches!(
        condition,
        TriggerCondition::Cron { expression } if expression == "0 9 * * *"
    ));
}

#[test]
fn test_trigger_condition_polling() {
    let condition = TriggerCondition::Polling {
        interval_secs: 30,
        checker: "health_check".to_string(),
        params: serde_json::json!({"endpoint": "/api/health"}),
    };

    assert!(matches!(condition, TriggerCondition::Polling { .. }));
}

#[test]
fn test_trigger_condition_stream() {
    let condition = TriggerCondition::Stream {
        url: "wss://example.com/stream".to_string(),
        filter: Some("type=message".to_string()),
    };

    assert!(matches!(condition, TriggerCondition::Stream { .. }));
}

#[test]
fn test_trigger_condition_webhook() {
    let condition = TriggerCondition::Webhook {
        secret: Some("secret123".to_string()),
        schema: Some(serde_json::json!({"type": "object"})),
    };

    assert!(matches!(condition, TriggerCondition::Webhook { .. }));
}

#[test]
fn test_trigger_condition_custom() {
    let condition = TriggerCondition::Custom {
        name: "custom_check".to_string(),
        params: serde_json::json!({"threshold": 100}),
    };

    assert!(matches!(condition, TriggerCondition::Custom { .. }));
}

#[test]
fn test_trigger_condition_compound_and() {
    let conditions = vec![
        TriggerCondition::Cron {
            expression: "0 * * * *".to_string(),
        },
        TriggerCondition::PrivateMessage { from: None },
    ];

    let compound = TriggerCondition::Compound {
        op: CompoundOp::And,
        conditions,
    };

    assert!(matches!(
        compound,
        TriggerCondition::Compound {
            op: CompoundOp::And,
            ..
        }
    ));
}

#[test]
fn test_trigger_condition_compound_or() {
    let conditions = vec![
        TriggerCondition::Cron {
            expression: "0 * * * *".to_string(),
        },
        TriggerCondition::TaskAssigned {
            task_id: common::test_message_id(),
        },
    ];

    let compound = TriggerCondition::Compound {
        op: CompoundOp::Or,
        conditions,
    };

    assert!(matches!(
        compound,
        TriggerCondition::Compound {
            op: CompoundOp::Or,
            ..
        }
    ));
}

#[test]
fn test_trigger_condition_direct_call() {
    let condition = TriggerCondition::DirectCall {
        caller: "api".to_string(),
        params: serde_json::json!({"agent_id": common::test_agent_id().to_string()}),
    };

    assert!(matches!(condition, TriggerCondition::DirectCall { .. }));
}

#[test]
fn test_wakeup_method_send_message() {
    let method = WakeupMethod::SendMessage {
        content: "Hello, Agent!".to_string(),
    };

    assert!(matches!(method, WakeupMethod::SendMessage { content } if content == "Hello, Agent!"));
}

#[test]
fn test_wakeup_method_execute_skill() {
    let method = WakeupMethod::ExecuteSkill {
        skill: "research".to_string(),
        input: serde_json::json!({"query": "test"}),
    };

    assert!(matches!(method, WakeupMethod::ExecuteSkill { .. }));
}

#[test]
fn test_trigger_evaluation_met() {
    let eval = TriggerEvaluation::met("test_trigger");

    assert!(eval.condition_met);
    assert_eq!(eval.trigger_id, "test_trigger");
    assert!(eval.details.is_none());
}

#[test]
fn test_trigger_evaluation_not_met() {
    let eval = TriggerEvaluation::not_met("test_trigger");

    assert!(!eval.condition_met);
    assert_eq!(eval.trigger_id, "test_trigger");
}

#[test]
fn test_trigger_evaluation_with_details() {
    let eval = TriggerEvaluation::met("test_trigger").with_details("Condition satisfied");

    assert!(eval.condition_met);
    assert_eq!(eval.details, Some("Condition satisfied".to_string()));
}

#[test]
fn test_trigger_evaluation_with_data() {
    let eval =
        TriggerEvaluation::met("test_trigger").with_data(serde_json::json!({"key": "value"}));

    assert!(eval.condition_met);
    assert!(eval.data.is_some());
}

#[test]
fn test_wakeup_method_composite() {
    use clawlegion_sentinel::WakeupAction;

    let method = WakeupMethod::Composite {
        actions: vec![
            WakeupAction::SendMessage {
                content: "Wake up!".to_string(),
            },
            WakeupAction::Heartbeat,
        ],
    };

    assert!(matches!(method, WakeupMethod::Composite { actions } if actions.len() == 2));
}
