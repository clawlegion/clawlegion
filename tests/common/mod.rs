//! 集成测试通用工具

use uuid::Uuid;

/// 创建测试用的 CompanyId
pub fn test_company_id() -> Uuid {
    Uuid::new_v4()
}

/// 创建测试用的 AgentId
pub fn test_agent_id() -> Uuid {
    Uuid::new_v4()
}

/// 创建测试用的 MessageId
pub fn test_message_id() -> Uuid {
    Uuid::new_v4()
}

/// 测试 API 配置
#[allow(dead_code)]
pub const TEST_API_KEY: &str = "sk-SFng2PCyEKcQw82fA-cl-test";

#[allow(dead_code)]
pub const TEST_API_BASE: &str = "https://103.237.28.67:8317/v1";

#[allow(dead_code)]
pub const TEST_MODEL: &str = "qwen3-coder-plus";
