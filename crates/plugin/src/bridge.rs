use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use futures_core::Stream;
use futures_util::stream;
use parking_lot::RwLock;
use serde_json::json;
use uuid::Uuid;

use clawlegion_agent::AgentRegistry;
use clawlegion_capability::skill::SkillEvent;
use clawlegion_capability::{
    ExecutionMode, Skill, SkillContext, SkillInput, SkillMetadata, SkillOutput, SkillRegistry,
    SkillType, Tool, ToolContext, ToolMetadata, ToolRegistry, ToolResult, ToolVisibility,
    Visibility,
};
use clawlegion_core::{
    Agent, AgentConfig, AgentInfo, AgentStatus, AgentTypeDef, Error as CoreError, HeartbeatContext,
    HeartbeatResult, HeartbeatTrigger, LlmError, LlmMessage, LlmOptions, LlmProvider, LlmResponse,
    PluginCapabilityDescriptor, PluginCapabilityKind, PluginManifest, PluginType,
    Result as CoreResult, StreamChunk,
};
use clawlegion_llm::LlmRegistry;
use clawlegion_sentinel::{
    AgentWakeupHandler, SentinelManager, SentinelWatcher, TriggerCondition, WakeupMethod,
    WakeupTrigger,
};
use clawlegion_storage::{ProtocolBackedStorage, ProtocolStorageRuntime};

use crate::protocol::{
    AgentAction, AgentActionRequest, AgentActionResponse, LlmExecuteRequest, LlmExecuteResponse,
    LlmStreamChunk,
};

pub struct PluginBridgeHub {
    agent_registry: Arc<AgentRegistry>,
    llm_registry: Arc<LlmRegistry>,
    skill_registry: Arc<SkillRegistry>,
    tool_registry: Arc<ToolRegistry>,
    sentinel_manager: Arc<SentinelManager>,
    storage_backends: RwLock<HashMap<String, Arc<ProtocolBackedStorage>>>,
    trigger_cache: RwLock<HashMap<String, Vec<WakeupTrigger>>>,
    other_capabilities: RwLock<HashMap<String, Vec<PluginCapabilityDescriptor>>>,
}

impl PluginBridgeHub {
    pub fn new() -> Self {
        let watcher = Arc::new(SentinelWatcher::new());
        let sentinel_manager = Arc::new(SentinelManager::new(watcher));
        let agent_registry = Arc::new(AgentRegistry::new());
        let skill_registry = Arc::new(SkillRegistry::new());

        futures_executor::block_on(sentinel_manager.set_wakeup_handler(Box::new(
            BridgeWakeupHandler::new(Arc::clone(&agent_registry)),
        )));

        Self {
            agent_registry,
            llm_registry: Arc::new(LlmRegistry::new()),
            skill_registry,
            tool_registry: Arc::new(ToolRegistry::new()),
            sentinel_manager,
            storage_backends: RwLock::new(HashMap::new()),
            trigger_cache: RwLock::new(HashMap::new()),
            other_capabilities: RwLock::new(HashMap::new()),
        }
    }

    pub fn agent_registry(&self) -> Arc<AgentRegistry> {
        Arc::clone(&self.agent_registry)
    }

    pub fn llm_registry(&self) -> Arc<LlmRegistry> {
        Arc::clone(&self.llm_registry)
    }

    pub fn skill_registry(&self) -> Arc<SkillRegistry> {
        Arc::clone(&self.skill_registry)
    }

