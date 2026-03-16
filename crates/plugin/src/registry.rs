use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::RwLock;

use crate::capability::CapabilityRegistry;
use clawlegion_core::{
    Plugin, PluginContext, PluginInfo, PluginManifest, PluginState, PluginStatus,
};

pub struct RegistryEntry {
    pub info: PluginInfo,
    pub plugin: Option<Arc<RwLock<Box<dyn Plugin>>>>,
    pub context: Option<PluginContext>,
}

pub struct PluginRegistry {
    plugins: DashMap<String, RegistryEntry>,
    search_paths: RwLock<Vec<PathBuf>>,
    data_dir: PathBuf,
    config_dir: PathBuf,
    capability_registry: Arc<CapabilityRegistry>,
}

impl PluginRegistry {
    pub fn new(data_dir: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            plugins: DashMap::new(),
            search_paths: RwLock::new(Vec::new()),
            data_dir,
            config_dir,
            capability_registry: Arc::new(CapabilityRegistry::new()),
        }
    }

    pub fn add_search_path(&self, path: PathBuf) {
        self.search_paths.write().push(path);
    }

    pub fn search_paths(&self) -> Vec<PathBuf> {
        self.search_paths.read().clone()
    }

    pub fn capability_registry(&self) -> Arc<CapabilityRegistry> {
        Arc::clone(&self.capability_registry)
    }

    pub fn register_manifest(
        &self,
        manifest: PluginManifest,
        manifest_path: Option<PathBuf>,
        load_path: Option<PathBuf>,
    ) -> Result<()> {
        if self.plugins.contains_key(&manifest.id) {
            return Err(anyhow!("plugin {} already registered", manifest.id));
        }

        let info = PluginInfo::new(
            manifest.id.clone(),
            manifest,
            PluginState::Discovered,
            load_path,
            manifest_path,
        );

        self.plugins.insert(
            info.id.clone(),
            RegistryEntry {
                info,
                plugin: None,
                context: None,
            },
        );

        Ok(())
    }

    pub fn attach_plugin(
        &self,
        plugin_id: &str,
        plugin: Box<dyn Plugin>,
        context: PluginContext,
    ) -> Result<()> {
        let mut entry = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| anyhow!("plugin {} not found", plugin_id))?;
        entry.context = Some(context);
        entry.plugin = Some(Arc::new(RwLock::new(plugin)));
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<PluginInfo> {
        self.plugins.get(id).map(|entry| entry.info.clone())
    }

    pub fn get_context(&self, id: &str) -> Option<PluginContext> {
        self.plugins.get(id).and_then(|entry| entry.context.clone())
    }

    pub fn plugin_handle(&self, id: &str) -> Option<Arc<RwLock<Box<dyn Plugin>>>> {
        self.plugins.get(id).and_then(|entry| entry.plugin.clone())
    }

    pub fn has_instance(&self, id: &str) -> bool {
        self.plugins
            .get(id)
            .map(|entry| entry.plugin.is_some())
            .unwrap_or(false)
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let mut plugins: Vec<_> = self
            .plugins
            .iter()
            .map(|entry| entry.info.clone())
            .collect();
        plugins.sort_by(|left, right| left.id.cmp(&right.id));
        plugins
    }

    pub fn set_plugin_state(
        &self,
        id: &str,
        state: PluginState,
        detail: Option<String>,
    ) -> Result<()> {
        let mut entry = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| anyhow!("plugin {} not found", id))?;
        entry.info.state = state.clone();
        entry.info.status = PluginStatus {
            state,
            detail,
            last_transition_unix: Some(chrono::Utc::now().timestamp()),
        };
        Ok(())
    }

    pub fn set_plugin_health(&self, id: &str, health: Option<String>) -> Result<()> {
        let mut entry = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| anyhow!("plugin {} not found", id))?;
        entry.info.health = health;
        Ok(())
    }

    pub fn push_error(&self, id: &str, error: String) -> Result<()> {
        let mut entry = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| anyhow!("plugin {} not found", id))?;
        entry.info.errors.push(error);
        Ok(())
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let mut entry = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| anyhow!("plugin {} not found", id))?;
        entry.info.enabled = enabled;
        Ok(())
    }

    pub fn unregister(&self, id: &str) -> Result<()> {
        self.capability_registry.unregister_plugin(id);
        self.plugins
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| anyhow!("plugin {} not found", id))
    }

    pub fn is_registered(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }

    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    pub fn config_dir(&self) -> &std::path::Path {
        &self.config_dir
    }
}
