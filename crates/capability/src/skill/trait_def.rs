//! Skill trait definition

use async_trait::async_trait;
use clawlegion_core::Result;

use crate::skill::{
    context::SkillContext,
    metadata::{SkillEvent, SkillInput, SkillMetadata, SkillOutput},
    types::{ExecutionMode, SkillType},
};

/// Skill trait - defines high-level capabilities
#[async_trait]
pub trait Skill: Send + Sync {
    /// Get skill metadata
    fn metadata(&self) -> &SkillMetadata;

    /// Get skill type (LLM/Code/Hybrid)
    fn skill_type(&self) -> SkillType {
        self.metadata().skill_type.clone()
    }

    /// Get execution mode
    fn execution_mode(&self) -> ExecutionMode {
        self.metadata().execution_mode
    }

    /// Initialize the skill
    async fn init(&mut self, _ctx: &SkillContext) -> Result<()> {
        Ok(())
    }

    /// Execute the skill with given input
    async fn execute(&self, ctx: &SkillContext, input: SkillInput) -> Result<SkillOutput>;

    /// Get the skill's system prompt (for LLM-based skills)
    fn system_prompt(&self) -> Option<&str> {
        None
    }

    /// Handle events (for event-driven skills)
    async fn on_event(&self, _ctx: &SkillContext, _event: SkillEvent) -> Result<()> {
        Ok(())
    }

    /// Shutdown the skill
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Type-erased skill wrapper
pub type SkillBox = Box<dyn Skill>;

/// Skill instance for an agent
pub struct SkillInstance {
    pub skill: SkillBox,
    pub context: SkillContext,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
}

impl SkillInstance {
    pub fn new(skill: SkillBox, agent_id: clawlegion_core::AgentId) -> Self {
        Self {
            skill,
            context: SkillContext::new(agent_id),
            loaded_at: chrono::Utc::now(),
        }
    }

    pub fn with_config(
        mut self,
        config: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        self.context.config = config;
        self
    }

    /// Get skill metadata
    pub fn metadata(&self) -> &SkillMetadata {
        self.skill.metadata()
    }

    /// Execute the skill
    pub async fn execute(&self, input: SkillInput) -> Result<SkillOutput> {
        self.skill.execute(&self.context, input).await
    }

    /// Shutdown the skill
    pub async fn shutdown(&self) -> Result<()> {
        self.skill.shutdown().await
    }
}
