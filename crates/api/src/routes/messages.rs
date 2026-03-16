use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    dto::message_api_dto::{
        ConversationResponse, ConversationSummaryResponse, CreateConversationRequest,
        ListConversationsResponse, ListMessagesResponse, MessageResponse, PollResponse,
        SendMessageRequest,
    },
    services::message_service::{MessageParticipant, MessageRecord, MessageServiceError},
    state::ApiState,
};

#[derive(Debug, Deserialize)]
pub struct ConversationListQuery {
    pub cursor: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct MessageListQuery {
    pub cursor: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct PollQuery {
    pub since: DateTime<Utc>,
    pub conversation_limit: Option<usize>,
    pub message_limit: Option<usize>,
}

pub async fn list_conversations(
    State(state): State<ApiState>,
    Query(query): Query<ConversationListQuery>,
) -> Result<Json<ListConversationsResponse>, StatusCode> {
    let conversations = state
        .services
        .list_conversations(query.cursor, query.limit)
        .await
        .into_iter()
        .map(ConversationSummaryResponse::from)
        .collect::<Vec<_>>();

    let next_cursor = conversations.last().and_then(|item| item.last_message_at);

    Ok(Json(ListConversationsResponse {
        conversations,
        next_cursor,
    }))
}

pub async fn get_conversation(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ConversationResponse>, StatusCode> {
    state
        .services
        .get_conversation(&id)
        .await
        .map(ConversationResponse::from)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn list_messages(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Query(query): Query<MessageListQuery>,
) -> Result<Json<ListMessagesResponse>, StatusCode> {
    if state.services.get_conversation(&id).await.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let messages = state
        .services
        .list_messages(&id, query.cursor, query.limit)
        .await
        .into_iter()
        .map(MessageResponse::from)
        .collect::<Vec<_>>();
    let next_cursor = messages.last().map(|message| message.timestamp);

    Ok(Json(ListMessagesResponse {
        messages,
        next_cursor,
    }))
}

pub async fn create_conversation(
    State(state): State<ApiState>,
    Json(payload): Json<CreateConversationRequest>,
) -> Result<Json<ConversationResponse>, StatusCode> {
    if payload.participant_ids.is_empty()
        || payload.participant_ids.len() != payload.participant_names.len()
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let participants = payload
        .participant_ids
        .into_iter()
        .zip(payload.participant_names.into_iter())
        .map(|(id, name)| MessageParticipant { id, name })
        .collect();

    let conversation = state
        .services
        .create_conversation(payload.kind, participants)
        .await
        .map_err(map_message_service_error)?;

    Ok(Json(ConversationResponse::from(conversation)))
}

pub async fn send_message(
    State(state): State<ApiState>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<MessageResponse>, StatusCode> {
    if payload.content.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let message = MessageRecord {
        message_id: Uuid::new_v4().to_string(),
        conversation_id: payload.conversation_id,
        sender_id: payload.sender_id,
        sender_name: payload.sender_name,
        recipient_id: payload.recipient_id,
        recipient_name: payload.recipient_name,
        content: payload.content,
        timestamp: Utc::now(),
        message_type: payload.message_type,
        reply_to_id: payload.reply_to_id,
        read: false,
    };

    let sent = state
        .services
        .send_message(message)
        .await
        .map_err(map_message_service_error)?;
    Ok(Json(MessageResponse::from(sent)))
}

pub async fn poll_updates(
    State(state): State<ApiState>,
    Query(query): Query<PollQuery>,
) -> Json<PollResponse> {
    Json(PollResponse::from(
        state
            .services
            .poll_updates(query.since, query.conversation_limit, query.message_limit)
            .await,
    ))
}

fn map_message_service_error(error: MessageServiceError) -> StatusCode {
    match error {
        MessageServiceError::ConversationNotFound(_) => StatusCode::NOT_FOUND,
        MessageServiceError::InvalidParticipantSet => StatusCode::BAD_REQUEST,
        MessageServiceError::PageSizeTooLarge { .. } => StatusCode::BAD_REQUEST,
        MessageServiceError::ConversationCapacityExceeded
        | MessageServiceError::MessageCapacityExceeded => StatusCode::INSUFFICIENT_STORAGE,
    }
}
