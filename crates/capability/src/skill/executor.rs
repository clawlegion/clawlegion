//! Skill Executor - handles skill execution with timeout and concurrency control

use clawlegion_core::{AgentId, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, warn};

use crate::skill::{
    context::SkillContext,
    metadata::{SkillInput, SkillOutput},
    trait_def::Skill,
};

/// Execution result with timing information
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub output: SkillOutput,
    pub execution_time_ms: u64,
    pub skill_name: String,
    pub agent_id: AgentId,
}

/// Skill Executor configuration
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Default timeout for skill execution (ms)
    pub default_timeout_ms: u64,
    /// Maximum concurrent executions per agent
    pub max_concurrent_per_agent: usize,
    /// Enable execution logging
    pub enable_logging: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 60000, // 60 seconds
            max_concurrent_per_agent: 5,
            enable_logging: true,
        }
    }
}

/// Skill Executor
pub struct SkillExecutor {
    config: ExecutorConfig,
    /// Track concurrent executions per agent
    concurrent_count: Arc<dashmap::DashMap<AgentId, usize>>,
    /// Execution statistics
    stats: Arc<dashmap::DashMap<String, ExecutionStats>>,
}

impl SkillExecutor {
    /// Create a new skill executor
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            config,
            concurrent_count: Arc::new(dashmap::DashMap::new()),
            stats: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(ExecutorConfig::default())
    }

    /// Execute a skill with timeout
    pub async fn execute(
        &self,
        skill: &dyn Skill,
        ctx: &SkillContext,
        input: SkillInput,
    ) -> Result<ExecutionResult> {
        let skill_name = skill.metadata().name.clone();
        let agent_id = ctx.agent_id;
        let timeout_ms = self.config.default_timeout_ms;

        // Update concurrent count
        let mut count = self.concurrent_count.entry(agent_id).or_insert(0);
        *count += 1;
        let concurrent = *count;
        drop(count);

        // Check concurrent limit
        if concurrent > self.config.max_concurrent_per_agent {
            self.concurrent_count
                .entry(agent_id)
                .and_modify(|c| *c -= 1);
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Too many concurrent executions for agent {:?} (max: {})",
                    agent_id, self.config.max_concurrent_per_agent
                )),
            ));
        }

        let start = std::time::Instant::now();

        // Execute with timeout
        let result =
            match timeout(Duration::from_millis(timeout_ms), skill.execute(ctx, input)).await {
                Ok(Ok(output)) => Ok(output),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(clawlegion_core::Error::Capability(
                    clawlegion_core::CapabilityError::NotFound(format!(
                        "Skill '{}' execution timed out after {}ms",
                        skill_name, timeout_ms
                    )),
                )),
            };

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Update concurrent count
        self.concurrent_count
            .entry(agent_id)
            .and_modify(|c| *c -= 1);

        // Update statistics
        self.update_stats(&skill_name, execution_time_ms, result.is_ok());

        // Log if enabled
        if self.config.enable_logging {
            match &result {
                Ok(output) => {
                    if output.success {
                        info!(
                            "Skill '{}' executed successfully in {}ms",
                            skill_name, execution_time_ms
                        );
                    } else {
                        warn!(
                            "Skill '{}' completed with error: {:?}",
                            skill_name, output.error
                        );
                    }
                }
                Err(e) => {
                    error!("Skill '{}' execution failed: {}", skill_name, e);
                }
            }
        }

        Ok(ExecutionResult {
            output: result.unwrap_or_else(|e| SkillOutput::error(e.to_string())),
            execution_time_ms,
            skill_name,
            agent_id,
        })
    }

    /// Execute a skill in streaming mode (for LLM-based skills)
    pub async fn execute_stream(
        &self,
        skill: &dyn Skill,
        ctx: &SkillContext,
        input: SkillInput,
    ) -> Result<ExecutionResult> {
        // For now, just delegate to regular execute
        // In a full implementation, this would return a stream of chunks
        self.execute(skill, ctx, input).await
    }

    /// Execute multiple skills in parallel
    pub async fn execute_batch(
        &self,
        skills: Vec<(&dyn Skill, SkillContext, SkillInput)>,
    ) -> Vec<Result<ExecutionResult>> {
        let futures: Vec<_> = skills
            .into_iter()
            .map(|(skill, ctx, input)| async move { self.execute(skill, &ctx, input).await })
            .collect();

        futures_util::future::join_all(futures).await
    }

    /// Get execution statistics for a skill
    pub fn get_stats(&self, skill_name: &str) -> Option<ExecutionStats> {
        self.stats.get(skill_name).map(|s| s.clone())
    }

    /// Get all execution statistics
    pub fn get_all_stats(&self) -> HashMap<String, ExecutionStats> {
        self.stats
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    fn update_stats(&self, skill_name: &str, execution_time_ms: u64, success: bool) {
        let mut stats = self.stats.entry(skill_name.to_string()).or_default();
        stats.update(execution_time_ms, success);
    }

    /// Get the current concurrent execution count for an agent
    pub fn get_concurrent_count(&self, agent_id: &AgentId) -> usize {
        self.concurrent_count.get(agent_id).map(|c| *c).unwrap_or(0)
    }

    /// Get the executor configuration
    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }
}

