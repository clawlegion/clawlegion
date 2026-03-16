//! 测试通用工具和 fixtures

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

/// 创建测试用的 CompanyId
pub fn test_company_id() -> Uuid {
    Uuid::parse_str("12345678-1234-1234-1234-123456789012").unwrap()
}

/// 创建测试用的 AgentId
pub fn test_agent_id() -> Uuid {
    Uuid::parse_str("87654321-4321-4321-4321-210987654321").unwrap()
}

/// 创建测试用的 MessageId
pub fn test_message_id() -> Uuid {
    Uuid::new_v4()
}

/// 创建过去的 DateTime（用于测试过期逻辑）
#[allow(dead_code)]
pub fn past_datetime(hours: i64) -> DateTime<Utc> {
    Utc::now() - Duration::hours(hours)
}

/// 创建未来的 DateTime（用于测试过期逻辑）
#[allow(dead_code)]
pub fn future_datetime(hours: i64) -> DateTime<Utc> {
    Utc::now() + Duration::hours(hours)
}
