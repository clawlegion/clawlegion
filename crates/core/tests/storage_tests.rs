//! 存储系统测试

mod common;

use clawlegion_core::{MemoryCategory, MemoryEntry, MemorySearchQuery};

#[test]
fn test_memory_entry_creation() {
    let company_id = common::test_company_id();
    let agent_id = Some(common::test_agent_id());

    let memory = MemoryEntry {
        id: common::test_message_id(),
        company_id,
        agent_id,
        content: "Test memory content".to_string(),
        category: MemoryCategory::ShortTerm,
        importance: 0.8,
        access_count: 0,
        last_accessed_at: chrono::Utc::now(),
        created_at: chrono::Utc::now(),
        tags: vec!["test".to_string()],
        related_messages: vec![],
        expires_at: None,
    };

    assert_eq!(memory.company_id, company_id);
    assert_eq!(memory.agent_id, agent_id);
    assert_eq!(memory.content, "Test memory content");
    assert_eq!(memory.category, MemoryCategory::ShortTerm);
    assert_eq!(memory.importance, 0.8);
}

#[test]
fn test_memory_category_str() {
    assert_eq!(MemoryCategory::ShortTerm.as_str(), "short_term");
    assert_eq!(MemoryCategory::LongTerm.as_str(), "long_term");
    assert_eq!(MemoryCategory::Procedural.as_str(), "procedural");
    assert_eq!(MemoryCategory::Semantic.as_str(), "semantic");
    assert_eq!(MemoryCategory::Episodic.as_str(), "episodic");
}

#[test]
fn test_memory_retention_score() {
    // 高重要性记忆应该有更高的保留分数
    let high_importance_memory = MemoryEntry {
        id: common::test_message_id(),
        company_id: common::test_company_id(),
        agent_id: None,
        content: "Important".to_string(),
        category: MemoryCategory::LongTerm,
        importance: 1.0,
        access_count: 10,
        last_accessed_at: chrono::Utc::now(),
        created_at: chrono::Utc::now(),
        tags: vec![],
        related_messages: vec![],
        expires_at: None,
    };

    let low_importance_memory = MemoryEntry {
        id: common::test_message_id(),
        company_id: common::test_company_id(),
        agent_id: None,
        content: "Unimportant".to_string(),
        category: MemoryCategory::ShortTerm,
        importance: 0.1,
        access_count: 0,
        last_accessed_at: common::past_datetime(24),
        created_at: common::past_datetime(48),
        tags: vec![],
        related_messages: vec![],
        expires_at: None,
    };

    let high_score = high_importance_memory.retention_score();
    let low_score = low_importance_memory.retention_score();

    // 调试输出
    eprintln!("High importance score: {}", high_score);
    eprintln!("Low importance score: {}", low_score);

    // 高重要性记忆（刚访问，高重要性，高访问次数）应该有更高的保留分数
    // 低重要性记忆（24 小时前访问，低重要性，无访问）应该有较低的保留分数
    assert!(
        high_score > low_score,
        "Expected high_score ({}) > low_score ({})",
        high_score,
        low_score
    );
}

#[test]
fn test_memory_should_compress() {
    // 短期记忆且保留分数低应该压缩
    let compressible_memory = MemoryEntry {
        id: common::test_message_id(),
        company_id: common::test_company_id(),
        agent_id: None,
        content: "Old short-term".to_string(),
        category: MemoryCategory::ShortTerm,
        importance: 0.1,
        access_count: 0,
        last_accessed_at: common::past_datetime(168), // 1 周前
        created_at: common::past_datetime(168),
        tags: vec![],
        related_messages: vec![],
        expires_at: None,
    };

    // 长期记忆不应该压缩
    let long_term_memory = MemoryEntry {
        id: common::test_message_id(),
        company_id: common::test_company_id(),
        agent_id: None,
        content: "Important long-term".to_string(),
        category: MemoryCategory::LongTerm,
        importance: 0.9,
        access_count: 5,
        last_accessed_at: chrono::Utc::now(),
        created_at: common::past_datetime(24),
        tags: vec![],
        related_messages: vec![],
        expires_at: None,
    };

    assert!(compressible_memory.should_compress());
    assert!(!long_term_memory.should_compress());
}