    pub fn tool_registry(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.tool_registry)
    }

    pub fn sentinel_manager(&self) -> Arc<SentinelManager> {
        Arc::clone(&self.sentinel_manager)
    }

    pub fn register_plugin_capabilities(
        &self,
        plugin_id: &str,
        manifest: &PluginManifest,
        capabilities: &[PluginCapabilityDescriptor],
        plugin_config: Option<&HashMap<String, serde_json::Value>>,
    ) -> CoreResult<()> {
        let mut triggers = Vec::new();
        let mut remaining = Vec::new();
        for capability in capabilities {
            match capability.kind {
                PluginCapabilityKind::LlmProvider => {
                    let provider = ProtocolBackedLlmProvider {
                        name: capability
                            .display_name
                            .clone()
                            .unwrap_or_else(|| capability.id.clone()),
                        model: capability
                            .interface
                            .clone()
                            .unwrap_or_else(|| "plugin-proxy".to_string()),
                        plugin_id: plugin_id.to_string(),
                        runtime: manifest.runtime.clone(),
                        entrypoint: manifest.entrypoint.clone(),
                    };
                    let _ = self
                        .llm_registry
                        .register(capability.id.clone(), Arc::new(provider));
                }
                PluginCapabilityKind::Skill => {
                    let skill = ProtocolBackedSkill::from_descriptor(
                        plugin_id,
                        manifest,
                        capability.clone(),
                    );
                    let _ = self.skill_registry.register(Box::new(skill));
                }
                PluginCapabilityKind::Tool => {
                    let tool = ProtocolBackedTool::from_descriptor(
                        plugin_id,
                        manifest,
                        capability.clone(),
                    );
                    let _ = self.tool_registry.register(Arc::new(tool));
                }
                PluginCapabilityKind::Agent => {
                    let agent = ProtocolBackedAgent::from_descriptor(
                        plugin_id,
                        manifest,
                        capability.clone(),
                    );
                    let _ = self.agent_registry.register(Box::new(agent));
                }
                PluginCapabilityKind::Trigger | PluginCapabilityKind::Watcher => {
                    if let Some(trigger) =
                        Self::build_wakeup_trigger(plugin_id, manifest, capability, plugin_config)
                    {
                        self.sentinel_manager.register_trigger(trigger.clone());
                        triggers.push(trigger);
                    }
                }
                PluginCapabilityKind::Storage => {
                    let runtime = match manifest.runtime {
                        PluginType::Python => ProtocolStorageRuntime::Python,
                        PluginType::Remote => ProtocolStorageRuntime::Remote,
                        _ => {
                            remaining.push(capability.clone());
                            continue;
                        }
                    };
                    self.storage_backends.write().insert(
                        format!("{}:{}", plugin_id, capability.id),
                        Arc::new(ProtocolBackedStorage::new(
                            runtime,
                            manifest.entrypoint.clone(),
                            plugin_id.to_string(),
                        )),
                    );
                }
            }
        }
        if !triggers.is_empty() {
            self.trigger_cache
                .write()
                .insert(plugin_id.to_string(), triggers);
        }
        self.other_capabilities
            .write()
            .insert(plugin_id.to_string(), remaining);
        Ok(())
    }

    pub fn unregister_plugin(&self, plugin_id: &str) {
        if let Some(triggers) = self.trigger_cache.write().remove(plugin_id) {
            for trigger in triggers {
                self.sentinel_manager.unregister_trigger(&trigger.id);
            }
        }
        let plugin_tag = format!("plugin:{plugin_id}");
        for skill in self.skill_registry.list() {
            if skill.tags.iter().any(|tag| tag == &plugin_tag) {
                let _ = self.skill_registry.unregister(&skill.name);
            }
        }
        for tool in self.tool_registry.list() {
            if tool.tags.iter().any(|tag| tag == &plugin_tag) {
                let _ = self.tool_registry.unregister(&tool.name);
            }
        }
        for provider in self.llm_registry.list_providers() {
            if provider.contains(plugin_id) {
                let _ = self.llm_registry.unregister(&provider);
            }
        }
        for agent in self.agent_registry.list_agents() {
            if agent.config.name == format!("plugin:{plugin_id}") {
                let _ = self.agent_registry.unregister(agent.config.id);
            }
        }
        self.other_capabilities.write().remove(plugin_id);
        self.storage_backends
            .write()
            .retain(|key, _| !key.starts_with(&format!("{}:", plugin_id)));
    }

    pub fn snapshot(&self) -> HashMap<String, Vec<String>> {
        let mut snapshot = HashMap::new();
        snapshot.insert(
            "llm".to_string(),
            self.llm_registry.list_providers().iter().cloned().collect(),
        );
        snapshot.insert(
            "skill".to_string(),
            self.skill_registry
                .list()
                .iter()
                .map(|skill| skill.name.clone())
                .collect(),
        );
        snapshot.insert(
            "tool".to_string(),
            self.tool_registry
                .list()
                .iter()
                .map(|tool| tool.name.clone())
                .collect(),
        );
        snapshot.insert(
            "agent".to_string(),
            self.agent_registry
                .list_agents()
                .iter()
                .map(|agent| agent.config.name.clone())
                .collect(),
        );
        snapshot.insert(
            "trigger".to_string(),
            self.trigger_cache
                .read()
                .values()
                .flat_map(|triggers| triggers.iter().map(|trigger| trigger.id.clone()))
                .collect(),
        );
        snapshot.insert(
            "storage".to_string(),
            self.storage_backends
                .read()
                .keys()
                .map(ToString::to_string)
                .collect(),
        );
        for (plugin_id, capabilities) in self.other_capabilities.read().iter() {
            for capability in capabilities {
                snapshot
                    .entry(format!("{:?}", capability.kind))
                    .or_insert_with(Vec::new)
                    .push(format!("{}:{}", plugin_id, capability.id));
            }
        }
        snapshot
    }

    fn build_wakeup_trigger(
        plugin_id: &str,
        manifest: &PluginManifest,
        capability: &PluginCapabilityDescriptor,
        plugin_config: Option<&HashMap<String, serde_json::Value>>,
    ) -> Option<WakeupTrigger> {
        let agent_id =
            Self::resolve_trigger_agent_id(plugin_id, manifest, capability, plugin_config)?;
        let interval_secs = plugin_config
            .and_then(|config| config.get("interval_secs"))
            .and_then(|value| value.as_u64())
            .unwrap_or(60);
        let priority = plugin_config
            .and_then(|config| config.get("priority"))
            .and_then(|value| value.as_i64())
            .unwrap_or(0) as i32;
        let cooldown_secs = plugin_config
            .and_then(|config| config.get("cooldown_secs"))
            .and_then(|value| value.as_u64())
            .unwrap_or(30);
        let condition = if let Some(expression) = plugin_config
            .and_then(|config| config.get("cron"))
            .and_then(|value| value.as_str())
        {
            TriggerCondition::Cron {
                expression: expression.to_string(),
            }
        } else {
            TriggerCondition::Custom {
                name: capability.id.clone(),
                params: plugin_config
                    .and_then(|config| config.get("params"))
                    .cloned()
                    .unwrap_or_else(|| json!({ "plugin_id": plugin_id })),
            }
        };

        let wakeup_method = plugin_config
            .and_then(|config| config.get("wakeup_method"))
            .and_then(|value| value.as_str())
            .and_then(|name| match name {
                "execute_skill" => Some(WakeupMethod::ExecuteSkill {
                    skill: plugin_config
                        .and_then(|config| config.get("wakeup_skill"))
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    input: plugin_config
                        .and_then(|config| config.get("wakeup_input"))
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                }),
                "heartbeat" => Some(WakeupMethod::Heartbeat),
                _ => None,
            })
            .unwrap_or(WakeupMethod::Heartbeat);

        let mut trigger =
            WakeupTrigger::new(capability.id.clone(), agent_id, condition, wakeup_method);
        trigger.enabled = plugin_config
            .and_then(|config| config.get("enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        trigger.priority = priority;
        trigger.cooldown_secs = cooldown_secs.max(interval_secs);
        Some(trigger)
    }

    fn resolve_trigger_agent_id(
        plugin_id: &str,
        manifest: &PluginManifest,
        capability: &PluginCapabilityDescriptor,
        plugin_config: Option<&HashMap<String, serde_json::Value>>,
    ) -> Option<Uuid> {
        if let Some(agent_id) = plugin_config
            .and_then(|config| config.get("target_agent_id"))
            .and_then(|value| value.as_str())
            .and_then(|value| Uuid::parse_str(value).ok())
        {
            return Some(agent_id);
        }

        let target_plugin_id = plugin_config
            .and_then(|config| config.get("target_plugin_id"))
            .and_then(|value| value.as_str())
            .unwrap_or(plugin_id);
        let target_capability = plugin_config
            .and_then(|config| config.get("target_agent_capability"))
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .or_else(|| {
                manifest
                    .capabilities
                    .iter()
                    .find(|item| item.kind == PluginCapabilityKind::Agent)
                    .map(|item| item.id.clone())
            });

        target_capability
            .or_else(|| {
                if capability.kind == PluginCapabilityKind::Watcher {
                    Some(capability.id.clone())
                } else {
                    None
                }
            })
            .map(|capability_id| Self::deterministic_agent_id(target_plugin_id, &capability_id))
    }

    fn deterministic_agent_id(plugin_id: &str, capability_id: &str) -> Uuid {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        plugin_id.hash(&mut hasher);
        capability_id.hash(&mut hasher);
        let high = hasher.finish() as u128;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        capability_id.hash(&mut hasher);
        plugin_id.hash(&mut hasher);
        let low = hasher.finish() as u128;
        Uuid::from_u128((high << 64) | low)
    }
}

struct BridgeWakeupHandler {
    agent_registry: Arc<AgentRegistry>,
}

impl BridgeWakeupHandler {
    fn new(agent_registry: Arc<AgentRegistry>) -> Self {
        Self { agent_registry }
    }
}

#[async_trait]
impl AgentWakeupHandler for BridgeWakeupHandler {
    async fn handle_wakeup(
        &self,
        agent_id: Uuid,
        method: &WakeupMethod,
        _data: Option<serde_json::Value>,
    ) -> clawlegion_core::Result<()> {
        match method {
            WakeupMethod::Heartbeat => {
                let Some(agent) = self.agent_registry.get(agent_id) else {
                    return Err(CoreError::Agent(clawlegion_core::AgentError::NotFound(
                        agent_id.to_string(),
                    )));
                };
                let mut guard = agent.write().await;
                let ctx = HeartbeatContext {
                    trigger: HeartbeatTrigger::Scheduled,
                    timestamp: Utc::now(),
                };
                guard.heartbeat(ctx).await?;
                self.agent_registry.update_agent_info(agent_id).await?;
                Ok(())
            }
            WakeupMethod::ExecuteSkill { skill, input } => {
                let Some(agent) = self.agent_registry.get(agent_id) else {
                    return Err(CoreError::Agent(clawlegion_core::AgentError::NotFound(
                        agent_id.to_string(),
                    )));
                };

                let mut guard = agent.write().await;
                guard.load_skill(skill).await?;
                let ctx = HeartbeatContext {
                    trigger: HeartbeatTrigger::Custom {
                        trigger_id: "plugin-trigger".to_string(),
                        data: json!({"skill":skill,"input":input}),
                    },
                    timestamp: Utc::now(),
                };
                guard.heartbeat(ctx).await?;
                self.agent_registry.update_agent_info(agent_id).await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

struct ProtocolBackedLlmProvider {
    name: String,
    model: String,
    plugin_id: String,
    runtime: PluginType,
    entrypoint: String,
}

#[async_trait]
impl LlmProvider for ProtocolBackedLlmProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: LlmOptions,
    ) -> CoreResult<LlmResponse> {
        let request = LlmExecuteRequest::new(self.model.clone(), messages, options)?;
        let response =
            execute_llm(&self.runtime, &self.plugin_id, &self.entrypoint, &request).await?;
        Ok(response.into_llm_response())
    }

    async fn stream(
        &self,
        messages: Vec<LlmMessage>,
        options: LlmOptions,
    ) -> CoreResult<Box<dyn Stream<Item = CoreResult<StreamChunk>> + Send + Unpin>> {
        let request = LlmExecuteRequest::new(self.model.clone(), messages, options)?;
        let chunks = stream_llm(&self.runtime, &self.plugin_id, &self.entrypoint, &request).await?;
        let mapped = chunks
            .into_iter()
            .map(|chunk| Ok(chunk.into_stream_chunk()));
        Ok(Box::new(stream::iter(mapped)))
    }
}

struct ProtocolBackedSkill {
    metadata: SkillMetadata,
    plugin_id: String,
    runtime: PluginType,
    entrypoint: String,
}

struct ProtocolBackedTool {
    metadata: ToolMetadata,
    plugin_id: String,
    runtime: PluginType,
    entrypoint: String,
}

struct ProtocolBackedAgent {
    id: uuid::Uuid,
    info: AgentInfo,
    runtime: PluginType,
    entrypoint: String,
    plugin_id: String,
    capability_id: String,
    loaded_skills: Vec<String>,
}

impl ProtocolBackedTool {
    fn from_descriptor(
        plugin_id: &str,
        manifest: &PluginManifest,
        descriptor: PluginCapabilityDescriptor,
    ) -> Self {
        Self {
            metadata: ToolMetadata {
                name: descriptor
                    .display_name
                    .clone()
                    .unwrap_or_else(|| descriptor.id.clone()),
                version: manifest.version.clone(),
                description: descriptor
                    .description
                    .clone()
                    .unwrap_or_else(|| "Plugin-provided tool".to_string()),
                visibility: ToolVisibility::Private,
                tags: descriptor
                    .tags
                    .iter()
                    .cloned()
                    .chain(std::iter::once(format!("plugin:{plugin_id}")))
                    .collect(),
                input_schema: serde_json::json!({"type":"object"}),
                output_schema: Some(serde_json::json!({"type":"object"})),
                requires_llm: false,
            },
            plugin_id: plugin_id.to_string(),
            runtime: manifest.runtime.clone(),
            entrypoint: manifest.entrypoint.clone(),
        }
    }
}

impl ProtocolBackedAgent {
    fn from_descriptor(
        plugin_id: &str,
        manifest: &PluginManifest,
        descriptor: PluginCapabilityDescriptor,
    ) -> Self {
        let id = PluginBridgeHub::deterministic_agent_id(plugin_id, &descriptor.id);
        Self {
            id,
            info: AgentInfo {
                config: AgentConfig {
                    id,
                    company_id: uuid::Uuid::nil(),
                    name: format!("plugin:{plugin_id}"),
                    role: "plugin".to_string(),
                    title: descriptor
                        .description
                        .clone()
                        .unwrap_or_else(|| "Plugin-provided agent".to_string()),
                    agent_type: AgentTypeDef::Custom {
                        type_name: manifest.id.clone(),
                    },
                    icon: None,
                    reports_to: None,
                    capabilities: descriptor.id.clone(),
                    skills: Vec::new(),
                    budget_monthly_cents: None,
                    adapter_type: "plugin-bridge".to_string(),
                    adapter_config: HashMap::new(),
                    runtime_config: HashMap::new(),
                    tags: descriptor.tags.clone(),
                },
                status: AgentStatus::Idle,
                last_heartbeat_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            runtime: manifest.runtime.clone(),
            entrypoint: manifest.entrypoint.clone(),
            plugin_id: plugin_id.to_string(),
            capability_id: descriptor.id,
            loaded_skills: Vec::new(),
        }
    }

    async fn execute_action(&self, action: AgentAction) -> CoreResult<AgentActionResponse> {
        let request = AgentActionRequest {
            plugin_id: self.plugin_id.clone(),
            capability_id: self.capability_id.clone(),
            agent_id: self.id,
            action,
        };

        match self.runtime {
            PluginType::Python => {
                let payload = serde_json::to_string(&request).map_err(invalid_schema_error)?;
                let script_path = plugin_entrypoint_path(&self.plugin_id, &self.entrypoint);
                let output = Command::new("python3")
                    .arg(script_path)
                    .arg("--execute-agent-json")
                    .arg(payload)
                    .output()
                    .map_err(|error| {
                        CoreError::Agent(clawlegion_core::AgentError::ExecutionFailed(format!(
                            "python agent action failed to launch: {}",
                            error
                        )))
                    })?;
                if !output.status.success() {
                    return Err(CoreError::Agent(
                        clawlegion_core::AgentError::ExecutionFailed(
                            String::from_utf8_lossy(&output.stderr).trim().to_string(),
                        ),
                    ));
                }
                let response: AgentActionResponse =
                    serde_json::from_slice(&output.stdout).map_err(invalid_schema_error)?;
                response.validate()?;
                Ok(response)
            }
            PluginType::Remote => {
                let endpoint = format_endpoint(&self.entrypoint, "/agent");
                let response = reqwest::Client::new()
                    .post(endpoint)
                    .json(&request)
                    .send()
                    .await
                    .map_err(|error| {
                        CoreError::Agent(clawlegion_core::AgentError::ExecutionFailed(format!(
                            "remote agent action request failed: {}",
                            error
                        )))
                    })?;
                let status = response.status();
                let body = response.text().await.map_err(|error| {
                    CoreError::Agent(clawlegion_core::AgentError::ExecutionFailed(format!(
                        "remote agent action response read failed: {}",
                        error
                    )))
                })?;
                if !status.is_success() {
                    return Err(CoreError::Agent(
                        clawlegion_core::AgentError::ExecutionFailed(format!(
                            "remote agent action returned {}: {}",
                            status, body
                        )),
                    ));
                }
                let response: AgentActionResponse =
                    serde_json::from_str(&body).map_err(invalid_schema_error)?;
                response.validate()?;
                Ok(response)
            }
            _ => Err(CoreError::Agent(
                clawlegion_core::AgentError::ExecutionFailed(format!(
                    "unsupported agent runtime: {:?}",
                    self.runtime
                )),
            )),
        }
    }
}

impl ProtocolBackedSkill {
    fn from_descriptor(
        plugin_id: &str,
        manifest: &PluginManifest,
        descriptor: PluginCapabilityDescriptor,
    ) -> Self {
        let mut metadata = SkillMetadata::new(
            descriptor
                .display_name
                .clone()
                .unwrap_or_else(|| descriptor.id.clone()),
            "0.1.0",
            descriptor
                .description
                .clone()
                .unwrap_or_else(|| "Plugin-provided skill".to_string()),
        );
        metadata.visibility = Visibility::Private;
        metadata.skill_type = SkillType::Hybrid;
        metadata.execution_mode = ExecutionMode::Async;
        metadata.tags = descriptor
            .tags
            .iter()
            .cloned()
            .chain(std::iter::once(format!("plugin:{plugin_id}")))
            .collect();
        Self {
            metadata,
            plugin_id: plugin_id.to_string(),
            runtime: manifest.runtime.clone(),
            entrypoint: manifest.entrypoint.clone(),
        }
    }
}

#[async_trait]
impl Skill for ProtocolBackedSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    async fn init(&mut self, _ctx: &SkillContext) -> CoreResult<()> {
        Ok(())
    }

    async fn execute(&self, _ctx: &SkillContext, _input: SkillInput) -> CoreResult<SkillOutput> {
        match self.runtime {
            PluginType::Python => {
                let payload = serde_json::to_string(&_input).map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "failed to serialize skill input: {}",
                        error
                    )))
                })?;
                let script_path = plugin_entrypoint_path(&self.plugin_id, &self.entrypoint);
                let output = Command::new("python3")
                    .arg(script_path)
                    .arg("--execute-json")
                    .arg(payload)
                    .output()
                    .map_err(|error| {
                        CoreError::Llm(LlmError::RequestFailed(format!(
                            "python skill {} failed to launch: {}",
                            self.metadata.name, error
                        )))
                    })?;
                if !output.status.success() {
                    return Ok(SkillOutput::error(
                        String::from_utf8_lossy(&output.stderr).trim().to_string(),
                    ));
                }
                serde_json::from_slice::<SkillOutput>(&output.stdout).map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "python skill {} returned invalid json: {}",
                        self.metadata.name, error
                    )))
                })
            }
            PluginType::Remote => {
                let response = reqwest::Client::new()
                    .post(&self.entrypoint)
                    .json(&_input)
                    .send()
                    .await
                    .map_err(|error| {
                        CoreError::Llm(LlmError::RequestFailed(format!(
                            "remote skill {} request failed: {}",
                            self.metadata.name, error
                        )))
                    })?;
                let status = response.status();
                let body = response.text().await.map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "remote skill {} response read failed: {}",
                        self.metadata.name, error
                    )))
                })?;
                if !status.is_success() {
                    return Ok(SkillOutput::error(format!(
                        "remote skill {} returned {}: {}",
                        self.metadata.name, status, body
                    )));
                }
                serde_json::from_str::<SkillOutput>(&body).map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "remote skill {} returned invalid json: {}",
                        self.metadata.name, error
                    )))
                })
            }
            _ => Ok(SkillOutput::error(format!(
                "plugin-backed skill {} from {} is not executable with runtime {:?}",
                self.metadata.name, self.plugin_id, self.runtime
            ))),
        }
    }

    async fn on_event(&self, _ctx: &SkillContext, _event: SkillEvent) -> CoreResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> CoreResult<()> {
        Ok(())
    }
}

