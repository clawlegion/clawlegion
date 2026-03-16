//! 消息系统测试

mod common;

use chrono::Duration;
use clawlegion_core::{
    ChatMessage, ChatType, MessageContent, MessageMetadata, NotificationLevel, Uuid,
};

#[test]
fn test_chat_message_private_creation() {
    let company_id = common::test_company_id();
    let sender_id = common::test_agent_id();
    let recipient_id = common::test_agent_id();

    let message = ChatMessage::private(
        company_id,
        sender_id,
        recipient_id,
        MessageContent::Text("Hello".to_string()),
    );

    assert_eq!(message.company_id, company_id);
    assert_eq!(message.sender, sender_id);
    assert!(matches!(
        message.chat_type,
        ChatType::Private { participant } if participant == recipient_id
    ));
    assert!(matches!(message.content, MessageContent::Text(ref text) if text == "Hello"));
}

#[test]
fn test_chat_message_reply() {
    let company_id = common::test_company_id();
    let chat_type = ChatType::Private {
        participant: common::test_agent_id(),
    };
    let sender_id = common::test_agent_id();
    let in_reply_to = common::test_message_id();

    let message = ChatMessage::reply(
        company_id,
        chat_type,
        sender_id,
        "Reply content".to_string(),
        in_reply_to,
    );

    assert_eq!(message.company_id, company_id);
    assert_eq!(message.sender, sender_id);
    assert!(matches!(
        message.content,
        MessageContent::Reply { text, in_reply_to: reply_id }
        if text == "Reply content" && reply_id == in_reply_to
    ));
}

#[test]
fn test_message_content_types() {
    // Text content
    let text_content = MessageContent::Text("Test".to_string());
    assert!(matches!(text_content, MessageContent::Text(_)));

    // Command content
    let cmd_content = MessageContent::Command {
        command: "run".to_string(),
        args: vec!["--test".to_string()],
    };
    assert!(matches!(cmd_content, MessageContent::Command { .. }));

    // Notification content
    let notif_content = MessageContent::Notification {
        title: "Alert".to_string(),
        body: "Test notification".to_string(),
        level: NotificationLevel::Warning,
    };
    assert!(matches!(
        notif_content,
        MessageContent::Notification {
            level: NotificationLevel::Warning,
            ..
        }
    ));
}

#[test]
fn test_message_metadata_default() {
    let metadata = MessageMetadata::default();
    assert!(!metadata.important);
    assert!(metadata.expires_at.is_none());
    assert!(metadata.tags.is_empty());
    assert!(!metadata.is_expired());
}

#[test]
fn test_message_metadata_new() {
    let metadata = MessageMetadata::new();
    assert!(!metadata.important);
    assert!(metadata.expires_at.is_none());
    assert!(!metadata.is_expired());
}

#[test]
fn test_message_metadata_expiring_in() {
    let metadata = MessageMetadata::expiring_in(Duration::hours(1));
    assert!(metadata.expires_at.is_some());
    assert!(!metadata.is_expired());
}

#[test]
fn test_message_metadata_expiring_in_past() {
    let metadata = MessageMetadata::expiring_in(Duration::hours(-1));
    assert!(metadata.expires_at.is_some());
    assert!(metadata.is_expired());
}

#[test]
fn test_message_metadata_with_expiry() {
    let future_time = common::future_datetime(24);
    let metadata = MessageMetadata::with_expiry(future_time);
    assert_eq!(metadata.expires_at, Some(future_time));
    assert!(!metadata.is_expired());

    let past_time = common::past_datetime(1);
    let metadata_past = MessageMetadata::with_expiry(past_time);
    assert!(metadata_past.is_expired());
}

#[test]
fn test_message_metadata_tags() {
    let mut metadata = MessageMetadata::new();
    metadata.tags.push("important".to_string());
    metadata.tags.push("urgent".to_string());

    assert_eq!(metadata.tags.len(), 2);
    assert!(metadata.tags.contains(&"important".to_string()));
}

#[test]
fn test_message_metadata_important() {
    let mut metadata = MessageMetadata::new();
    metadata.important = true;

    assert!(metadata.important);
}

#[test]
fn test_message_metadata_custom_fields() {
    let mut metadata = MessageMetadata::new();
    metadata
        .custom
        .insert("key".to_string(), serde_json::json!("value"));
    metadata.related_issue_id = Some(Uuid::new_v4());

    assert!(metadata.custom.contains_key("key"));
    assert!(metadata.related_issue_id.is_some());
}
