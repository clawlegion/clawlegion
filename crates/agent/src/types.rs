//! Agent types and implementations

pub use clawlegion_core::{
    Agent, AgentConfig, AgentStatus, HeartbeatContext, HeartbeatResult, HeartbeatTrigger,
};

use async_trait::async_trait;
use clawlegion_core::{AgentError, AgentInfo, LlmProviderConfig, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Base agent implementation
pub struct BaseAgent {
    config: AgentConfig,
    status: AgentStatus,
    info: AgentInfo,
    loaded_skills: Vec<String>,
    capabilities: Arc<dyn AgentCapabilities>,
}

impl BaseAgent {
    /// Create a new base agent
    pub fn new(config: AgentConfig, capabilities: Arc<dyn AgentCapabilities>) -> Self {
        let info = AgentInfo::new(config.clone());

        Self {
            config,
            status: AgentStatus::Initializing,
            info,
            loaded_skills: vec![],
            capabilities,
        }
    }

    /// Get agent config
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Get loaded skills
    pub fn loaded_skills(&self) -> Vec<String> {
        self.loaded_skills.clone()
    }
}

#[async_trait]
impl Agent for BaseAgent {
    fn id(&self) -> clawlegion_core::AgentId {
        self.config.id
    }

    fn info(&self) -> AgentInfo {
        AgentInfo {
            config: self.config.clone(),
            status: self.status,
            last_heartbeat_at: self.info.last_heartbeat_at,
            created_at: self.info.created_at,
            updated_at: self.info.updated_at,
        }
    }

    fn set_status(&mut self, status: AgentStatus) {
        self.status = status;
        self.info.updated_at = chrono::Utc::now();
    }

    async fn heartbeat(&mut self, ctx: HeartbeatContext) -> Result<HeartbeatResult> {
        self.status = AgentStatus::Running;

        // Execute heartbeat logic
        let result = self.capabilities.execute_heartbeat(self, &ctx).await;

        self.status = AgentStatus::Idle;
        self.info.last_heartbeat_at = Some(chrono::Utc::now());

        result
    }

    async fn load_skill(&mut self, skill_name: &str) -> Result<()> {
        if !self.loaded_skills.iter().any(|s| s == skill_name) {
            self.loaded_skills.push(skill_name.to_string());
        }
        Ok(())
    }

    async fn unload_skill(&mut self, skill_name: &str) -> Result<()> {
        self.loaded_skills.retain(|s| s != skill_name);
        Ok(())
    }

    fn loaded_skills(&self) -> Vec<String> {
        self.loaded_skills.clone()
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.status = AgentStatus::Stopping;
        Ok(())
    }
}

/// Agent capabilities trait
///
/// Implement this trait to define custom agent behavior.
#[async_trait::async_trait]
pub trait AgentCapabilities: Send + Sync {
    /// Execute heartbeat logic
    async fn execute_heartbeat(
        &self,
        agent: &BaseAgent,
        ctx: &HeartbeatContext,
    ) -> Result<HeartbeatResult>;

    /// Called when agent receives a message
    async fn on_message_received(
        &self,
        _agent: &BaseAgent,
        _message: &clawlegion_core::ChatMessage,
    ) -> Result<()> {
        Ok(())
    }

    /// Called when agent is assigned a task
    async fn on_task_assigned(&self, _agent: &BaseAgent, _task_id: uuid::Uuid) -> Result<()> {
        Ok(())
    }

    /// Called when agent is mentioned in a group
    async fn on_mentioned(
        &self,
        _agent: &BaseAgent,
        _message: &clawlegion_core::ChatMessage,
    ) -> Result<()> {
        Ok(())
    }
}

/// React Agent implementation
pub struct ReactAgent {
    base: BaseAgent,
}

impl ReactAgent {
    /// Create a new React agent
    pub fn new(config: AgentConfig, capabilities: Arc<dyn AgentCapabilities>) -> Self {
        let base = BaseAgent::new(config, capabilities);
        Self { base }
    }

    pub fn base(&self) -> &BaseAgent {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

#[async_trait]
impl Agent for ReactAgent {
    fn id(&self) -> clawlegion_core::AgentId {
        self.base.id()
    }

    fn info(&self) -> AgentInfo {
        self.base.info()
    }

    fn set_status(&mut self, status: AgentStatus) {
        self.base.set_status(status);
    }

    async fn heartbeat(&mut self, ctx: HeartbeatContext) -> Result<HeartbeatResult> {
        // React agent specific logic: Reasoning + Acting cycle
        tracing::info!("ReactAgent {} starting heartbeat", self.id());

        let result = self.base.heartbeat(ctx).await?;

        // Add React-specific post-processing
        // In a real implementation, this would involve:
        // 1. Reasoning about the current state
        // 2. Deciding on actions
        // 3. Executing actions
        // 4. Observing results
        // 5. Repeating until goal is reached

        Ok(result)
    }

    async fn load_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.load_skill(skill_name).await
    }

    async fn unload_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.unload_skill(skill_name).await
    }

    fn loaded_skills(&self) -> Vec<String> {
        self.base.loaded_skills()
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.base.shutdown().await
    }
}

/// Flow Agent implementation
pub struct FlowAgent {
    base: BaseAgent,
    flows: HashMap<String, FlowDefinition>,
}

impl FlowAgent {
    /// Create a new Flow agent
    pub fn new(config: AgentConfig, capabilities: Arc<dyn AgentCapabilities>) -> Self {
        let base = BaseAgent::new(config, capabilities);
        Self {
            base,
            flows: HashMap::new(),
        }
    }

    /// Register a flow
    pub fn register_flow(&mut self, flow: FlowDefinition) {
        self.flows.insert(flow.name.clone(), flow);
    }

    pub fn base(&self) -> &BaseAgent {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

#[async_trait]
impl Agent for FlowAgent {
    fn id(&self) -> clawlegion_core::AgentId {
        self.base.id()
    }

    fn info(&self) -> AgentInfo {
        self.base.info()
    }

    fn set_status(&mut self, status: AgentStatus) {
        self.base.set_status(status);
    }

    async fn heartbeat(&mut self, ctx: HeartbeatContext) -> Result<HeartbeatResult> {
        tracing::info!("FlowAgent {} starting heartbeat", self.id());

        // Flow agent specific logic: Execute predefined flows
        let result = self.base.heartbeat(ctx).await?;

        // In a real implementation, this would:
        // 1. Determine which flow to execute based on trigger
        // 2. Execute flow steps in order
        // 3. Handle flow branching and conditions

        Ok(result)
    }

    async fn load_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.load_skill(skill_name).await
    }

    async fn unload_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.unload_skill(skill_name).await
    }

    fn loaded_skills(&self) -> Vec<String> {
        self.base.loaded_skills()
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.base.shutdown().await
    }
}

/// Flow definition
#[derive(Debug, Clone)]
pub struct FlowDefinition {
    pub name: String,
    pub description: String,
    pub steps: Vec<FlowStep>,
    pub triggers: Vec<FlowTrigger>,
}

/// Flow step
#[derive(Debug, Clone)]
pub struct FlowStep {
    pub name: String,
    pub action: FlowAction,
    pub condition: Option<FlowCondition>,
}

/// Flow action
#[derive(Debug, Clone)]
pub enum FlowAction {
    SendMessage {
        recipient: String,
        content: String,
    },
    CallTool {
        tool: String,
        args: serde_json::Value,
    },
    ExecuteSkill {
        skill: String,
        input: serde_json::Value,
    },
    WaitForEvent {
        event_type: String,
        timeout_secs: Option<u64>,
    },
    Branch {
        condition: FlowCondition,
        true_step: String,
        false_step: String,
    },
}

/// Flow trigger
#[derive(Debug, Clone)]
pub enum FlowTrigger {
    MessageReceived { from: Option<String> },
    TaskAssigned,
    Scheduled { cron: String },
    Manual,
}

/// Flow condition
#[derive(Debug, Clone)]
pub struct FlowCondition {
    pub expression: String,
}

/// Normal Agent implementation (no LLM)
pub struct NormalAgent {
    base: BaseAgent,
    rules: Vec<AgentRule>,
}

impl NormalAgent {
    /// Create a new Normal agent
    pub fn new(config: AgentConfig, capabilities: Arc<dyn AgentCapabilities>) -> Self {
        let base = BaseAgent::new(config, capabilities);
        Self {
            base,
            rules: vec![],
        }
    }

    /// Register a rule
    pub fn register_rule(&mut self, rule: AgentRule) {
        self.rules.push(rule);
    }

    pub fn base(&self) -> &BaseAgent {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

#[async_trait]
impl Agent for NormalAgent {
    fn id(&self) -> clawlegion_core::AgentId {
        self.base.id()
    }

    fn info(&self) -> AgentInfo {
        self.base.info()
    }

    fn set_status(&mut self, status: AgentStatus) {
        self.base.set_status(status);
    }

    async fn heartbeat(&mut self, ctx: HeartbeatContext) -> Result<HeartbeatResult> {
        tracing::info!("NormalAgent {} starting heartbeat", self.id());

        // Clone trigger before passing ctx to avoid borrow-after-move
        let trigger = ctx.trigger.clone();

        // Normal agent: Rule-based execution without LLM
        let result = self.base.heartbeat(ctx).await?;

        // Evaluate rules - collect status updates to apply after borrowing ends
        let mut status_updates: Vec<AgentStatus> = Vec::new();

        for rule in &self.rules {
            if rule.condition.matches(&trigger) {
                match &rule.action {
                    RuleAction::SendMessage { content: _ } => {
                        tracing::info!("Rule triggered: Send message");
                    }
                    RuleAction::CallTool { tool, args: _ } => {
                        tracing::info!("Rule triggered: Call tool {}", tool);
                    }
                    RuleAction::UpdateStatus { status } => {
                        status_updates.push(*status);
                    }
                }
            }
        }

        // Apply status updates after borrowing self.rules is done
        for status in status_updates {
            self.set_status(status);
        }

        Ok(result)
    }

    async fn load_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.load_skill(skill_name).await
    }

    async fn unload_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.unload_skill(skill_name).await
    }

    fn loaded_skills(&self) -> Vec<String> {
        self.base.loaded_skills()
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.base.shutdown().await
    }
}

/// Codex Agent implementation
pub struct CodexAgent {
    base: BaseAgent,
    runner: crate::CodexCliRunner,
}

impl CodexAgent {
    /// Create a new Codex agent
    pub fn new(config: AgentConfig, capabilities: Arc<dyn AgentCapabilities>) -> Self {
        let base = BaseAgent::new(config, capabilities);
        Self {
            base,
            runner: crate::CodexCliRunner::default(),
        }
    }

    fn provider_config(&self) -> Result<LlmProviderConfig> {
        let config = self.base.config();
        let raw = config
            .adapter_config
            .get("llm_provider")
            .cloned()
            .unwrap_or_else(|| {
                serde_json::Value::Object(config.adapter_config.clone().into_iter().collect())
            });

        serde_json::from_value(raw).map_err(|error| {
            clawlegion_core::Error::Agent(AgentError::ExecutionFailed(format!(
                "invalid Codex LLM provider config for agent '{}': {}",
                config.name, error
            )))
        })
    }

    fn runtime_string(&self, key: &str) -> Option<String> {
        self.base
            .config()
            .runtime_config
            .get(key)
            .map(runtime_value_to_string)
            .filter(|value| !value.is_empty())
    }

    fn working_directory(&self) -> Option<PathBuf> {
        self.base
            .config()
            .runtime_config
            .get("working_directory")
            .and_then(|value| value.as_str())
            .map(PathBuf::from)
    }
}

#[async_trait]
impl Agent for CodexAgent {
    fn id(&self) -> clawlegion_core::AgentId {
        self.base.id()
    }

    fn info(&self) -> AgentInfo {
        self.base.info()
    }

    fn set_status(&mut self, status: AgentStatus) {
        self.base.set_status(status);
    }

    async fn heartbeat(&mut self, ctx: HeartbeatContext) -> Result<HeartbeatResult> {
        tracing::info!("CodexAgent {} starting heartbeat", self.id());

        self.base.status = AgentStatus::Running;
        self.base.info.updated_at = chrono::Utc::now();

        let provider_config = self.provider_config()?;
        let _started_at = std::time::Instant::now();
        let request = crate::CodexRunRequest {
            prompt: crate::build_codex_prompt(
                &self.base.config.name,
                &self.base.config.role,
                &self.base.config.title,
                &self.base.config.capabilities,
                &ctx.trigger,
            ),
            working_directory: self.working_directory(),
            profile: provider_config
                .extra
                .get("profile")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("profile")),
            sandbox: provider_config
                .extra
                .get("sandbox")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("sandbox")),
            approval_policy: provider_config
                .extra
                .get("approval_policy")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("approval_policy")),
            system_prompt: provider_config
                .extra
                .get("system_prompt")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("system_prompt")),
            web_search: provider_config
                .extra
                .get("web_search")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("web_search")),
            provider_config,
        };

        let outcome = self.runner.run(&request).await;
        self.base.status = AgentStatus::Idle;
        self.base.info.last_heartbeat_at = Some(chrono::Utc::now());
        self.base.info.updated_at = chrono::Utc::now();

        match outcome {
            Ok(result) => {
                tracing::info!(
                    agent_id = %self.id(),
                    event_count = result.raw_events.len(),
                    final_message = %result.final_message,
                    "Codex agent completed heartbeat"
                );

                Ok(HeartbeatResult {
                    success: true,
                    completed_tasks: vec![],
                    created_tasks: vec![],
                    sent_messages: vec![],
                    error: None,
                })
            }
            Err(error) => {
                self.base.status = AgentStatus::Error;
                self.base.info.updated_at = chrono::Utc::now();
                Err(error)
            }
        }
    }

    async fn load_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.load_skill(skill_name).await
    }

    async fn unload_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.unload_skill(skill_name).await
    }

    fn loaded_skills(&self) -> Vec<String> {
        self.base.loaded_skills()
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.base.shutdown().await
    }
}

