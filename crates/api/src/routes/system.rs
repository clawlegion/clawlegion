//! System routes

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::state::ApiState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealthSummary {
    pub healthy: usize,
    pub degraded: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    pub status: String,
    pub uptime_secs: u64,
    pub version: String,
    pub agents_total: usize,
    pub agents_active: usize,
    pub plugins_loaded: usize,
    pub plugins_active: usize,
    pub memory_usage_mb: u64,
    pub plugin_health: PluginHealthSummary,
    pub message_conversations: usize,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub checks: HealthChecks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthChecks {
    pub database: String,
    pub llm_provider: String,
    pub plugin_system: String,
    pub message_service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginListResponse {
    pub plugins: Vec<clawlegion_core::PluginInfo>,
    pub capability_index: HashMap<String, Vec<String>>,
    pub bridge_index: HashMap<String, Vec<String>>,
    pub sentinel_triggers: Vec<clawlegion_plugin::PluginTriggerStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMutationResponse {
    pub ok: bool,
    pub plugin: Option<clawlegion_core::PluginInfo>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallPluginRequest {
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPluginRequest {
    pub alias: String,
    pub public_key_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignPluginRequest {
    pub private_key_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginLogsResponse {
    pub plugin_id: String,
    pub logs: Vec<clawlegion_plugin::PluginRuntimeLog>,
    pub health: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDoctorResponse {
    pub reports: Vec<serde_json::Value>,
}

pub async fn get_status(State(state): State<ApiState>) -> Json<SystemStatusResponse> {
    let agents = state.agent_registry.list_agents();
    let active_agents = agents
        .iter()
        .filter(|a| matches!(a.status, clawlegion_core::AgentStatus::Running))
        .count();

    let plugins = {
        let manager = state.plugin_manager.read();
        manager.list_plugins()
    };
    let message_stats = state.message_service.stats().await;
    let plugin_health = PluginHealthSummary {
        healthy: plugins
            .iter()
            .filter(|plugin| plugin.health.as_deref() == Some("healthy"))
            .count(),
        degraded: plugins
            .iter()
            .filter(|plugin| plugin.state == clawlegion_core::PluginState::Degraded)
            .count(),
        failed: plugins
            .iter()
            .filter(|plugin| plugin.state == clawlegion_core::PluginState::Failed)
            .count(),
    };

    Json(SystemStatusResponse {
        status: "running".to_string(),
        uptime_secs: state.uptime_secs(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        agents_total: agents.len(),
        agents_active: active_agents,
        plugins_loaded: plugins.len(),
        plugins_active: plugins
            .iter()
            .filter(|plugin| plugin.state == clawlegion_core::PluginState::Active)
            .count(),
        memory_usage_mb: get_memory_usage_mb(),
        plugin_health,
        message_conversations: message_stats.conversations,
        message_count: message_stats.messages,
    })
}

pub async fn get_health(State(state): State<ApiState>) -> Json<HealthResponse> {
    let has_failed = {
        let manager = state.plugin_manager.read();
        manager
            .list_plugins()
            .iter()
            .any(|plugin| plugin.state == clawlegion_core::PluginState::Failed)
    };
    let message_stats = state.message_service.stats().await;

    Json(HealthResponse {
        status: if has_failed {
            "degraded".to_string()
        } else {
            "healthy".to_string()
        },
        checks: HealthChecks {
            database: "ok".to_string(),
            llm_provider: "ok".to_string(),
            plugin_system: if has_failed {
                "degraded".to_string()
            } else {
                "ok".to_string()
            },
            message_service: if message_stats.messages <= message_stats.max_messages {
                "ok".to_string()
            } else {
                "degraded".to_string()
            },
        },
    })
}

pub async fn list_plugins(State(state): State<ApiState>) -> Json<PluginListResponse> {
    let manager = state.plugin_manager.read();
    Json(PluginListResponse {
        plugins: manager.list_plugins(),
        capability_index: manager.capability_snapshot(),
        bridge_index: manager.bridge_snapshot(),
        sentinel_triggers: manager.sentinel_trigger_snapshot(),
    })
}

pub async fn get_plugin(
    State(state): State<ApiState>,
    Path(plugin_id): Path<String>,
) -> Result<Json<clawlegion_core::PluginInfo>, StatusCode> {
    let manager = state.plugin_manager.read();
    manager
        .inspect(&plugin_id)
        .map(Json)
        .map_err(|_| StatusCode::NOT_FOUND)
}

pub async fn enable_plugin(
    State(state): State<ApiState>,
    Path(plugin_id): Path<String>,
) -> Result<Json<PluginMutationResponse>, StatusCode> {
    let manager = std::sync::Arc::clone(&state.plugin_manager);
    let plugin = tokio::task::spawn_blocking(move || -> Result<_, StatusCode> {
        let mut manager = manager.write();
        futures_executor::block_on(manager.enable(&plugin_id))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        manager
            .inspect(&plugin_id)
            .map_err(|_| StatusCode::NOT_FOUND)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok(Json(PluginMutationResponse {
        ok: true,
        plugin: Some(plugin),
        detail: "plugin enabled".to_string(),
    }))
}

pub async fn disable_plugin(
    State(state): State<ApiState>,
    Path(plugin_id): Path<String>,
) -> Result<Json<PluginMutationResponse>, StatusCode> {
    let manager = std::sync::Arc::clone(&state.plugin_manager);
    let plugin = tokio::task::spawn_blocking(move || -> Result<_, StatusCode> {
        let mut manager = manager.write();
        futures_executor::block_on(manager.disable(&plugin_id))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        manager
            .inspect(&plugin_id)
            .map_err(|_| StatusCode::NOT_FOUND)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok(Json(PluginMutationResponse {
        ok: true,
        plugin: Some(plugin),
        detail: "plugin disabled".to_string(),
    }))
}

pub async fn reload_plugin(
    State(state): State<ApiState>,
    Path(plugin_id): Path<String>,
) -> Result<Json<PluginMutationResponse>, StatusCode> {
    let manager = std::sync::Arc::clone(&state.plugin_manager);
    let plugin = tokio::task::spawn_blocking(move || -> Result<_, StatusCode> {
        let mut manager = manager.write();
        futures_executor::block_on(manager.reload_config(&plugin_id))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        manager
            .inspect(&plugin_id)
            .map_err(|_| StatusCode::NOT_FOUND)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok(Json(PluginMutationResponse {
        ok: true,
        plugin: Some(plugin),
        detail: "plugin reloaded".to_string(),
    }))
}

pub async fn install_plugin(
    State(state): State<ApiState>,
    Json(payload): Json<InstallPluginRequest>,
) -> Result<Json<PluginMutationResponse>, StatusCode> {
    let manager = std::sync::Arc::clone(&state.plugin_manager);
    let plugin = tokio::task::spawn_blocking(move || -> Result<_, StatusCode> {
        let mut manager = manager.write();
        manager
            .install(std::path::Path::new(&payload.source_path))
            .map_err(|_| StatusCode::BAD_REQUEST)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok(Json(PluginMutationResponse {
        ok: true,
        plugin: Some(plugin),
        detail: "plugin installed".to_string(),
    }))
}

pub async fn uninstall_plugin(
    State(state): State<ApiState>,
    Path(plugin_id): Path<String>,
) -> Result<Json<PluginMutationResponse>, StatusCode> {
    let manager = std::sync::Arc::clone(&state.plugin_manager);
    tokio::task::spawn_blocking(move || -> Result<(), StatusCode> {
        let mut manager = manager.write();
        futures_executor::block_on(manager.uninstall(&plugin_id))
            .map_err(|_| StatusCode::BAD_REQUEST)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok(Json(PluginMutationResponse {
        ok: true,
        plugin: None,
        detail: "plugin uninstalled".to_string(),
    }))
}

pub async fn trust_plugin_key(
    State(state): State<ApiState>,
    Json(payload): Json<TrustPluginRequest>,
) -> Result<Json<PluginMutationResponse>, StatusCode> {
    let manager = state.plugin_manager.read();
    manager
        .trust_key(
            &payload.alias,
            std::path::Path::new(&payload.public_key_path),
        )
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(PluginMutationResponse {
        ok: true,
        plugin: None,
        detail: "trust key stored".to_string(),
    }))
}

pub async fn sign_plugin(
    State(state): State<ApiState>,
    Path(plugin_id): Path<String>,
    Json(payload): Json<SignPluginRequest>,
) -> Result<Json<PluginMutationResponse>, StatusCode> {
    let manager = state.plugin_manager.read();
    manager
        .sign_plugin(&plugin_id, std::path::Path::new(&payload.private_key_path))
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let plugin = manager
        .inspect(&plugin_id)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(PluginMutationResponse {
        ok: true,
        plugin: Some(plugin),
        detail: "plugin signed".to_string(),
    }))
}

pub async fn plugin_logs(
    State(state): State<ApiState>,
    Path(plugin_id): Path<String>,
) -> Result<Json<PluginLogsResponse>, StatusCode> {
    let manager = state.plugin_manager.read();
    let plugin = manager
        .inspect(&plugin_id)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let last_error = plugin.errors.last().cloned();
    let logs = manager.plugin_logs(&plugin_id);
    Ok(Json(PluginLogsResponse {
        plugin_id: plugin.id,
        logs,
        health: plugin.health,
        last_error,
    }))
}

pub async fn plugin_doctor(State(state): State<ApiState>) -> Json<PluginDoctorResponse> {
    let manager = state.plugin_manager.read();
    Json(PluginDoctorResponse {
        reports: manager.doctor_report(),
    })
}

fn get_memory_usage_mb() -> u64 {
    0
}
