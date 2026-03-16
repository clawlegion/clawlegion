//! 测试通用工具

use uuid::Uuid;

/// 创建测试用的 CompanyId
#[allow(dead_code)]
pub fn test_company_id() -> Uuid {
    Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap()
}

/// 创建测试用的 AgentId
pub fn test_agent_id() -> Uuid {
    Uuid::parse_str("66666666-6666-6666-6666-666666666666").unwrap()
}

/// 创建测试用的 MessageId
pub fn test_message_id() -> Uuid {
    Uuid::parse_str("77777777-7777-7777-7777-777777777777").unwrap()
}