/// Claude Code Agent implementation
pub struct ClaudeCodeAgent {
    base: BaseAgent,
    runner: crate::ClaudeCodeCliRunner,
}

impl ClaudeCodeAgent {
    /// Create a new Claude Code agent
    pub fn new(config: AgentConfig, capabilities: Arc<dyn AgentCapabilities>) -> Self {
        let base = BaseAgent::new(config, capabilities);
        Self {
            base,
            runner: crate::ClaudeCodeCliRunner::default(),
        }
    }

    fn provider_config(&self) -> Result<LlmProviderConfig> {
        let config = self.base.config();
        let raw = config
            .adapter_config
            .get("llm_provider")
            .cloned()
            .unwrap_or_else(|| {
                serde_json::Value::Object(config.adapter_config.clone().into_iter().collect())
            });

        serde_json::from_value(raw).map_err(|error| {
            clawlegion_core::Error::Agent(AgentError::ExecutionFailed(format!(
                "invalid ClaudeCode LLM provider config for agent '{}': {}",
                config.name, error
            )))
        })
    }

    fn runtime_string(&self, key: &str) -> Option<String> {
        self.base
            .config()
            .runtime_config
            .get(key)
            .map(runtime_value_to_string)
            .filter(|value| !value.is_empty())
    }

    fn working_directory(&self) -> Option<PathBuf> {
        self.base
            .config()
            .runtime_config
            .get("working_directory")
            .and_then(|value| value.as_str())
            .map(PathBuf::from)
    }
}

