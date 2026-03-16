use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    BuiltIn,
    Dynamic,
    Python,
    Remote,
    Config,
}

impl PluginType {
    pub fn runtime_family(&self) -> PluginRuntimeFamily {
        match self {
            Self::BuiltIn | Self::Dynamic => PluginRuntimeFamily::NativeAbi,
            Self::Python => PluginRuntimeFamily::HostProcess,
            Self::Remote => PluginRuntimeFamily::RemoteProtocol,
            Self::Config => PluginRuntimeFamily::Config,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimeFamily {
    NativeAbi,
    HostProcess,
    RemoteProtocol,
    Config,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapabilityKind {
    LlmProvider,
    Tool,
    Skill,
    Storage,
    Agent,
    Trigger,
    Watcher,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermissionScope {
    Network,
    Filesystem,
    Secrets,
    Models,
    OrganizationData,
    Webhook,
    Subprocess,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    Discovered,
    Verified,
    Resolved,
    Loaded,
    Initialized,
    Active,
    Degraded,
    Reloading,
    Stopping,
    Stopped,
    Failed,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginDependency {
    pub name: String,
    pub version_req: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginPermission {
    pub scope: PluginPermissionScope,
    #[serde(default)]
    pub resource: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginCapabilityDescriptor {
    pub id: String,
    pub kind: PluginCapabilityKind,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub interface: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PluginUiMetadata {
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PluginSignature {
    #[serde(default)]
    pub algorithm: Option<String>,
    #[serde(default)]
    pub public_key_id: Option<String>,
    #[serde(default)]
    pub signature_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PluginHealthcheck {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub interval_secs: Option<u64>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub core_version: String,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for PluginMetadata {
    fn default() -> Self {
        Self {
            name: "unnamed-plugin".to_string(),
            version: "0.1.0".to_string(),
            description: String::new(),
            author: "unknown".to_string(),
            core_version: env!("CARGO_PKG_VERSION").to_string(),
            dependencies: Vec::new(),
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    pub version: String,
    pub api_version: String,
    pub runtime: PluginType,
    pub entrypoint: String,
    pub metadata: PluginMetadata,
    #[serde(default)]
    pub capabilities: Vec<PluginCapabilityDescriptor>,
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    #[serde(default)]
    pub compatible_host_versions: Vec<String>,
    #[serde(default)]
    pub signature: Option<PluginSignature>,
    #[serde(default)]
    pub healthcheck: Option<PluginHealthcheck>,
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub ui_metadata: PluginUiMetadata,
}

impl PluginManifest {
    pub fn from_metadata(
        id: impl Into<String>,
        runtime: PluginType,
        metadata: PluginMetadata,
    ) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            version: metadata.version.clone(),
            api_version: "v2".to_string(),
            runtime,
            entrypoint: id.clone(),
            capabilities: Vec::new(),
            permissions: Vec::new(),
            dependencies: metadata.dependencies.clone(),
            compatible_host_versions: vec![metadata.core_version.clone()],
            signature: None,
            healthcheck: None,
            config_schema: None,
            ui_metadata: PluginUiMetadata::default(),
            metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PluginContext {
    pub plugin_id: String,
    pub runtime: PluginType,
    pub config: HashMap<String, serde_json::Value>,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub host_version: String,
}

impl PluginContext {
    pub fn new(
        plugin_id: impl Into<String>,
        runtime: PluginType,
        config: HashMap<String, serde_json::Value>,
        data_dir: PathBuf,
        config_dir: PathBuf,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            runtime,
            config,
            data_dir,
            config_dir,
            host_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn get_config<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.config
            .get(key)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginStatus {
    pub state: PluginState,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub last_transition_unix: Option<i64>,
}

impl PluginStatus {
    pub fn new(state: PluginState) -> Self {
        Self {
            state,
            detail: None,
            last_transition_unix: Some(chrono::Utc::now().timestamp()),
        }
    }
}

impl Default for PluginStatus {
    fn default() -> Self {
        Self::new(PluginState::Discovered)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginInfo {
    pub id: String,
    pub metadata: PluginMetadata,
    pub manifest: PluginManifest,
    pub plugin_type: PluginType,
    pub state: PluginState,
    #[serde(default)]
    pub status: PluginStatus,
    #[serde(default)]
    pub load_path: Option<PathBuf>,
    #[serde(default)]
    pub manifest_path: Option<PathBuf>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub health: Option<String>,
    #[serde(default)]
    pub errors: Vec<String>,
}

impl PluginInfo {
    pub fn new(
        id: String,
        manifest: PluginManifest,
        state: PluginState,
        load_path: Option<PathBuf>,
        manifest_path: Option<PathBuf>,
    ) -> Self {
        let metadata = manifest.metadata.clone();
        let plugin_type = manifest.runtime.clone();
        Self {
            id,
            metadata,
            manifest,
            plugin_type,
            state: state.clone(),
            status: PluginStatus::new(state),
            load_path,
            manifest_path,
            enabled: true,
            health: None,
            errors: Vec::new(),
        }
    }
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    fn manifest(&self) -> PluginManifest {
        PluginManifest::from_metadata(
            self.metadata().name.clone(),
            PluginType::Dynamic,
            self.metadata(),
        )
    }

    async fn init(&mut self, _ctx: PluginContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    async fn enable(&mut self) -> Result<()> {
        Ok(())
    }

    async fn disable(&mut self) -> Result<()> {
        Ok(())
    }

    async fn on_config_reload(
        &mut self,
        _new_config: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        Ok(())
    }
}
