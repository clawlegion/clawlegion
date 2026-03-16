//! Sentinel Manager - coordinates watching and agent wakeup

use crate::{BuiltinTriggerRegistry, SentinelWatcher, WakeupAction, WakeupMethod, WakeupTrigger};
use clawlegion_core::{AgentId, Error, Result, SentinelError};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

/// Sentinel Manager
///
/// Main coordinator for the sentinel system.
/// - Manages the watcher
/// - Handles agent wakeup logic
/// - Coordinates with external systems
/// - Manages builtin triggers
pub struct SentinelManager {
    /// Sentinel watcher
    watcher: Arc<SentinelWatcher>,

    /// Agent wakeup handler (using async RwLock for tokio compatibility)
    wakeup_handler: Arc<AsyncRwLock<Box<dyn AgentWakeupHandler>>>,

    /// Builtin trigger registry
    builtin_triggers: Arc<BuiltinTriggerRegistry>,

    /// Running flag
    running: Arc<RwLock<bool>>,

    /// Shutdown signal sender
    shutdown_tx: RwLock<Option<tokio::sync::broadcast::Sender<()>>>,
}

impl SentinelManager {
    /// Create a new sentinel manager
    pub fn new(watcher: Arc<SentinelWatcher>) -> Self {
        Self {
            watcher,
            wakeup_handler: Arc::new(AsyncRwLock::new(Box::new(DefaultWakeupHandler))),
            builtin_triggers: Arc::new(BuiltinTriggerRegistry::with_defaults()),
            running: Arc::new(RwLock::new(false)),
            shutdown_tx: RwLock::new(None),
        }
    }

    /// Create a new sentinel manager with custom builtin triggers
    pub fn with_builtin_triggers(
        watcher: Arc<SentinelWatcher>,
        builtin_triggers: BuiltinTriggerRegistry,
    ) -> Self {
        Self {
            watcher,
            wakeup_handler: Arc::new(AsyncRwLock::new(Box::new(DefaultWakeupHandler))),
            builtin_triggers: Arc::new(builtin_triggers),
            running: Arc::new(RwLock::new(false)),
            shutdown_tx: RwLock::new(None),
        }
    }

    /// Set a custom wakeup handler
    pub async fn set_wakeup_handler(&self, handler: Box<dyn AgentWakeupHandler>) {
        *self.wakeup_handler.write().await = handler;
    }

    /// Get the builtin trigger registry
    pub fn builtin_triggers(&self) -> &BuiltinTriggerRegistry {
        &self.builtin_triggers
    }

    /// Register a custom builtin trigger
    ///
    /// Note: This method is currently a no-op as the builtin trigger registry
    /// is immutable after creation. For custom triggers, create a custom
    /// BuiltinTriggerRegistry and use `with_builtin_triggers` constructor.
    pub fn register_builtin_trigger(&self, _trigger: Box<dyn crate::BuiltinWakeupTrigger>) {
        // Note: The builtin_triggers field is Arc<BuiltinTriggerRegistry> which is immutable.
        // To register custom triggers, use the `with_builtin_triggers` constructor
        // with a custom BuiltinTriggerRegistry instance.
        tracing::debug!(
            "register_builtin_trigger called. Use with_builtin_triggers constructor for custom triggers."
        );
    }

    /// Start the sentinel monitoring loop
    pub async fn start(&self, check_interval_secs: u64) -> Result<()> {
        if *self.running.read() {
            return Err(Error::Sentinel(SentinelError::Watchdog(
                "Sentinel is already running".to_string(),
            )));
        }

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);
        *self.shutdown_tx.write() = Some(shutdown_tx.clone());
        *self.running.write() = true;

