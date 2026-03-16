use chrono::{Duration, Utc};

use clawlegion_api::services::message_service::{
    ConversationKind, MessageParticipant, MessageRecord, MessageService,
};

#[tokio::test]
async fn creates_and_reuses_same_conversation_for_same_participants() {
    let service = MessageService::default();
    let participants = vec![
        MessageParticipant {
            id: "agent-alpha".to_string(),
            name: "Alpha".to_string(),
        },
        MessageParticipant {
            id: "agent-bravo".to_string(),
            name: "Bravo".to_string(),
        },
    ];

    let first = service
        .get_or_create_conversation(ConversationKind::AgentAgent, participants.clone())
        .await
        .expect("first conversation");
    let second = service
        .get_or_create_conversation(ConversationKind::AgentAgent, participants)
        .await
        .expect("second conversation");

    assert_eq!(first.conversation_id, second.conversation_id);
}

#[tokio::test]
async fn poll_updates_returns_messages_after_timestamp() {
    let service = MessageService::default();
    let conversation = service
        .get_or_create_conversation(
            ConversationKind::UserAgent,
            vec![
                MessageParticipant {
                    id: "user-console".to_string(),
                    name: "Console User".to_string(),
                },
                MessageParticipant {
                    id: "agent-charlie".to_string(),
                    name: "Charlie".to_string(),
                },
            ],
        )
        .await
        .expect("conversation");

    let since = Utc::now() - Duration::seconds(1);

    service
        .send_message(MessageRecord {
            message_id: "msg-1".to_string(),
            conversation_id: conversation.conversation_id.clone(),
            sender_id: "user-console".to_string(),
            sender_name: "Console User".to_string(),
            recipient_id: "agent-charlie".to_string(),
            recipient_name: "Charlie".to_string(),
            content: "ping".to_string(),
            timestamp: Utc::now(),
            message_type: "text".to_string(),
            reply_to_id: None,
            read: false,
        })
        .await
        .expect("message");

    let poll = service.poll_updates(since, None, None).await;
    assert_eq!(poll.messages.len(), 1);
    assert_eq!(poll.conversations.len(), 1);
    assert_eq!(poll.messages[0].content, "ping");
}
