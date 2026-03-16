//! 测试通用工具

use uuid::Uuid;

/// 创建测试用的 CompanyId
pub fn test_company_id() -> Uuid {
    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
}

/// 创建测试用的 AgentId
pub fn test_agent_id() -> Uuid {
    Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()
}

/// 创建测试用的 MessageId
pub fn test_message_id() -> Uuid {
    Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap()
}