#[async_trait]
impl Tool for ProtocolBackedTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, _ctx: &ToolContext, args: serde_json::Value) -> CoreResult<ToolResult> {
        match self.runtime {
            PluginType::Python => {
                let payload = serde_json::to_string(&args).map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "failed to serialize tool args: {}",
                        error
                    )))
                })?;
                let script_path = plugin_entrypoint_path(&self.plugin_id, &self.entrypoint);
                let output = Command::new("python3")
                    .arg(script_path)
                    .arg("--execute-tool-json")
                    .arg(payload)
                    .output()
                    .map_err(|error| {
                        CoreError::Llm(LlmError::RequestFailed(format!(
                            "python tool {} failed to launch: {}",
                            self.metadata.name, error
                        )))
                    })?;
                if !output.status.success() {
                    return Ok(ToolResult::error(
                        String::from_utf8_lossy(&output.stderr).trim().to_string(),
                    ));
                }
                serde_json::from_slice::<ToolResult>(&output.stdout).map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "python tool {} returned invalid json: {}",
                        self.metadata.name, error
                    )))
                })
            }
            PluginType::Remote => {
                let response = reqwest::Client::new()
                    .post(&self.entrypoint)
                    .json(&json!({
                        "plugin_id": self.plugin_id,
                        "tool": self.metadata.name,
                        "args": args,
                    }))
                    .send()
                    .await
                    .map_err(|error| {
                        CoreError::Llm(LlmError::RequestFailed(format!(
                            "remote tool {} request failed: {}",
                            self.metadata.name, error
                        )))
                    })?;
                let value = response
                    .json::<serde_json::Value>()
                    .await
                    .map_err(|error| {
                        CoreError::Llm(LlmError::RequestFailed(format!(
                            "remote tool {} returned invalid json: {}",
                            self.metadata.name, error
                        )))
                    })?;
                if let Some(success) = value.get("success").and_then(|item| item.as_bool()) {
                    Ok(ToolResult {
                        success,
                        data: value.get("data").cloned(),
                        error: value
                            .get("error")
                            .and_then(|item| item.as_str())
                            .map(ToString::to_string),
                        execution_time_ms: value
                            .get("execution_time_ms")
                            .and_then(|item| item.as_u64())
                            .unwrap_or(0),
                    })
                } else {
                    Ok(ToolResult::success(value))
                }
            }
            PluginType::Config => Ok(ToolResult::success(json!({
                "plugin_id": self.plugin_id,
                "tool": self.metadata.name,
                "args": args,
                "runtime": "config",
            }))),
            _ => Ok(ToolResult::error(format!(
                "plugin-backed tool {} from {} is not executable with runtime {:?}",
                self.metadata.name, self.plugin_id, self.runtime
            ))),
        }
    }
}