/// Execution statistics for a skill
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub total_time_ms: u64,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub avg_time_ms: f64,
}

impl ExecutionStats {
    pub fn update(&mut self, execution_time_ms: u64, success: bool) {
        self.total_executions += 1;
        self.total_time_ms += execution_time_ms;

        if success {
            self.successful_executions += 1;
        } else {
            self.failed_executions += 1;
        }

        if self.min_time_ms == 0 || execution_time_ms < self.min_time_ms {
            self.min_time_ms = execution_time_ms;
        }
        if execution_time_ms > self.max_time_ms {
            self.max_time_ms = execution_time_ms;
        }

        self.avg_time_ms = self.total_time_ms as f64 / self.total_executions as f64;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            self.successful_executions as f64 / self.total_executions as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{types::Visibility, SkillContext, SkillMetadata};

    struct TestSkill {
        delay_ms: u64,
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl Skill for TestSkill {
        fn metadata(&self) -> &SkillMetadata {
            static METADATA: std::sync::OnceLock<SkillMetadata> = std::sync::OnceLock::new();
            METADATA.get_or_init(|| {
                SkillMetadata::new("test-skill", "1.0.0", "A test skill")
                    .with_visibility(Visibility::Public)
            })
        }

        async fn execute(&self, _ctx: &SkillContext, _input: SkillInput) -> Result<SkillOutput> {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            if self.should_fail {
                Ok(SkillOutput::error("Intentional failure"))
            } else {
                Ok(SkillOutput::success("Success"))
            }
        }
    }

    #[tokio::test]
    async fn test_executor_basic_execution() {
        let executor = SkillExecutor::with_defaults();
        let skill = TestSkill {
            delay_ms: 10,
            should_fail: false,
        };
        let ctx =
            SkillContext::new(AgentId::parse_str("00000000-0000-0000-0000-000000000001").unwrap());
        let input = SkillInput::text("test");

        let result = executor.execute(&skill, &ctx, input).await.unwrap();
        assert!(result.output.success);
        assert!(result.execution_time_ms >= 10);
    }

    #[tokio::test]
    async fn test_executor_timeout() {
        let executor = SkillExecutor::new(ExecutorConfig {
            default_timeout_ms: 50,
            ..Default::default()
        });
        let skill = TestSkill {
            delay_ms: 100,
            should_fail: false,
        };
        let ctx =
            SkillContext::new(AgentId::parse_str("00000000-0000-0000-0000-000000000001").unwrap());
        let input = SkillInput::text("test");

        let result = executor.execute(&skill, &ctx, input).await.unwrap();
        assert!(!result.output.success);
        assert!(result.output.error.unwrap().contains("timed out"));
    }

    #[tokio::test]
    async fn test_executor_stats() {
        let executor = SkillExecutor::with_defaults();
        let skill = TestSkill {
            delay_ms: 10,
            should_fail: false,
        };
        let ctx =
            SkillContext::new(AgentId::parse_str("00000000-0000-0000-0000-000000000001").unwrap());

        for _ in 0..5 {
            executor
                .execute(&skill, &ctx, SkillInput::text("test"))
                .await
                .unwrap();
        }

        let stats = executor.get_stats("test-skill").unwrap();
        assert_eq!(stats.total_executions, 5);
        assert_eq!(stats.successful_executions, 5);
        assert_eq!(stats.failed_executions, 0);
        assert_eq!(stats.success_rate(), 1.0);
    }
}
