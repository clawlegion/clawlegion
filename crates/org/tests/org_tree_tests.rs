//! 组织树测试

mod common;

use clawlegion_org::{OrgAgent, OrgTree};
use uuid::Uuid;

#[test]
fn test_org_tree_creation() {
    let company_id = common::test_company_id();
    let tree = OrgTree::new(company_id);

    assert_eq!(tree.company_id(), company_id);
    assert_eq!(tree.agent_count(), 0);
}

#[test]
fn test_org_tree_add_ceo() {
    let company_id = common::test_company_id();

    let tree = OrgTree::new(company_id);
    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = common::test_agent_id();

    let result = tree.add_agent(ceo);
    assert!(result.is_ok());

    // 验证 CEO 被设置
    let ceo_agent = tree.get_ceo();
    assert!(ceo_agent.is_some());

    assert_eq!(tree.agent_count(), 1);
}

#[test]
fn test_org_tree_add_subordinate() {
    let company_id = common::test_company_id();
    let ceo_id = common::test_agent_id();
    let manager_id = common::test_message_id();

    let tree = OrgTree::new(company_id);

    // 添加 CEO
    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = ceo_id;
    assert!(tree.add_agent(ceo).is_ok());

    // 添加经理（汇报给 CEO）
    let mut manager = OrgAgent::new(
        company_id,
        "Manager".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager.id = manager_id;
    manager.set_reports_to(Some(ceo_id));
    assert!(tree.add_agent(manager).is_ok());

    // 验证层级关系
    assert_eq!(tree.agent_count(), 2);

    let direct_reports = tree.get_direct_reports(ceo_id);
    assert_eq!(direct_reports.len(), 1);
    assert_eq!(direct_reports[0].read().id, manager_id);
}

#[test]
fn test_org_tree_get_manager() {
    let company_id = common::test_company_id();
    let ceo_id = common::test_agent_id();
    let employee_id = common::test_message_id();

    let tree = OrgTree::new(company_id);

    // 添加 CEO
    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = ceo_id;
    tree.add_agent(ceo).unwrap();

    // 添加员工（汇报给 CEO）
    let mut employee = OrgAgent::new(
        company_id,
        "Employee".to_string(),
        "engineer".to_string(),
        "工程师".to_string(),
    );
    employee.id = employee_id;
    employee.set_reports_to(Some(ceo_id));
    tree.add_agent(employee).unwrap();

    // 验证获取管理者
    let manager = tree.get_manager(employee_id);
    assert!(manager.is_some());
    assert_eq!(manager.unwrap().read().id, ceo_id);
}

#[test]
fn test_org_tree_chain_of_command() {
    let company_id = common::test_company_id();
    let ceo_id = common::test_agent_id();
    let manager_id = common::test_message_id();
    let employee_id = common::test_company_id();

    let tree = OrgTree::new(company_id);

    // CEO
    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = ceo_id;
    tree.add_agent(ceo).unwrap();

    // Manager (reports to CEO)
    let mut manager = OrgAgent::new(
        company_id,
        "Manager".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager.id = manager_id;
    manager.set_reports_to(Some(ceo_id));
    tree.add_agent(manager).unwrap();

    // Employee (reports to Manager)
    let mut employee = OrgAgent::new(
        company_id,
        "Employee".to_string(),
        "engineer".to_string(),
        "工程师".to_string(),
    );
    employee.id = employee_id;
    employee.set_reports_to(Some(manager_id));
    tree.add_agent(employee).unwrap();

    // 验证指挥链
    let chain = tree.get_chain_of_command(employee_id);
    assert_eq!(chain.len(), 3); // Employee -> Manager -> CEO

    // 验证顺序（从员工到 CEO）
    assert_eq!(chain[0].read().id, employee_id);
    assert_eq!(chain[1].read().id, manager_id);
    assert_eq!(chain[2].read().id, ceo_id);
}

#[test]
fn test_org_tree_is_in_chain() {
    let company_id = common::test_company_id();
    let ceo_id = common::test_agent_id();
    let manager_id = common::test_message_id();
    let employee_id = common::test_company_id();

    let tree = OrgTree::new(company_id);

    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = ceo_id;
    tree.add_agent(ceo).unwrap();

    let mut manager = OrgAgent::new(
        company_id,
        "Manager".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager.id = manager_id;
    manager.set_reports_to(Some(ceo_id));
    tree.add_agent(manager).unwrap();

    let mut employee = OrgAgent::new(
        company_id,
        "Employee".to_string(),
        "engineer".to_string(),
        "工程师".to_string(),
    );
    employee.id = employee_id;
    employee.set_reports_to(Some(manager_id));
    tree.add_agent(employee).unwrap();

    // CEO 在员工的指挥链中
    assert!(tree.is_in_chain(ceo_id, employee_id));

    // Manager 在员工的指挥链中
    assert!(tree.is_in_chain(manager_id, employee_id));

    // 员工不在 CEO 的指挥链中
    assert!(!tree.is_in_chain(employee_id, ceo_id));
}

#[test]
fn test_org_tree_get_depth() {
    let company_id = common::test_company_id();
    let ceo_id = common::test_agent_id();
    let manager_id = common::test_message_id();
    let employee_id = common::test_company_id();

    let tree = OrgTree::new(company_id);

    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = ceo_id;
    tree.add_agent(ceo).unwrap();

    let mut manager = OrgAgent::new(
        company_id,
        "Manager".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager.id = manager_id;
    manager.set_reports_to(Some(ceo_id));
    tree.add_agent(manager).unwrap();

    let mut employee = OrgAgent::new(
        company_id,
        "Employee".to_string(),
        "engineer".to_string(),
        "工程师".to_string(),
    );
    employee.id = employee_id;
    employee.set_reports_to(Some(manager_id));
    tree.add_agent(employee).unwrap();

    // 验证深度
    assert_eq!(tree.get_depth(ceo_id), Some(0)); // CEO 深度为 0
    assert_eq!(tree.get_depth(manager_id), Some(1)); // Manager 深度为 1
    assert_eq!(tree.get_depth(employee_id), Some(2)); // Employee 深度为 2
}

#[test]
fn test_org_tree_multiple_ceos_fails() {
    let company_id = common::test_company_id();
    let ceo_id1 = common::test_agent_id();
    let ceo_id2 = common::test_message_id();

    let tree = OrgTree::new(company_id);

    // 添加第一个 CEO
    let mut ceo1 = OrgAgent::new(
        company_id,
        "CEO1".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo1.id = ceo_id1;
    assert!(tree.add_agent(ceo1).is_ok());

    // 尝试添加第二个 CEO（应该失败）
    let mut ceo2 = OrgAgent::new(
        company_id,
        "CEO2".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo2.id = ceo_id2;
    let result = tree.add_agent(ceo2);
    assert!(result.is_err());
}

#[test]
fn test_org_tree_remove_agent_with_reports() {
    let company_id = common::test_company_id();
    let ceo_id = common::test_agent_id();
    let manager_id = common::test_message_id();

    let tree = OrgTree::new(company_id);

    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = ceo_id;
    tree.add_agent(ceo).unwrap();

    let mut manager = OrgAgent::new(
        company_id,
        "Manager".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager.id = manager_id;
    manager.set_reports_to(Some(ceo_id));
    tree.add_agent(manager).unwrap();

    // 删除有直接汇报的 CEO 应该失败
    let result = tree.remove_agent(ceo_id);
    assert!(result.is_err());

    // 先删除 manager
    assert!(tree.remove_agent(manager_id).is_ok());

    // 现在可以删除 CEO
    assert!(tree.remove_agent(ceo_id).is_ok());
    assert_eq!(tree.agent_count(), 0);
}

#[test]
fn test_org_tree_get_all_reports() {
    let company_id = common::test_company_id();
    let ceo_id = common::test_agent_id();
    let manager1_id = common::test_message_id();
    // 使用 Uuid::new_v4() 避免 ID 冲突
    let manager2_id = Uuid::new_v4();
    let employee_id = Uuid::new_v4();

    let tree = OrgTree::new(company_id);

    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = ceo_id;
    tree.add_agent(ceo).unwrap();

    let mut manager1 = OrgAgent::new(
        company_id,
        "Manager1".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager1.id = manager1_id;
    manager1.set_reports_to(Some(ceo_id));
    tree.add_agent(manager1).unwrap();

    let mut manager2 = OrgAgent::new(
        company_id,
        "Manager2".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager2.id = manager2_id;
    manager2.set_reports_to(Some(ceo_id));
    tree.add_agent(manager2).unwrap();

    let mut employee = OrgAgent::new(
        company_id,
        "Employee".to_string(),
        "engineer".to_string(),
        "工程师".to_string(),
    );
    employee.id = employee_id;
    employee.set_reports_to(Some(manager1_id));
    tree.add_agent(employee).unwrap();

    // 获取 CEO 的所有下属
    let all_reports = tree.get_all_reports(ceo_id);
    assert_eq!(all_reports.len(), 3); // manager1, manager2, employee
}

#[test]
fn test_org_tree_org_chart() {
    let company_id = common::test_company_id();
    let ceo_id = common::test_agent_id();
    let manager_id = common::test_message_id();

    let tree = OrgTree::new(company_id);

    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = ceo_id;
    tree.add_agent(ceo).unwrap();

    let mut manager = OrgAgent::new(
        company_id,
        "Manager".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager.id = manager_id;
    manager.set_reports_to(Some(ceo_id));
    tree.add_agent(manager).unwrap();

    // 获取组织图表
    let chart = tree.get_org_chart();
    assert!(chart.is_some());

    let root = chart.unwrap();
    assert_eq!(root.id, ceo_id);
    assert_eq!(root.name, "CEO");
    assert_eq!(root.reports.len(), 1);
    assert_eq!(root.reports[0].name, "Manager");
}

#[test]
fn test_org_agent_is_ceo() {
    let company_id = common::test_company_id();

    // CEO (no manager)
    let mut ceo = OrgAgent::new(
        company_id,
        "CEO".to_string(),
        "ceo".to_string(),
        "首席执行官".to_string(),
    );
    ceo.id = common::test_agent_id();
    assert!(ceo.is_ceo());

    // Manager (has manager)
    let mut manager = OrgAgent::new(
        company_id,
        "Manager".to_string(),
        "manager".to_string(),
        "经理".to_string(),
    );
    manager.id = common::test_message_id();
    manager.set_reports_to(Some(common::test_agent_id()));
    assert!(!manager.is_ceo());
}

#[test]
fn test_org_agent_permissions() {
    use clawlegion_org::AgentPermissions;

    let ceo_perms = AgentPermissions::ceo();
    assert!(ceo_perms.can_hire);
    assert!(ceo_perms.can_fire);
    assert!(ceo_perms.can_manage_budget);

    let manager_perms = AgentPermissions::manager();
    assert!(manager_perms.can_hire);
    assert!(!manager_perms.can_fire);
    assert!(manager_perms.can_manage_budget);

    let contributor_perms = AgentPermissions::contributor();
    assert!(!contributor_perms.can_hire);
    assert!(!contributor_perms.can_fire);
    assert!(!contributor_perms.can_manage_budget);
}