#[async_trait]
impl Agent for ProtocolBackedAgent {
    fn id(&self) -> uuid::Uuid {
        self.id
    }

    fn info(&self) -> AgentInfo {
        self.info.clone()
    }

    fn set_status(&mut self, status: AgentStatus) {
        self.info.status = status;
        self.info.updated_at = Utc::now();
    }

    async fn heartbeat(&mut self, ctx: HeartbeatContext) -> CoreResult<HeartbeatResult> {
        let response = self
            .execute_action(AgentAction::Heartbeat {
                trigger: format!("{:?}", ctx.trigger),
                timestamp_rfc3339: ctx.timestamp.to_rfc3339(),
            })
            .await?;
        self.info.last_heartbeat_at = Some(Utc::now());
        self.info.updated_at = Utc::now();
        self.info.status = if response.success {
            AgentStatus::Idle
        } else {
            AgentStatus::Error
        };
        Ok(response.into_heartbeat_result())
    }

    async fn load_skill(&mut self, skill_name: &str) -> CoreResult<()> {
        let response = self
            .execute_action(AgentAction::LoadSkill {
                skill_name: skill_name.to_string(),
            })
            .await?;
        if !response.success {
            return Err(CoreError::Agent(
                clawlegion_core::AgentError::ExecutionFailed(
                    response
                        .error
                        .unwrap_or_else(|| "agent load_skill failed".to_string()),
                ),
            ));
        }
        if !self.loaded_skills.iter().any(|skill| skill == skill_name) {
            self.loaded_skills.push(skill_name.to_string());
        }
        Ok(())
    }