#[async_trait]
impl Agent for ClaudeCodeAgent {
    fn id(&self) -> clawlegion_core::AgentId {
        self.base.id()
    }

    fn info(&self) -> AgentInfo {
        self.base.info()
    }

    fn set_status(&mut self, status: AgentStatus) {
        self.base.set_status(status);
    }

    async fn heartbeat(&mut self, ctx: HeartbeatContext) -> Result<HeartbeatResult> {
        tracing::info!("ClaudeCodeAgent {} starting heartbeat", self.id());

        self.base.status = AgentStatus::Running;
        self.base.info.updated_at = chrono::Utc::now();

        let provider_config = self.provider_config()?;
        let _started_at = std::time::Instant::now();
        let request = crate::ClaudeCodeRunRequest {
            prompt: crate::build_claude_code_prompt(
                &self.base.config.name,
                &self.base.config.role,
                &self.base.config.title,
                &self.base.config.capabilities,
                &ctx.trigger,
            ),
            working_directory: self.working_directory(),
            profile: provider_config
                .extra
                .get("profile")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("profile")),
            sandbox: provider_config
                .extra
                .get("sandbox")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("sandbox")),
            approval_policy: provider_config
                .extra
                .get("approval_policy")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("approval_policy")),
            system_prompt: provider_config
                .extra
                .get("system_prompt")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("system_prompt")),
            web_search: provider_config
                .extra
                .get("web_search")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("web_search")),
            provider_config,
        };

        let outcome = self.runner.run(&request).await;
        self.base.status = AgentStatus::Idle;
        self.base.info.last_heartbeat_at = Some(chrono::Utc::now());
        self.base.info.updated_at = chrono::Utc::now();

        match outcome {
            Ok(result) => {
                tracing::info!(
                    agent_id = %self.id(),
                    event_count = result.raw_events.len(),
                    final_message = %result.final_message,
                    "ClaudeCode agent completed heartbeat"
                );

                Ok(HeartbeatResult {
                    success: true,
                    completed_tasks: vec![],
                    created_tasks: vec![],
                    sent_messages: vec![],
                    error: None,
                })
            }
            Err(error) => {
                self.base.status = AgentStatus::Error;
                self.base.info.updated_at = chrono::Utc::now();
                Err(error)
            }
        }
    }

    async fn load_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.load_skill(skill_name).await
    }

    async fn unload_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.unload_skill(skill_name).await
    }

    fn loaded_skills(&self) -> Vec<String> {
        self.base.loaded_skills()
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.base.shutdown().await
    }
}

