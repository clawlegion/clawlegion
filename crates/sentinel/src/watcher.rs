//! Watcher implementations for different trigger types

use crate::{
    ConditionEvaluator, CustomConditionHandler, TriggerCondition, TriggerEvaluation, TriggerId,
    WakeupTrigger,
};
use clawlegion_core::{Error, Result, SentinelError};
use parking_lot::RwLock as ParkingRwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Sentinel Watcher
///
/// Monitors all registered triggers and evaluates their conditions.
#[derive(Clone)]
pub struct SentinelWatcher {
    /// Registered triggers (using parking_lot for sync access)
    triggers: Arc<ParkingRwLock<HashMap<TriggerId, WakeupTrigger>>>,

    /// Condition evaluators (using parking_lot for sync access)
    _evaluators: Arc<ParkingRwLock<HashMap<String, Arc<dyn ConditionEvaluator>>>>,

    /// Custom condition handlers (using async RwLock for tokio compatibility)
    custom_handlers: Arc<RwLock<HashMap<String, Box<dyn CustomConditionHandler>>>>,
}

impl SentinelWatcher {
    /// Create a new sentinel watcher
    pub fn new() -> Self {
        Self {
            triggers: Arc::new(ParkingRwLock::new(HashMap::new())),
            _evaluators: Arc::new(ParkingRwLock::new(HashMap::new())),
            custom_handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a trigger
    pub fn register_trigger(&self, trigger: WakeupTrigger) {
        let id = trigger.id.clone();
        self.triggers.write().insert(id, trigger);
    }

    /// Unregister a trigger
    pub fn unregister_trigger(&self, trigger_id: &str) -> Option<WakeupTrigger> {
        self.triggers.write().remove(trigger_id)
    }

    /// Get a trigger by ID
    pub fn get_trigger(&self, trigger_id: &str) -> Option<WakeupTrigger> {
        self.triggers.read().get(trigger_id).cloned()
    }

    /// Get all triggers
    pub fn list_triggers(&self) -> Vec<WakeupTrigger> {
        self.triggers.read().values().cloned().collect()
    }

    /// Get triggers for an agent
    pub fn get_triggers_for_agent(&self, agent_id: clawlegion_core::AgentId) -> Vec<WakeupTrigger> {
        self.triggers
            .read()
            .values()
            .filter(|t| t.agent_id == agent_id && t.enabled)
            .cloned()
            .collect()
    }

    /// Enable a trigger
    pub fn enable_trigger(&self, trigger_id: &str) -> Result<()> {
        let mut triggers = self.triggers.write();
        let trigger = triggers.get_mut(trigger_id).ok_or_else(|| {
            Error::Sentinel(SentinelError::TriggerRegistration(format!(
                "Trigger '{}' not found",
                trigger_id
            )))
        })?;

        trigger.enabled = true;
        Ok(())
    }

    /// Disable a trigger
    pub fn disable_trigger(&self, trigger_id: &str) -> Result<()> {
        let mut triggers = self.triggers.write();
        let trigger = triggers.get_mut(trigger_id).ok_or_else(|| {
            Error::Sentinel(SentinelError::TriggerRegistration(format!(
                "Trigger '{}' not found",
                trigger_id
            )))
        })?;

        trigger.enabled = false;
        Ok(())
    }

    /// Register a custom condition handler
    pub async fn register_custom_handler(
        &self,
        name: impl Into<String>,
        handler: Box<dyn CustomConditionHandler>,
    ) {
        self.custom_handlers
            .write()
            .await
            .insert(name.into(), handler);
    }

    /// Evaluate all triggers
    pub async fn evaluate_all(&self) -> Vec<TriggerEvaluation> {
        let triggers: Vec<WakeupTrigger> = self
            .triggers
            .read()
            .values()
            .filter(|t| t.enabled && !t.is_in_cooldown())
            .cloned()
            .collect();

        let mut evaluations = Vec::new();

        for trigger in triggers {
            let evaluation = self
                .evaluate_condition(&trigger.condition, &trigger.id)
                .await;
            if evaluation.condition_met {
                evaluations.push(evaluation);
            }
        }

        evaluations
    }

    /// Evaluate triggers for a specific agent
    pub async fn evaluate_for_agent(
        &self,
        agent_id: clawlegion_core::AgentId,
    ) -> Vec<TriggerEvaluation> {
        let triggers = self.get_triggers_for_agent(agent_id);
        let mut evaluations = Vec::new();

        for trigger in triggers {
            if trigger.is_in_cooldown() {
                continue;
            }

            let evaluation = self
                .evaluate_condition(&trigger.condition, &trigger.id)
                .await;
            if evaluation.condition_met {
                evaluations.push(evaluation);
            }
        }

        evaluations
    }

    /// Evaluate a single condition (inner implementation with owned parameters)
    fn evaluate_condition_owned(
        self: Arc<Self>,
        condition: TriggerCondition,
        trigger_id: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = TriggerEvaluation> + Send>> {
        use futures_util::future::FutureExt;

        match condition {
            TriggerCondition::PrivateMessage { from } => {
                let eval = TriggerEvaluation::not_met(trigger_id)
                    .with_details(format!("PrivateMessage filter: from={:?}", from));
                async move { eval }.boxed()
            }

            TriggerCondition::TaskAssigned { task_id } => {
                let eval = TriggerEvaluation::not_met(trigger_id)
                    .with_details(format!("TaskAssigned task={:?}", task_id));
                async move { eval }.boxed()
            }

            TriggerCondition::ManagerAssigned { manager_id } => {
                let eval = TriggerEvaluation::not_met(trigger_id)
                    .with_details(format!("ManagerAssigned manager={:?}", manager_id));
                async move { eval }.boxed()
            }

            TriggerCondition::Cron { expression } => {
                let eval = TriggerEvaluation::not_met(trigger_id)
                    .with_details(format!("Cron expression={}", expression));
                async move { eval }.boxed()
            }

            TriggerCondition::Polling {
                interval_secs,
                checker,
                params: _,
            } => {
                let eval = TriggerEvaluation::not_met(trigger_id).with_details(format!(
                    "Polling checker={}, interval={}s",
                    checker, interval_secs
                ));
                async move { eval }.boxed()
            }

            TriggerCondition::Stream { url, filter } => {
                let eval = TriggerEvaluation::not_met(trigger_id)
                    .with_details(format!("Stream url={}, filter={:?}", url, filter));
                async move { eval }.boxed()
            }

            TriggerCondition::Webhook {
                secret: _,
                schema: _,
            } => {
                let eval = TriggerEvaluation::not_met(trigger_id)
                    .with_details("Webhook waiting for external call".to_string());
                async move { eval }.boxed()
            }

            TriggerCondition::Custom { name, params } => async move {
                let handler_result = {
                    let handlers = self.custom_handlers.read().await;
                    if let Some(handler) = handlers.get(&name) {
                        Some(handler.evaluate(&params).await)
                    } else {
                        None
                    }
                };

                if let Some(result) = handler_result {
                    match result {
                        Ok(result) => {
                            if result {
                                TriggerEvaluation::met(trigger_id)
                            } else {
                                TriggerEvaluation::not_met(trigger_id)
                            }
                        }
                        Err(e) => TriggerEvaluation::not_met(trigger_id)
                            .with_details(format!("Custom handler error: {}", e)),
                    }
                } else {
                    TriggerEvaluation::not_met(trigger_id)
                        .with_details(format!("Custom handler '{}' not found", name))
                }
            }
            .boxed(),

            TriggerCondition::Compound { op, conditions } => async move {
                let mut results = Vec::new();

                for condition in conditions {
                    let eval =
                        Self::evaluate_condition_owned(self.clone(), condition, trigger_id.clone())
                            .await;
                    results.push(eval.condition_met);
                }

                let met = match op {
                    crate::CompoundOp::And => results.iter().all(|&r| r),
                    crate::CompoundOp::Or => results.iter().any(|&r| r),
                    crate::CompoundOp::Xor => results.iter().filter(|&&r| r).count() == 1,
                };

                if met {
                    TriggerEvaluation::met(trigger_id)
                } else {
                    TriggerEvaluation::not_met(trigger_id)
                }
            }
            .boxed(),

            TriggerCondition::DirectCall { caller, params: _ } => {
                // DirectCall is a passive trigger type - it's only triggered via direct API call
                // Not through the polling/evaluation cycle
                let eval = TriggerEvaluation::not_met(trigger_id)
                    .with_details(format!("DirectCall (passive trigger) caller={}", caller));
                async move { eval }.boxed()
            }
        }
    }

    /// Evaluate a single condition (public API)
    async fn evaluate_condition(
        &self,
        condition: &TriggerCondition,
        trigger_id: &str,
    ) -> TriggerEvaluation {
        Self::evaluate_condition_owned(
            Arc::new(self.clone()),
            condition.clone(),
            trigger_id.to_string(),
        )
        .await
    }

    /// Mark a trigger as triggered (for cooldown)
    pub fn mark_triggered(&self, trigger_id: &str) {
        if let Some(trigger) = self.triggers.write().get_mut(trigger_id) {
            trigger.mark_triggered();
        }
    }
}

impl Default for SentinelWatcher {
    fn default() -> Self {
        Self::new()
    }
}