    async fn unload_skill(&mut self, skill_name: &str) -> CoreResult<()> {
        let response = self
            .execute_action(AgentAction::UnloadSkill {
                skill_name: skill_name.to_string(),
            })
            .await?;
        if !response.success {
            return Err(CoreError::Agent(
                clawlegion_core::AgentError::ExecutionFailed(
                    response
                        .error
                        .unwrap_or_else(|| "agent unload_skill failed".to_string()),
                ),
            ));
        }
        self.loaded_skills.retain(|skill| skill != skill_name);
        Ok(())
    }

    fn loaded_skills(&self) -> Vec<String> {
        self.loaded_skills.clone()
    }

    async fn shutdown(&mut self) -> CoreResult<()> {
        let response = self.execute_action(AgentAction::Shutdown).await?;
        if !response.success {
            return Err(CoreError::Agent(
                clawlegion_core::AgentError::ExecutionFailed(
                    response
                        .error
                        .unwrap_or_else(|| "agent shutdown failed".to_string()),
                ),
            ));
        }
        self.set_status(AgentStatus::Stopping);
        Ok(())
    }
}

fn plugin_entrypoint_path(plugin_id: &str, entrypoint: &str) -> PathBuf {
    if PathBuf::from(entrypoint).is_absolute() {
        PathBuf::from(entrypoint)
    } else {
        PathBuf::from("./plugins").join(plugin_id).join(entrypoint)
    }
}

