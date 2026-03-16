use std::sync::Arc;

use clawlegion_core::{CapabilityError, Error, Result};
use dashmap::DashMap;

use crate::tool::{Tool, ToolBox, ToolMetadata, Visibility};

pub struct ToolRegistry {
    tools: DashMap<String, ToolBox>,
    metadata_cache: DashMap<String, ToolMetadata>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
            metadata_cache: DashMap::new(),
        }
    }

    pub fn register(&self, tool: Arc<dyn Tool>) -> Result<()> {
        let metadata = tool.metadata().clone();
        let name = metadata.name.clone();
        if self.tools.contains_key(&name) {
            return Err(Error::Capability(CapabilityError::NotFound(format!(
                "Tool '{}' already registered",
                name
            ))));
        }
        self.metadata_cache.insert(name.clone(), metadata);
        self.tools.insert(name, tool);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<ToolBox> {
        self.tools.get(name).map(|entry| entry.clone())
    }

    pub fn get_metadata(&self, name: &str) -> Option<ToolMetadata> {
        self.metadata_cache.get(name).map(|entry| entry.clone())
    }

    pub fn unregister(&self, name: &str) -> bool {
        let removed_tool = self.tools.remove(name).is_some();
        let removed_metadata = self.metadata_cache.remove(name).is_some();
        removed_tool || removed_metadata
    }

    pub fn list(&self) -> Vec<ToolMetadata> {
        self.metadata_cache
            .iter()
            .map(|entry| entry.clone())
            .collect()
    }

    pub fn list_public(&self) -> Vec<ToolMetadata> {
        self.metadata_cache
            .iter()
            .filter(|entry| entry.visibility == Visibility::Public)
            .map(|entry| entry.clone())
            .collect()
    }

    pub fn list_by_tag(&self, tag: &str) -> Vec<ToolMetadata> {
        self.metadata_cache
            .iter()
            .filter(|entry| entry.tags.iter().any(|candidate| candidate == tag))
            .map(|entry| entry.clone())
            .collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
