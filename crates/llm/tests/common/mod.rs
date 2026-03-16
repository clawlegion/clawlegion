//! LLM 测试通用工具

use uuid::Uuid;

/// 创建测试用的 MessageId
#[allow(dead_code)]
pub fn test_message_id() -> Uuid {
    Uuid::new_v4()
}