fn format_endpoint(base: &str, suffix: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        suffix.trim_start_matches('/')
    )
}

fn invalid_schema_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::Llm(LlmError::RequestFailed(format!(
        "invalid protocol payload: {}",
        error
    )))
}

async fn execute_llm(
    runtime: &PluginType,
    plugin_id: &str,
    entrypoint: &str,
    request: &LlmExecuteRequest,
) -> CoreResult<LlmExecuteResponse> {
    match runtime {
        PluginType::Python => {
            let payload = serde_json::to_string(request).map_err(invalid_schema_error)?;
            let script_path = plugin_entrypoint_path(plugin_id, entrypoint);
            let output = Command::new("python3")
                .arg(script_path)
                .arg("--execute-llm-json")
                .arg(payload)
                .output()
                .map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "python llm request failed to launch: {}",
                        error
                    )))
                })?;
            if !output.status.success() {
                return Err(CoreError::Llm(LlmError::RequestFailed(
                    String::from_utf8_lossy(&output.stderr).trim().to_string(),
                )));
            }
            let response: LlmExecuteResponse =
                serde_json::from_slice(&output.stdout).map_err(invalid_schema_error)?;
            response.validate()?;
            Ok(response)
        }
        PluginType::Remote => {
            let endpoint = format_endpoint(entrypoint, "/llm/chat");
            let response = reqwest::Client::new()
                .post(endpoint)
                .json(request)
                .send()
                .await
                .map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "remote llm chat request failed: {}",
                        error
                    )))
                })?;
            let status = response.status();
            let body = response.text().await.map_err(|error| {
                CoreError::Llm(LlmError::RequestFailed(format!(
                    "remote llm chat response read failed: {}",
                    error
                )))
            })?;
            if !status.is_success() {
                return Err(CoreError::Llm(LlmError::RequestFailed(format!(
                    "remote llm chat returned {}: {}",
                    status, body
                ))));
            }
            let response: LlmExecuteResponse =
                serde_json::from_str(&body).map_err(invalid_schema_error)?;
            response.validate()?;
            Ok(response)
        }
        _ => Err(CoreError::Llm(LlmError::RequestFailed(format!(
            "unsupported llm runtime: {:?}",
            runtime
        )))),
    }
}