/// OpenCode Agent implementation
pub struct OpenCodeAgent {
    base: BaseAgent,
    runner: crate::OpenCodeCliRunner,
}

impl OpenCodeAgent {
    /// Create a new OpenCode agent
    pub fn new(config: AgentConfig, capabilities: Arc<dyn AgentCapabilities>) -> Self {
        let base = BaseAgent::new(config, capabilities);
        Self {
            base,
            runner: crate::OpenCodeCliRunner::default(),
        }
    }

    fn provider_config(&self) -> Result<LlmProviderConfig> {
        let config = self.base.config();
        let raw = config
            .adapter_config
            .get("llm_provider")
            .cloned()
            .unwrap_or_else(|| {
                serde_json::Value::Object(config.adapter_config.clone().into_iter().collect())
            });

        serde_json::from_value(raw).map_err(|error| {
            clawlegion_core::Error::Agent(AgentError::ExecutionFailed(format!(
                "invalid OpenCode LLM provider config for agent '{}': {}",
                config.name, error
            )))
        })
    }

    fn runtime_string(&self, key: &str) -> Option<String> {
        self.base
            .config()
            .runtime_config
            .get(key)
            .map(runtime_value_to_string)
            .filter(|value| !value.is_empty())
    }

    fn working_directory(&self) -> Option<PathBuf> {
        self.base
            .config()
            .runtime_config
            .get("working_directory")
            .and_then(|value| value.as_str())
            .map(PathBuf::from)
    }
}