        let watcher = self.watcher.clone();
        let wakeup_handler = self.wakeup_handler.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(check_interval_secs));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Evaluate all triggers
                        let evaluations = watcher.evaluate_all().await;

                        // Collect work items before processing (to avoid holding locks)
                        let mut work_items = Vec::new();
                        for evaluation in evaluations {
                            if let Some(trigger) = watcher.get_trigger(&evaluation.trigger_id) {
                                work_items.push((trigger.id.clone(), trigger.agent_id, trigger.wakeup_method.clone(), evaluation.data.clone()));
                            }
                        }

                        // Process work items
                        for (trigger_id, agent_id, method, data) in work_items {
                            let handler_result = call_handler(wakeup_handler.clone(), agent_id, method, data).await;

                            if let Err(e) = handler_result {
                                tracing::error!("Failed to wake up agent: {}", e);
                            } else {
                                watcher.mark_triggered(&trigger_id);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        *running.write() = false;
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the sentinel monitoring loop
    pub async fn stop(&self) -> Result<()> {
        if !*self.running.read() {
            return Ok(());
        }

        if let Some(tx) = self.shutdown_tx.read().as_ref() {
            let _ = tx.send(());
        }

        // Wait for the loop to stop
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        *self.running.write() = false;
        *self.shutdown_tx.write() = None;

        Ok(())
    }

    /// Check if sentinel is running
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    /// Get the watcher
    pub fn watcher(&self) -> &SentinelWatcher {
        &self.watcher
    }

    /// Register a trigger
    pub fn register_trigger(&self, trigger: WakeupTrigger) {
        self.watcher.register_trigger(trigger);
    }

    /// Unregister a trigger
    pub fn unregister_trigger(&self, trigger_id: &str) -> Option<WakeupTrigger> {
        self.watcher.unregister_trigger(trigger_id)
    }

    /// Manually trigger an agent wakeup
    pub async fn manual_wakeup(&self, agent_id: AgentId, method: WakeupMethod) -> Result<()> {
        self.wakeup_handler
            .read()
            .await
            .handle_wakeup(agent_id, &method, None)
            .await
    }

    /// Get triggers for an agent
    pub fn get_triggers_for_agent(&self, agent_id: AgentId) -> Vec<WakeupTrigger> {
        self.watcher.get_triggers_for_agent(agent_id)
    }

    /// Get all triggers
    pub fn list_triggers(&self) -> Vec<WakeupTrigger> {
        self.watcher.list_triggers()
    }

    /// Directly trigger an agent wakeup via DirectCall trigger type
    ///
    /// This method bypasses the normal polling/evaluation cycle and immediately
    /// wakes up the specified agent. It's designed for programmatic triggers
    /// where code directly calls to wake an agent.
    ///
    /// # Supported builtin trigger types
    /// - "private_message" - Trigger private message wakeup
    /// - "task_assigned" - Trigger task assignment wakeup
    /// - "manager_assigned" - Trigger manager assignment wakeup
    ///
    /// # Arguments
    /// * `agent_id` - The agent to wake up
    /// * `caller` - Identifier for the caller (e.g., tool name, function name, or builtin trigger type)
    /// * `params` - Parameters to pass to the agent
    ///
    /// # Example
    /// ```no_run
    /// use clawlegion_core::AgentId;
    /// use clawlegion_sentinel::SentinelManager;
    /// use serde_json::json;
    ///
    /// async fn example(manager: &SentinelManager, agent_id: AgentId) {
    ///     // Use builtin trigger
    ///     manager.direct_wakeup(
    ///         agent_id,
    ///         "private_message",
    ///         json!({ "message_id": "...", "sender": "..." })
    ///     ).await.unwrap();
    ///
    ///     // Or use custom DirectCall trigger
    ///     manager.direct_wakeup(
    ///         agent_id,
    ///         "my_tool",
    ///         json!({ "action": "process_data", "data": "..." })
    ///     ).await.unwrap();
    /// }
    /// ```
    pub async fn direct_wakeup(
        &self,
        agent_id: AgentId,
        caller: impl Into<String>,
        params: serde_json::Value,
    ) -> Result<()> {
        let caller_str = caller.into();

        // Check if caller is a builtin trigger type
        if let Some(builtin_trigger) = self.builtin_triggers.get_by_type(&caller_str) {
            // Create trigger context from params
            let context = crate::TriggerContext::new();

            // Check if the builtin trigger should fire
            if builtin_trigger
                .should_trigger(&context)
                .await
                .unwrap_or(false)
            {
                // Execute the builtin trigger wakeup
                return builtin_trigger.wakeup(agent_id, params).await;
            }
            // If builtin trigger condition not met, fall through to DirectCall triggers
        }

        // Find DirectCall triggers for this agent
        let triggers = self.watcher.get_triggers_for_agent(agent_id);
        let direct_call_triggers: Vec<_> = triggers
            .into_iter()
            .filter(|t| matches!(t.condition, crate::TriggerCondition::DirectCall { .. }))
            .collect();

        if direct_call_triggers.is_empty() {
            // No DirectCall trigger registered, use manual wakeup with Heartbeat as default
            tracing::warn!(
                "No DirectCall trigger registered for agent {}, using default heartbeat",
                agent_id
            );
            return self.manual_wakeup(agent_id, WakeupMethod::Heartbeat).await;
        }

        // Execute wakeup for each matching trigger
        for trigger in direct_call_triggers {
            // Mark trigger as triggered (for cooldown)
            self.watcher.mark_triggered(&trigger.id);

            // Execute the wakeup method
            let data = Some(serde_json::json!({
                "caller": caller_str,
                "params": params
            }));

            let handler_result = call_handler(
                self.wakeup_handler.clone(),
                agent_id,
                trigger.wakeup_method.clone(),
                data.clone(),
            )
            .await;

            if let Err(e) = handler_result {
                tracing::error!(
                    "Failed to execute DirectCall wakeup for agent {}: {}",
                    agent_id,
                    e
                );
                return Err(e);
            }
        }

        Ok(())
    }

    /// Directly trigger an agent wakeup with a specific skill
    ///
    /// This is a convenience method that combines direct_wakeup with ExecuteSkill method.
    ///
    /// # Arguments
    /// * `agent_id` - The agent to wake up
    /// * `skill_name` - The skill to execute
    /// * `input` - Skill input parameters
    /// * `caller` - Identifier for the caller
    pub async fn direct_wakeup_with_skill(
        &self,
        agent_id: AgentId,
        skill_name: impl Into<String>,
        input: serde_json::Value,
        _caller: impl Into<String>,
    ) -> Result<()> {
        let method = WakeupMethod::ExecuteSkill {
            skill: skill_name.into(),
            input,
        };
        self.manual_wakeup(agent_id, method).await?;

        // Mark any DirectCall triggers as triggered
        let triggers = self.watcher.get_triggers_for_agent(agent_id);
        for trigger in triggers {
            if matches!(
                trigger.condition,
                crate::TriggerCondition::DirectCall { .. }
            ) {
                self.watcher.mark_triggered(&trigger.id);
            }
        }

        Ok(())
    }
}

/// Trait for handling agent wakeup
#[async_trait::async_trait]
pub trait AgentWakeupHandler: Send + Sync + 'static {
    /// Handle agent wakeup
    async fn handle_wakeup(
        &self,
        agent_id: AgentId,
        method: &WakeupMethod,
        data: Option<serde_json::Value>,
    ) -> Result<()>;
}