#[test]
fn test_memory_should_forget() {
    // 保留分数极低且无保护的应该被遗忘
    let forgettable_memory = MemoryEntry {
        id: common::test_message_id(),
        company_id: common::test_company_id(),
        agent_id: None,
        content: "Forgettable".to_string(),
        category: MemoryCategory::ShortTerm,
        importance: 0.01,
        access_count: 0,
        last_accessed_at: common::past_datetime(720), // 30 天前
        created_at: common::past_datetime(720),
        tags: vec![],
        related_messages: vec![],
        expires_at: None,
    };

    // 受保护的标签不应该被遗忘
    let protected_memory = MemoryEntry {
        id: common::test_message_id(),
        company_id: common::test_company_id(),
        agent_id: None,
        content: "Protected".to_string(),
        category: MemoryCategory::ShortTerm,
        importance: 0.01,
        access_count: 0,
        last_accessed_at: common::past_datetime(720),
        created_at: common::past_datetime(720),
        tags: vec!["protected".to_string()],
        related_messages: vec![],
        expires_at: None,
    };

    assert!(forgettable_memory.should_forget());
    assert!(!protected_memory.should_forget());
}

#[test]
fn test_memory_is_expired() {
    // 无过期时间的永不过期
    let no_expiry_memory = MemoryEntry {
        id: common::test_message_id(),
        company_id: common::test_company_id(),
        agent_id: None,
        content: "No expiry".to_string(),
        category: MemoryCategory::LongTerm,
        importance: 0.5,
        access_count: 0,
        last_accessed_at: chrono::Utc::now(),
        created_at: chrono::Utc::now(),
        tags: vec![],
        related_messages: vec![],
        expires_at: None,
    };
    assert!(!no_expiry_memory.is_expired());

    // 未来过期时间的未过期
    let future_expiry_memory = MemoryEntry {
        id: common::test_message_id(),
        company_id: common::test_company_id(),
        agent_id: None,
        content: "Future expiry".to_string(),
        category: MemoryCategory::LongTerm,
        importance: 0.5,
        access_count: 0,
        last_accessed_at: chrono::Utc::now(),
        created_at: chrono::Utc::now(),
        tags: vec![],
        related_messages: vec![],
        expires_at: Some(common::future_datetime(24)),
    };
    assert!(!future_expiry_memory.is_expired());

    // 过去过期时间的已过期
    let past_expiry_memory = MemoryEntry {
        id: common::test_message_id(),
        company_id: common::test_company_id(),
        agent_id: None,
        content: "Past expiry".to_string(),
        category: MemoryCategory::LongTerm,
        importance: 0.5,
        access_count: 0,
        last_accessed_at: chrono::Utc::now(),
        created_at: chrono::Utc::now(),
        tags: vec![],
        related_messages: vec![],
        expires_at: Some(common::past_datetime(1)),
    };
    assert!(past_expiry_memory.is_expired());
}

#[test]
fn test_memory_search_query() {
    let query = MemorySearchQuery {
        keywords: Some(vec!["test".to_string(), "keyword".to_string()]),
        category: Some(MemoryCategory::ShortTerm),
        agent_id: Some(common::test_agent_id()),
        company_id: Some(common::test_company_id()),
        min_importance: Some(0.5),
        limit: Some(10),
        ..Default::default()
    };

    assert_eq!(query.keywords.as_ref().unwrap().len(), 2);
    assert_eq!(query.category, Some(MemoryCategory::ShortTerm));
    assert_eq!(query.min_importance, Some(0.5));
    assert_eq!(query.limit, Some(10));
}

#[test]
fn test_memory_search_query_default() {
    let query = MemorySearchQuery::default();
    assert!(query.keywords.is_none());
    assert!(query.category.is_none());
    assert!(query.agent_id.is_none());
    assert!(query.company_id.is_none());
    assert!(query.min_importance.is_none());
    assert!(query.limit.is_none());
}