#[async_trait]
impl Agent for OpenCodeAgent {
    fn id(&self) -> clawlegion_core::AgentId {
        self.base.id()
    }

    fn info(&self) -> AgentInfo {
        self.base.info()
    }

    fn set_status(&mut self, status: AgentStatus) {
        self.base.set_status(status);
    }

    async fn heartbeat(&mut self, ctx: HeartbeatContext) -> Result<HeartbeatResult> {
        tracing::info!("OpenCodeAgent {} starting heartbeat", self.id());

        self.base.status = AgentStatus::Running;
        self.base.info.updated_at = chrono::Utc::now();

        let provider_config = self.provider_config()?;
        let _started_at = std::time::Instant::now();
        let request = crate::OpenCodeRunRequest {
            prompt: crate::build_open_code_prompt(
                &self.base.config.name,
                &self.base.config.role,
                &self.base.config.title,
                &self.base.config.capabilities,
                &ctx.trigger,
            ),
            working_directory: self.working_directory(),
            profile: provider_config
                .extra
                .get("profile")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("profile")),
            sandbox: provider_config
                .extra
                .get("sandbox")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("sandbox")),
            approval_policy: provider_config
                .extra
                .get("approval_policy")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("approval_policy")),
            system_prompt: provider_config
                .extra
                .get("system_prompt")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("system_prompt")),
            web_search: provider_config
                .extra
                .get("web_search")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .or_else(|| self.runtime_string("web_search")),
            provider_config,
        };

        let outcome = self.runner.run(&request).await;
        self.base.status = AgentStatus::Idle;
        self.base.info.last_heartbeat_at = Some(chrono::Utc::now());
        self.base.info.updated_at = chrono::Utc::now();

        match outcome {
            Ok(result) => {
                tracing::info!(
                    agent_id = %self.id(),
                    event_count = result.raw_events.len(),
                    final_message = %result.final_message,
                    "OpenCode agent completed heartbeat"
                );

                Ok(HeartbeatResult {
                    success: true,
                    completed_tasks: vec![],
                    created_tasks: vec![],
                    sent_messages: vec![],
                    error: None,
                })
            }
            Err(error) => {
                self.base.status = AgentStatus::Error;
                self.base.info.updated_at = chrono::Utc::now();
                Err(error)
            }
        }
    }

    async fn load_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.load_skill(skill_name).await
    }

    async fn unload_skill(&mut self, skill_name: &str) -> Result<()> {
        self.base.unload_skill(skill_name).await
    }

    fn loaded_skills(&self) -> Vec<String> {
        self.base.loaded_skills()
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.base.shutdown().await
    }
}