/// Default wakeup handler
pub struct DefaultWakeupHandler;

#[async_trait::async_trait]
impl AgentWakeupHandler for DefaultWakeupHandler {
    async fn handle_wakeup(
        &self,
        agent_id: AgentId,
        method: &WakeupMethod,
        _data: Option<serde_json::Value>,
    ) -> Result<()> {
        match method {
            WakeupMethod::SendMessage { content } => {
                tracing::info!("Waking up agent {} with message: {}", agent_id, content);
                // In a real implementation, this would send a message to the agent
            }

            WakeupMethod::Heartbeat => {
                tracing::info!("Triggering heartbeat for agent {}", agent_id);
                // In a real implementation, this would trigger an agent heartbeat
            }

            WakeupMethod::ExecuteSkill { skill, input: _ } => {
                tracing::info!("Executing skill '{}' for agent {}", skill, agent_id);
                // In a real implementation, this would execute the skill
            }

            WakeupMethod::CallWebhook { url, method, body } => {
                tracing::info!("Calling webhook {} {} with body: {:?}", url, method, body);
                // In a real implementation, this would call the webhook
            }

            WakeupMethod::Composite { actions } => {
                for action in actions {
                    if let Err(e) = self.execute_action(agent_id, action).await {
                        tracing::error!("Failed to execute composite action: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

impl DefaultWakeupHandler {
    async fn execute_action(&self, agent_id: AgentId, action: &WakeupAction) -> Result<()> {
        match action {
            WakeupAction::SendMessage { content } => {
                tracing::info!("Action: Send message to agent {}: {}", agent_id, content);
            }

            WakeupAction::Heartbeat => {
                tracing::info!("Action: Trigger heartbeat for agent {}", agent_id);
            }

            WakeupAction::ExecuteSkill { skill, input: _ } => {
                tracing::info!("Action: Execute skill '{}' for agent {}", skill, agent_id);
            }
        }

        Ok(())
    }
}

/// Sentinel configuration
#[derive(Debug, Clone)]
pub struct SentinelConfig {
    /// Check interval in seconds
    pub check_interval_secs: u64,

    /// Default cooldown for triggers
    pub default_cooldown_secs: u64,

    /// Maximum concurrent wakeups
    pub max_concurrent_wakeups: usize,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 10,
            default_cooldown_secs: 60,
            max_concurrent_wakeups: 10,
        }
    }
}

/// Helper function to call handler without holding locks across await
async fn call_handler(
    handler: Arc<AsyncRwLock<Box<dyn AgentWakeupHandler>>>,
    agent_id: AgentId,
    method: WakeupMethod,
    data: Option<serde_json::Value>,
) -> Result<()> {
    let guard = handler.read().await;
    let result = guard.handle_wakeup(agent_id, &method, data).await;
    drop(guard);
    result
}
