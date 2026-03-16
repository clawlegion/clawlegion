//! Agent routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use clawlegion_core::AgentId;

use crate::{
    dto::{
        AgentDetailResponse, AgentResponse, AgentSkillResponse, AgentStatusResponse,
        ListAgentSkillsResponse, ListAgentsResponse,
    },
    state::ApiState,
};

pub async fn list_agents(State(state): State<ApiState>) -> Json<ListAgentsResponse> {
    let agents = state.agent_registry.list_agents();
    let agent_responses: Vec<AgentResponse> =
        agents.iter().map(AgentResponse::from_agent_info).collect();

    Json(ListAgentsResponse {
        agents: agent_responses,
    })
}

pub async fn get_agent(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<AgentDetailResponse>, StatusCode> {
    let agent_id = id.parse::<AgentId>().map_err(|_| StatusCode::BAD_REQUEST)?;
    let info = state
        .agent_registry
        .get_info(agent_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(AgentDetailResponse::from_agent_info(&info)))
}

pub async fn get_agent_status(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<AgentStatusResponse>, StatusCode> {
    let agent_id = id.parse::<AgentId>().map_err(|_| StatusCode::BAD_REQUEST)?;
    let info = state
        .agent_registry
        .get_info(agent_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(AgentStatusResponse::from_agent_info(&info, None)))
}

pub async fn get_agent_skills(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ListAgentSkillsResponse>, StatusCode> {
    let agent_id = id.parse::<AgentId>().map_err(|_| StatusCode::BAD_REQUEST)?;
    let info = state
        .agent_registry
        .get_info(agent_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let skills = info
        .config
        .skills
        .iter()
        .map(|skill| AgentSkillResponse {
            name: skill.clone(),
            version: "unknown".to_string(),
            description: "skill metadata unavailable".to_string(),
            execution_count: None,
        })
        .collect();

    Ok(Json(ListAgentSkillsResponse {
        agent_id: info.config.id.to_string(),
        skills,
    }))
}