async fn stream_llm(
    runtime: &PluginType,
    plugin_id: &str,
    entrypoint: &str,
    request: &LlmExecuteRequest,
) -> CoreResult<Vec<LlmStreamChunk>> {
    match runtime {
        PluginType::Python => {
            let payload = serde_json::to_string(request).map_err(invalid_schema_error)?;
            let script_path = plugin_entrypoint_path(plugin_id, entrypoint);
            let output = Command::new("python3")
                .arg(script_path)
                .arg("--execute-llm-json")
                .arg(payload)
                .arg("--stream")
                .output()
                .map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "python llm stream failed to launch: {}",
                        error
                    )))
                })?;
            if !output.status.success() {
                return Err(CoreError::Llm(LlmError::RequestFailed(
                    String::from_utf8_lossy(&output.stderr).trim().to_string(),
                )));
            }
            let chunks: Vec<LlmStreamChunk> =
                serde_json::from_slice(&output.stdout).map_err(invalid_schema_error)?;
            if chunks.is_empty() {
                return Err(CoreError::Llm(LlmError::RequestFailed(
                    "stream response must contain at least one chunk".to_string(),
                )));
            }
            Ok(chunks)
        }
        PluginType::Remote => {
            let endpoint = format_endpoint(entrypoint, "/llm/stream");
            let response = reqwest::Client::new()
                .post(endpoint)
                .header("Accept", "text/event-stream")
                .json(request)
                .send()
                .await
                .map_err(|error| {
                    CoreError::Llm(LlmError::RequestFailed(format!(
                        "remote llm stream request failed: {}",
                        error
                    )))
                })?;
            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_else(|_| "".to_string());
                return Err(CoreError::Llm(LlmError::RequestFailed(format!(
                    "remote llm stream returned {}: {}",
                    status, body
                ))));
            }
            let body = response.text().await.map_err(|error| {
                CoreError::Llm(LlmError::RequestFailed(format!(
                    "remote llm stream response read failed: {}",
                    error
                )))
            })?;
            parse_sse_chunks(&body)
        }
        _ => Err(CoreError::Llm(LlmError::RequestFailed(format!(
            "unsupported llm runtime: {:?}",
            runtime
        )))),
    }
}

fn parse_sse_chunks(body: &str) -> CoreResult<Vec<LlmStreamChunk>> {
    let mut chunks = Vec::new();
    let mut current_event = String::new();

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current_event.is_empty() {
                let chunk = parse_sse_event(&current_event)?;
                chunks.push(chunk);
                current_event.clear();
            }
            continue;
        }

        if let Some(data) = trimmed.strip_prefix("data:") {
            if !current_event.is_empty() {
                current_event.push('\n');
            }
            current_event.push_str(data.trim());
        }
    }

    if !current_event.is_empty() {
        chunks.push(parse_sse_event(&current_event)?);
    }

    if chunks.is_empty() {
        return Err(CoreError::Llm(LlmError::RequestFailed(
            "sse stream contained no data chunks".to_string(),
        )));
    }

    Ok(chunks)
}