/// Agent rule
pub struct AgentRule {
    pub name: String,
    pub condition: RuleCondition,
    pub action: RuleAction,
}

/// Rule condition
pub struct RuleCondition {
    pub trigger_type: TriggerType,
}

impl RuleCondition {
    pub fn matches(&self, trigger: &HeartbeatTrigger) -> bool {
        matches!(
            (&self.trigger_type, trigger),
            (TriggerType::Any, _)
                | (
                    TriggerType::PrivateMessage,
                    HeartbeatTrigger::PrivateMessage { .. }
                )
                | (
                    TriggerType::TaskAssigned,
                    HeartbeatTrigger::TaskAssigned { .. }
                )
                | (TriggerType::Scheduled, HeartbeatTrigger::Scheduled)
        )
    }
}

/// Trigger type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerType {
    Any,
    PrivateMessage,
    GroupMention,
    TaskAssigned,
    Scheduled,
}

/// Rule action
#[derive(Debug, Clone)]
pub enum RuleAction {
    SendMessage {
        content: String,
    },
    CallTool {
        tool: String,
        args: serde_json::Value,
    },
    UpdateStatus {
        status: AgentStatus,
    },
}

fn runtime_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Bool(boolean) => boolean.to_string(),
        serde_json::Value::Number(number) => number.to_string(),
        other => other.to_string(),
    }
}
