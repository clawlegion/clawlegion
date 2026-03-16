//! Agent 测试通用工具

use uuid::Uuid;

/// 创建测试用的 CompanyId
#[allow(dead_code)]
pub fn test_company_id() -> Uuid {
    Uuid::parse_str("88888888-8888-8888-8888-888888888888").unwrap()
}

/// 创建测试用的 AgentId
#[allow(dead_code)]
pub fn test_agent_id() -> Uuid {
    Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap()
}

/// 创建测试用的 MessageId
#[allow(dead_code)]
pub fn test_message_id() -> Uuid {
    Uuid::new_v4()
}
