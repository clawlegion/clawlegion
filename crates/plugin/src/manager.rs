use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use parking_lot::RwLock;
use semver::{Version, VersionReq};

use crate::bridge::PluginBridgeHub;
use crate::loader::{DynamicPluginLoader, PluginLoadConfig};
use crate::manifest::DiscoveredPlugin;
use crate::registry::PluginRegistry;
use crate::runtime::{runtime_adapter, PythonPluginSupervisor, PythonSupervisorConfig};
use crate::signature::{SignatureAlgorithm, SignatureVerifier};
use clawlegion_core::{PluginContext, PluginInfo, PluginState};

pub struct PluginRuntimeManager {
    registry: Arc<PluginRegistry>,
    bridge_hub: Arc<PluginBridgeHub>,
    plugin_configs: HashMap<String, HashMap<String, serde_json::Value>>,
    loader: DynamicPluginLoader,
    signature_verifier: Option<SignatureVerifier>,
    load_config: PluginLoadConfig,
    python_supervisor: PythonPluginSupervisor,
}

pub type PluginManager = PluginRuntimeManager;
pub type SharedPluginManager = Arc<RwLock<PluginRuntimeManager>>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginTriggerStatus {
    pub id: String,
    pub agent_id: String,
    pub enabled: bool,
    pub condition: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginRuntimeLog {
    pub plugin_id: String,
    pub runtime: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManagerStats {
    pub total_plugins: usize,
    pub active_plugins: usize,
    pub degraded_plugins: usize,
    pub failed_plugins: usize,
}

impl PluginRuntimeManager {
    pub fn new() -> Self {
        Self::with_load_config(PluginLoadConfig::default())
    }

    pub fn with_load_config(load_config: PluginLoadConfig) -> Self {
        let loader = DynamicPluginLoader::new(vec![load_config.plugin_dir.clone()]);
        let signature_verifier = if load_config.verify_signatures {
            load_config
                .public_key
                .clone()
                .map(|public_key| SignatureVerifier::new(public_key, SignatureAlgorithm::Ed25519))
        } else {
            None
        };

        Self {
            registry: Arc::new(PluginRegistry::new(
                PathBuf::from("./data/plugins"),
                PathBuf::from("./config/plugins"),
            )),
            bridge_hub: Arc::new(PluginBridgeHub::new()),
            plugin_configs: HashMap::new(),
            loader,
            signature_verifier,
            load_config,
            python_supervisor: PythonPluginSupervisor::new(),
        }
    }

    pub fn with_signature_verification(mut self, public_key: Vec<u8>) -> Self {
        self.signature_verifier = Some(SignatureVerifier::new(
            public_key.clone(),
            SignatureAlgorithm::Ed25519,
        ));
        self.load_config = self
            .load_config
            .clone()
            .with_signature_verification(public_key);
        self
    }

    pub fn set_plugin_config(
        &mut self,
        plugin_id: String,
        config: HashMap<String, serde_json::Value>,
    ) {
        self.plugin_configs.insert(plugin_id, config);
    }

    pub fn get_plugin_config(
        &self,
        plugin_id: &str,
    ) -> Option<&HashMap<String, serde_json::Value>> {
        self.plugin_configs.get(plugin_id)
    }

    pub fn discover(&mut self) -> Result<Vec<PluginInfo>> {
        let discovered = self.loader.discover()?;
        for plugin in discovered {
            self.register_discovered(plugin)?;
        }
        Ok(self.registry.list_plugins())
    }

    fn register_discovered(&self, plugin: DiscoveredPlugin) -> Result<()> {
        if self.registry.is_registered(&plugin.manifest.id) {
            return Ok(());
        }

        self.registry.register_manifest(
            plugin.manifest,
            Some(plugin.manifest_path),
            Some(plugin.entrypoint_path),
        )
    }

    pub fn resolve_dependencies(&self) -> Result<Vec<String>> {
        let plugins = self.registry.list_plugins();
        let mut indegree: HashMap<String, usize> = HashMap::new();
        let mut edges: HashMap<String, Vec<String>> = HashMap::new();
        let plugin_ids: HashSet<_> = plugins.iter().map(|plugin| plugin.id.clone()).collect();

        for plugin in &plugins {
            indegree.entry(plugin.id.clone()).or_insert(0);
            for dependency in &plugin.manifest.dependencies {
                if !plugin_ids.contains(&dependency.name) {
                    if dependency.optional {
                        continue;
                    }
                    return Err(anyhow!(
                        "plugin {} requires missing dependency {} ({})",
                        plugin.id,
                        dependency.name,
                        dependency.version_req
                    ));
                }
                *indegree.entry(plugin.id.clone()).or_insert(0) += 1;
                edges
                    .entry(dependency.name.clone())
                    .or_default()
                    .push(plugin.id.clone());
            }
        }

        let mut ready: VecDeque<String> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(plugin_id, _)| plugin_id.clone())
            .collect();
        let mut ordered = Vec::with_capacity(plugins.len());

        while let Some(plugin_id) = ready.pop_front() {
            ordered.push(plugin_id.clone());
            if let Some(children) = edges.get(&plugin_id) {
                for child in children {
                    if let Some(degree) = indegree.get_mut(child) {
                        *degree -= 1;
                        if *degree == 0 {
                            ready.push_back(child.clone());
                        }
                    }
                }
            }
        }

        if ordered.len() != plugins.len() {
            return Err(anyhow!("plugin dependency cycle detected"));
        }

        for plugin_id in &ordered {
            let _ = self
                .registry
                .set_plugin_state(plugin_id, PluginState::Resolved, None);
        }

        Ok(ordered)
    }

    pub fn load_dynamic(&mut self, plugin_path: &Path) -> Result<()> {
        let loaded = self
            .loader
            .load(plugin_path, self.signature_verifier.as_ref())?;
        if !self.registry.is_registered(&loaded.manifest.id) {
            self.registry.register_manifest(
                loaded.manifest.clone(),
                Some(loaded.manifest_path.clone()),
                Some(loaded.entrypoint_path.clone()),
            )?;
        }

        self.registry
            .set_plugin_state(&loaded.manifest.id, PluginState::Loaded, None)?;
        Ok(())
    }

    pub async fn initialize(&mut self, plugin_id: &str) -> Result<()> {
        let plugin = self
            .registry
            .get(plugin_id)
            .ok_or_else(|| anyhow!("plugin {} not found", plugin_id))?;
        let probe = runtime_adapter(&plugin.plugin_type)
            .probe(&plugin.manifest, plugin.load_path.as_deref())?;
        self.registry
            .set_plugin_state(plugin_id, probe.state.clone(), probe.detail.clone())?;
        self.registry
            .capability_registry()
            .register_plugin_capabilities(plugin_id, &plugin.manifest.capabilities);
        self.bridge_hub.register_plugin_capabilities(
            plugin_id,
            &plugin.manifest,
            &plugin.manifest.capabilities,
            self.plugin_configs.get(plugin_id),
        )?;
        self.start_supervised_runtime_if_needed(plugin_id, &plugin)?;

        if let Some(plugin_handle) = self.registry.plugin_handle(plugin_id) {
            let context = self.build_context(&plugin);
            let mut plugin = plugin_handle.write();
            futures_executor::block_on(plugin.init(context.clone()))
                .context("plugin init failed")?;
        }

        let final_state = match probe.state {
            PluginState::Degraded | PluginState::Failed => probe.state.clone(),
            _ => PluginState::Active,
        };
        self.registry
            .set_plugin_state(plugin_id, final_state, probe.detail.clone())?;
        self.registry
            .set_plugin_health(plugin_id, Some(probe.health))?;
        Ok(())
    }

    pub async fn initialize_all(&mut self) -> Result<()> {
        let ordered = self.resolve_dependencies()?;
        for plugin_id in ordered {
            let plugin = self.registry.get(&plugin_id).unwrap();
            if !plugin.enabled || plugin.state == PluginState::Disabled {
                continue;
            }
            self.initialize(&plugin_id).await?;
        }
        Ok(())
    }

    pub async fn enable(&mut self, plugin_id: &str) -> Result<()> {
        self.registry.set_enabled(plugin_id, true)?;
        self.registry.set_plugin_state(
            plugin_id,
            PluginState::Resolved,
            Some("plugin enabled".to_string()),
        )?;
        if let Some(plugin_handle) = self.registry.plugin_handle(plugin_id) {
            let mut plugin = plugin_handle.write();
            futures_executor::block_on(plugin.enable())?;
        }
        self.initialize(plugin_id).await
    }

    pub async fn disable(&mut self, plugin_id: &str) -> Result<()> {
        if let Some(plugin_handle) = self.registry.plugin_handle(plugin_id) {
            let mut plugin = plugin_handle.write();
            futures_executor::block_on(plugin.disable())?;
        }
        self.registry
            .capability_registry()
            .unregister_plugin(plugin_id);
        self.bridge_hub.unregister_plugin(plugin_id);
        let _ = self.python_supervisor.stop(plugin_id);
        self.registry.set_enabled(plugin_id, false)?;
        self.registry.set_plugin_state(
            plugin_id,
            PluginState::Disabled,
            Some("plugin disabled".to_string()),
        )?;
        Ok(())
    }

    pub async fn reload_config(&mut self, plugin_id: &str) -> Result<()> {
        let plugin = self
            .registry
            .get(plugin_id)
            .ok_or_else(|| anyhow!("plugin {} not found", plugin_id))?;
        let config = self
            .plugin_configs
            .get(plugin_id)
            .cloned()
            .unwrap_or_default();
        self.registry
            .set_plugin_state(plugin_id, PluginState::Reloading, None)?;

        if let Some(plugin_handle) = self.registry.plugin_handle(plugin_id) {
            let mut plugin_instance = plugin_handle.write();
            futures_executor::block_on(plugin_instance.on_config_reload(config.clone()))?;
        } else {
            let _ = plugin;
        }

        self.registry.set_plugin_state(
            plugin_id,
            PluginState::Active,
            Some("config reloaded".to_string()),
        )?;
        Ok(())
    }

    pub async fn unload(&mut self, plugin_id: &str) -> Result<()> {
        self.registry
            .set_plugin_state(plugin_id, PluginState::Stopping, None)?;
        if let Some(plugin_handle) = self.registry.plugin_handle(plugin_id) {
            let mut plugin = plugin_handle.write();
            futures_executor::block_on(plugin.shutdown())?;
        }
        self.registry
            .capability_registry()
            .unregister_plugin(plugin_id);
        self.bridge_hub.unregister_plugin(plugin_id);
        let _ = self.python_supervisor.stop(plugin_id);
        self.registry
            .set_plugin_state(plugin_id, PluginState::Stopped, None)?;
        Ok(())
    }

    pub async fn unload_all(&mut self) -> Result<()> {
        let plugin_ids: Vec<_> = self
            .registry
            .list_plugins()
            .into_iter()
            .map(|plugin| plugin.id)
            .collect();
        for plugin_id in plugin_ids {
            self.unload(&plugin_id).await?;
        }
        Ok(())
    }

    pub fn inspect(&self, plugin_id: &str) -> Result<PluginInfo> {
        self.registry
            .get(plugin_id)
            .ok_or_else(|| anyhow!("plugin {} not found", plugin_id))
    }

    pub fn health_report(&self) -> Vec<(String, PluginState, Option<String>)> {
        self.registry
            .list_plugins()
            .into_iter()
            .map(|plugin| (plugin.id, plugin.state, plugin.health))
            .collect()
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.registry.list_plugins()
    }

    pub fn stats(&self) -> PluginManagerStats {
        let plugins = self.registry.list_plugins();
        PluginManagerStats {
            total_plugins: plugins.len(),
            active_plugins: plugins
                .iter()
                .filter(|plugin| plugin.state == PluginState::Active)
                .count(),
            degraded_plugins: plugins
                .iter()
                .filter(|plugin| plugin.state == PluginState::Degraded)
                .count(),
            failed_plugins: plugins
                .iter()
                .filter(|plugin| plugin.state == PluginState::Failed)
                .count(),
        }
    }

    pub fn install(&mut self, source: &Path) -> Result<PluginInfo> {
        let manifest_path = if source.is_dir() {
            source.join("plugin.toml")
        } else {
            source.to_path_buf()
        };
        let discovered = DiscoveredPlugin::load(&manifest_path)?;
        self.validate_install_dependencies(&discovered.manifest)?;
        let install_root = self.load_config.plugin_dir.join(&discovered.manifest.id);
        let rollback_dir = self.load_config.plugin_dir.join(format!(
            ".rollback-{}-{}",
            discovered.manifest.id,
            uuid::Uuid::new_v4()
        ));
        if install_root.exists() {
            fs::rename(&install_root, &rollback_dir).with_context(|| {
                format!(
                    "failed to prepare rollback from {} to {}",
                    install_root.display(),
                    rollback_dir.display()
                )
            })?;
        }
        let install_result: Result<PluginInfo> = (|| {
            fs::create_dir_all(&install_root)
                .with_context(|| format!("failed to create {}", install_root.display()))?;
            copy_dir_contents(&discovered.plugin_dir, &install_root)?;

            if !self
                .loader
                .search_paths()
                .iter()
                .any(|path| path == &self.load_config.plugin_dir)
            {
                self.loader
                    .add_search_path(self.load_config.plugin_dir.clone());
                self.registry
                    .add_search_path(self.load_config.plugin_dir.clone());
            }

            let installed_manifest = install_root.join("plugin.toml");
            let loaded = self
                .loader
                .load(&installed_manifest, self.signature_verifier.as_ref())?;
            if self.registry.is_registered(&loaded.manifest.id) {
                self.registry.unregister(&loaded.manifest.id)?;
            }
            self.registry.register_manifest(
                loaded.manifest.clone(),
                Some(loaded.manifest_path.clone()),
                Some(loaded.entrypoint_path.clone()),
            )?;
            self.registry.set_plugin_state(
                &loaded.manifest.id,
                PluginState::Loaded,
                Some("installed".to_string()),
            )?;
            self.inspect(&loaded.manifest.id)
        })();

        match install_result {
            Ok(plugin) => {
                if rollback_dir.exists() {
                    let _ = fs::remove_dir_all(&rollback_dir);
                }
                Ok(plugin)
            }
            Err(e) => {
                let _ = fs::remove_dir_all(&install_root);
                if rollback_dir.exists() {
                    let _ = fs::rename(&rollback_dir, &install_root);
                }
                Err(e)
            }
        }
    }

    pub async fn uninstall(&mut self, plugin_id: &str) -> Result<()> {
        self.ensure_no_reverse_dependencies(plugin_id)?;
        if self.registry.is_registered(plugin_id) {
            self.unload(plugin_id).await?;
            self.registry.unregister(plugin_id)?;
        }
        let install_root = self.load_config.plugin_dir.join(plugin_id);
        if install_root.exists() {
            fs::remove_dir_all(&install_root)
                .with_context(|| format!("failed to remove {}", install_root.display()))?;
        }
        Ok(())
    }

    pub fn trust_key(&self, alias: &str, public_key_path: &Path) -> Result<PathBuf> {
        let trust_dir = self.registry.config_dir().join("trust");
        fs::create_dir_all(&trust_dir)
            .with_context(|| format!("failed to create {}", trust_dir.display()))?;
        let target_path = trust_dir.join(format!("{}.pub", alias));
        fs::copy(public_key_path, &target_path).with_context(|| {
            format!(
                "failed to copy trust key from {} to {}",
                public_key_path.display(),
                target_path.display()
            )
        })?;
        Ok(target_path)
    }

    pub fn sign_plugin(&self, plugin_id: &str, private_key_path: &Path) -> Result<PathBuf> {
        let plugin = self.inspect(plugin_id)?;
        let plugin_path = plugin
            .load_path
            .ok_or_else(|| anyhow!("plugin {} has no artifact path", plugin_id))?;
        let private_key = fs::read(private_key_path)
            .with_context(|| format!("failed to read {}", private_key_path.display()))?;
        crate::signature::sign_plugin_file(&plugin_path, &private_key)?;
        Ok(plugin_path.with_extension(format!(
            "{}.sig",
            plugin_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("bin")
        )))
    }

    pub fn capability_snapshot(&self) -> HashMap<String, Vec<String>> {
        self.registry.capability_registry().snapshot_by_kind()
    }

    pub fn bridge_snapshot(&self) -> HashMap<String, Vec<String>> {
        self.bridge_hub.snapshot()
    }

    pub fn bridge_hub(&self) -> Arc<PluginBridgeHub> {
        Arc::clone(&self.bridge_hub)
    }

    pub fn plugin_logs(&self, plugin_id: &str) -> Vec<PluginRuntimeLog> {
        let mut logs: Vec<PluginRuntimeLog> = self
            .python_supervisor
            .logs(plugin_id)
            .into_iter()
            .map(|line| PluginRuntimeLog {
                plugin_id: plugin_id.to_string(),
                runtime: "python".to_string(),
                message: line,
            })
            .collect();
        if let Some(info) = self.registry.get(plugin_id) {
            let runtime = format!("{:?}", info.plugin_type);
            logs.extend(info.errors.into_iter().map(|error| PluginRuntimeLog {
                plugin_id: plugin_id.to_string(),
                runtime: runtime.clone(),
                message: error,
            }));
        }
        logs
    }

    pub fn doctor_report(&self) -> Vec<serde_json::Value> {
        self.registry
            .list_plugins()
            .into_iter()
            .map(|plugin| {
                serde_json::json!({
                    "plugin_id": plugin.id,
                    "runtime": format!("{:?}", plugin.plugin_type),
                    "state": format!("{:?}", plugin.state),
                    "health": plugin.health,
                    "last_error": plugin.errors.last().cloned(),
                    "dependencies": plugin.manifest.dependencies,
                    "permissions": plugin.manifest.permissions,
                    "restart_count": self.python_supervisor.restart_count(&plugin.id),
                })
            })
            .collect()
    }

    pub fn sentinel_trigger_snapshot(&self) -> Vec<PluginTriggerStatus> {
        self.bridge_hub
            .sentinel_manager()
            .list_triggers()
            .into_iter()
            .map(|trigger| PluginTriggerStatus {
                id: trigger.id,
                agent_id: trigger.agent_id.to_string(),
                enabled: trigger.enabled,
                condition: format!("{:?}", trigger.condition),
            })
            .collect()
    }

    pub fn registry(&self) -> &PluginRegistry {
        self.registry.as_ref()
    }

    fn build_context(&self, plugin: &PluginInfo) -> PluginContext {
        let config = self
            .plugin_configs
            .get(&plugin.id)
            .cloned()
            .unwrap_or_default();
        PluginContext::new(
            plugin.id.clone(),
            plugin.plugin_type.clone(),
            config,
            self.registry.data_dir().join(&plugin.id),
            self.registry.config_dir().join(&plugin.id),
        )
    }

    fn start_supervised_runtime_if_needed(
        &self,
        plugin_id: &str,
        plugin: &PluginInfo,
    ) -> Result<()> {
        if plugin.plugin_type != clawlegion_core::PluginType::Python {
            return Ok(());
        }
        let Some(load_path) = plugin.load_path.as_deref() else {
            return Ok(());
        };
        let plugin_cfg = self.plugin_configs.get(plugin_id);
        let mut config = PythonSupervisorConfig::default();
        if let Some(cfg) = plugin_cfg {
            if let Some(enabled) = cfg.get("supervisor_enabled").and_then(|v| v.as_bool()) {
                config.enabled = enabled;
            }
            if let Some(max_restarts) = cfg.get("supervisor_max_restarts").and_then(|v| v.as_u64())
            {
                config.max_restarts = max_restarts as usize;
            }
            if let Some(backoff) = cfg
                .get("supervisor_restart_backoff_ms")
                .and_then(|v| v.as_u64())
            {
                config.restart_backoff_ms = backoff;
            }
            if let Some(args) = cfg.get("supervisor_args").and_then(|v| v.as_array()) {
                config.serve_args = args
                    .iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect();
            }
        }
        self.python_supervisor
            .ensure_started(plugin_id, load_path, &config)?;
        Ok(())
    }

    fn validate_install_dependencies(
        &self,
        manifest: &clawlegion_core::PluginManifest,
    ) -> Result<()> {
        for dep in &manifest.dependencies {
            let dep_info = self.registry.get(&dep.name);
            if dep_info.is_none() && !dep.optional {
                return Err(anyhow!(
                    "install blocked: missing required dependency {} ({}) for {}",
                    dep.name,
                    dep.version_req,
                    manifest.id
                ));
            }
            if let Some(dep_info) = dep_info {
                let dep_ver = Version::parse(&dep_info.manifest.version).with_context(|| {
                    format!(
                        "invalid dependency version {} for plugin {}",
                        dep_info.manifest.version, dep_info.id
                    )
                })?;
                let req = VersionReq::parse(&dep.version_req).with_context(|| {
                    format!(
                        "invalid dependency version requirement {} for {}",
                        dep.version_req, dep.name
                    )
                })?;
                if !req.matches(&dep_ver) {
                    return Err(anyhow!(
                        "install blocked: dependency {} version {} does not satisfy {}",
                        dep.name,
                        dep_info.manifest.version,
                        dep.version_req
                    ));
                }
            }
        }
        Ok(())
    }

    fn ensure_no_reverse_dependencies(&self, plugin_id: &str) -> Result<()> {
        let dependents: Vec<String> = self
            .registry
            .list_plugins()
            .into_iter()
            .filter(|plugin| plugin.id != plugin_id)
            .filter(|plugin| {
                plugin
                    .manifest
                    .dependencies
                    .iter()
                    .any(|dep| dep.name == plugin_id && !dep.optional)
            })
            .map(|plugin| plugin.id)
            .collect();
        if !dependents.is_empty() {
            return Err(anyhow!(
                "uninstall blocked: {} is required by {}",
                plugin_id,
                dependents.join(", ")
            ));
        }
        Ok(())
    }
}

impl Default for PluginRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

fn copy_dir_contents(source: &Path, target: &Path) -> Result<()> {
    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read directory {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            fs::create_dir_all(&target_path)
                .with_context(|| format!("failed to create {}", target_path.display()))?;
            copy_dir_contents(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy file from {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use clawlegion_core::{PluginManifest, PluginMetadata, PluginType};

    #[test]
    fn resolves_dependency_order() {
        let manager = PluginRuntimeManager::new();
        let base = PluginManifest::from_metadata(
            "base-plugin",
            PluginType::Config,
            PluginMetadata {
                name: "base-plugin".to_string(),
                version: "0.1.0".to_string(),
                description: "base".to_string(),
                author: "test".to_string(),
                core_version: "0.1.0".to_string(),
                dependencies: Vec::new(),
                tags: Vec::new(),
            },
        );
        let mut dependent = PluginManifest::from_metadata(
            "dependent-plugin",
            PluginType::Config,
            PluginMetadata {
                name: "dependent-plugin".to_string(),
                version: "0.1.0".to_string(),
                description: "dependent".to_string(),
                author: "test".to_string(),
                core_version: "0.1.0".to_string(),
                dependencies: Vec::new(),
                tags: Vec::new(),
            },
        );
        dependent
            .dependencies
            .push(clawlegion_core::PluginDependency {
                name: "base-plugin".to_string(),
                version_req: "^0.1".to_string(),
                optional: false,
            });
        manager
            .registry()
            .register_manifest(base, None, None)
            .expect("register base");
        manager
            .registry()
            .register_manifest(dependent, None, None)
            .expect("register dependent");

        let ordered = manager
            .resolve_dependencies()
            .expect("resolve dependencies");
        assert_eq!(
            ordered,
            vec!["base-plugin".to_string(), "dependent-plugin".to_string()]
        );
    }

    #[test]
    fn installs_manifest_directory() {
        let temp_root =
            std::env::temp_dir().join(format!("clawlegion-install-{}", uuid::Uuid::new_v4()));
        let source_dir = temp_root.join("source-plugin");
        let install_root = temp_root.join("installed");
        fs::create_dir_all(&source_dir).expect("create source dir");
        fs::create_dir_all(&install_root).expect("create install dir");
        fs::write(
            source_dir.join("plugin.toml"),
            r#"
id = "installed-plugin"
version = "0.1.0"
api_version = "v2"
runtime = "config"
entrypoint = "plugin.toml"

[metadata]
name = "installed-plugin"
version = "0.1.0"
description = "installed"
author = "test"
core_version = "0.1.0"
dependencies = []
tags = []
"#,
        )
        .expect("write manifest");

        let mut manager = PluginRuntimeManager::with_load_config(
            PluginLoadConfig::new(install_root.clone()).without_signature_verification(),
        );
        let installed = manager.install(&source_dir).expect("install plugin");
        assert_eq!(installed.id, "installed-plugin");
        assert!(install_root.join("installed-plugin/plugin.toml").exists());

        fs::remove_dir_all(&temp_root).expect("cleanup");
    }
}