fn parse_sse_event(data: &str) -> CoreResult<LlmStreamChunk> {
    if data == "[DONE]" {
        return Ok(LlmStreamChunk {
            delta: String::new(),
            finish_reason: Some("stop".to_string()),
            usage: None,
        });
    }

    let chunk: LlmStreamChunk = serde_json::from_str(data).map_err(invalid_schema_error)?;
    chunk.validate()?;
    Ok(chunk)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;
    use clawlegion_core::{PluginMetadata, PluginType};

    fn test_manifest(id: &str, runtime: PluginType, entrypoint: String) -> PluginManifest {
        PluginManifest {
            id: id.to_string(),
            version: "0.1.0".to_string(),
            api_version: "v2".to_string(),
            runtime,
            entrypoint,
            metadata: PluginMetadata {
                name: id.to_string(),
                version: "0.1.0".to_string(),
                description: "test".to_string(),
                author: "test".to_string(),
                core_version: "0.1.0".to_string(),
                dependencies: Vec::new(),
                tags: Vec::new(),
            },
            capabilities: Vec::new(),
            permissions: Vec::new(),
            dependencies: Vec::new(),
            compatible_host_versions: vec!["0.1.0".to_string()],
            signature: None,
            healthcheck: None,
            config_schema: None,
            ui_metadata: clawlegion_core::PluginUiMetadata::default(),
        }
    }

    #[test]
    fn registers_and_executes_python_skill() {
        let temp_root =
            std::env::temp_dir().join(format!("clawlegion-python-skill-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_root).expect("create temp dir");
        let script_path = temp_root.join("plugin_main.py");
        fs::write(
            &script_path,
            r#"#!/usr/bin/env python3
import json
import sys

if "--execute-json" in sys.argv:
    payload = json.loads(sys.argv[sys.argv.index("--execute-json") + 1])
    print(json.dumps({
        "text": f"echo:{payload.get('text', '')}",
        "data": {"echo": payload.get("text", "")},
        "success": True,
        "error": None,
        "follow_ups": []
    }))
else:
    print("ok")
"#,
        )
        .expect("write script");

        let hub = PluginBridgeHub::new();
        let manifest = test_manifest(
            "python-test",
            PluginType::Python,
            script_path.to_string_lossy().to_string(),
        );
        let capability = PluginCapabilityDescriptor {
            id: "skill.python_test".to_string(),
            kind: PluginCapabilityKind::Skill,
            display_name: Some("Python Test".to_string()),
            description: Some("test skill".to_string()),
            interface: Some("clawlegion.skill.v1".to_string()),
            tags: vec!["test".to_string()],
        };

        hub.register_plugin_capabilities("python-test", &manifest, &[capability], None)
            .expect("register capabilities");
        let skill = hub
            .skill_registry()
            .get("Python Test")
            .expect("skill registered");
        let output = tokio::runtime::Runtime::new()
            .expect("create runtime")
            .block_on(skill.read().execute(
                &SkillContext::new(uuid::Uuid::nil()),
                SkillInput::text("hello"),
            ))
            .expect("execute skill");

        assert_eq!(output.text.as_deref(), Some("echo:hello"));

        fs::remove_dir_all(&temp_root).expect("cleanup");
    }

    #[test]
    fn parses_remote_llm_chat_response() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind server");
        let addr = listener.local_addr().expect("addr");
        let server = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer);
                let body = r#"{"text":"remote-chat","usage":{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3},"finish_reason":"stop"}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let provider = ProtocolBackedLlmProvider {
            name: "remote-llm".to_string(),
            model: "test-model".to_string(),
            plugin_id: "remote-plugin".to_string(),
            runtime: PluginType::Remote,
            entrypoint: format!("http://{}", addr),
        };

        let response = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(provider.chat(vec![LlmMessage::user("hello")], LlmOptions::default()))
            .expect("chat success");

        assert_eq!(response.content.as_deref(), Some("remote-chat"));
        assert_eq!(response.usage.total_tokens, 3);
        server.join().expect("join server");
    }

    #[test]
    fn parses_remote_llm_sse_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind server");
        let addr = listener.local_addr().expect("addr");
        let server = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer);
                let body = "data: {\"delta\":\"hello\",\"finish_reason\":null}\n\n\
                            data: {\"delta\":\" world\",\"finish_reason\":\"stop\"}\n\n\
                            data: [DONE]\n\n";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let provider = ProtocolBackedLlmProvider {
            name: "remote-llm".to_string(),
            model: "test-model".to_string(),
            plugin_id: "remote-plugin".to_string(),
            runtime: PluginType::Remote,
            entrypoint: format!("http://{}", addr),
        };

        let mut stream = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(provider.stream(vec![LlmMessage::user("hello")], LlmOptions::default()))
            .expect("stream success");

        let mut collected = String::new();
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                use futures_util::StreamExt;
                while let Some(item) = stream.next().await {
                    let chunk = item.expect("chunk");
                    collected.push_str(&chunk.delta);
                }
            });

        assert!(collected.contains("hello"));
        server.join().expect("join server");
    }

    #[test]
    fn executes_remote_agent_heartbeat() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind server");
        let addr = listener.local_addr().expect("addr");
        let server = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer);
                let body = r#"{"success":true,"heartbeat":{"success":true,"completed_tasks":[],"created_tasks":[],"sent_messages":[],"error":null,"usage":{"tokens":0,"cost_cents":0,"execution_time_ms":1}},"loaded_skills":[],"error":null}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let manifest = test_manifest("agent-test", PluginType::Remote, format!("http://{}", addr));
        let capability = PluginCapabilityDescriptor {
            id: "agent.remote".to_string(),
            kind: PluginCapabilityKind::Agent,
            display_name: Some("Remote Agent".to_string()),
            description: Some("agent".to_string()),
            interface: Some("clawlegion.agent.v2".to_string()),
            tags: vec![],
        };

        let mut agent = ProtocolBackedAgent::from_descriptor("agent-test", &manifest, capability);
        let result = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(agent.heartbeat(HeartbeatContext {
                trigger: HeartbeatTrigger::Scheduled,
                timestamp: Utc::now(),
            }))
            .expect("heartbeat success");

        assert!(result.success);
        server.join().expect("join server");
    }

    #[test]
    fn registers_plugin_triggers_with_sentinel() {
        let hub = PluginBridgeHub::new();
        let manifest = PluginManifest {
            id: "trigger-test".to_string(),
            version: "0.1.0".to_string(),
            api_version: "v2".to_string(),
            runtime: PluginType::Config,
            entrypoint: "plugin.toml".to_string(),
            capabilities: vec![
                PluginCapabilityDescriptor {
                    id: "agent.trigger_test".to_string(),
                    kind: PluginCapabilityKind::Agent,
                    display_name: Some("Trigger Test Agent".to_string()),
                    description: Some("test agent".to_string()),
                    interface: Some("clawlegion.agent.v1".to_string()),
                    tags: vec!["test".to_string()],
                },
                PluginCapabilityDescriptor {
                    id: "trigger.trigger_test".to_string(),
                    kind: PluginCapabilityKind::Trigger,
                    display_name: Some("Trigger Test".to_string()),
                    description: Some("test trigger".to_string()),
                    interface: Some("clawlegion.trigger.v1".to_string()),
                    tags: vec!["test".to_string()],
                },
            ],
            permissions: Vec::new(),
            dependencies: Vec::new(),
            compatible_host_versions: Vec::new(),
            signature: None,
            healthcheck: None,
            config_schema: None,
            ui_metadata: Default::default(),
            metadata: Default::default(),
        };
        let mut plugin_config = HashMap::new();
        plugin_config.insert(
            "target_agent_capability".to_string(),
            serde_json::Value::String("agent.trigger_test".to_string()),
        );
        plugin_config.insert(
            "cron".to_string(),
            serde_json::Value::String("*/5 * * * *".to_string()),
        );

        hub.register_plugin_capabilities(
            "trigger-test",
            &manifest,
            &manifest.capabilities,
            Some(&plugin_config),
        )
        .expect("register capabilities");

        let triggers = hub.sentinel_manager().list_triggers();
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].id, "trigger.trigger_test");
        assert_eq!(
            triggers[0].agent_id,
            PluginBridgeHub::deterministic_agent_id("trigger-test", "agent.trigger_test")
        );

        hub.unregister_plugin("trigger-test");
        assert!(hub.sentinel_manager().list_triggers().is_empty());
    }
}
