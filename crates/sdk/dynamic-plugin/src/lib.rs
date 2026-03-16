//! ClawLegion Dynamic Plugin SDK
//!
//! This SDK provides the necessary traits and types for developing dynamic plugins
//! that can be loaded at runtime by the ClawLegion system.
//!
//! # Example
//!
//! ```rust,ignore
//! use clawlegion_plugin_sdk::{Plugin, PluginMetadata, PluginContext, plugin};
//!
//! pub struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn metadata(&self) -> &PluginMetadata {
//!         // ...
//!         todo!()
//!     }
//!
//!     async fn init(&mut self, ctx: PluginContext) -> clawlegion_core::Result<()> {
//!         // ...
//!         Ok(())
//!     }
//!
//!     async fn shutdown(&mut self) -> clawlegion_core::Result<()> {
//!         // ...
//!         Ok(())
//!     }
//! }
//!
//! plugin!(MyPlugin);
//! ```

pub use clawlegion_core::{
    Error, LlmMessage, LlmOptions, LlmProvider, LlmProviderConfig, LlmResponse, Plugin,
    PluginCapabilityDescriptor, PluginContext, PluginInfo, PluginManifest, PluginMetadata,
    PluginState, PluginType, PluginUiMetadata, Result, StreamChunk, TokenUsage,
};

pub use clawlegion_plugin::{
    PluginLoadConfig, PluginManager, PluginRegistry, SignatureAlgorithm, SignatureVerifier,
};

pub use clawlegion_agent::{
    Agent, AgentCapabilities, AgentConfig, AgentStatus, HeartbeatContext, HeartbeatResult,
    HeartbeatTrigger,
};

pub use clawlegion_capability::{
    Mcp, McpContext, McpMetadata, McpResult, McpVisibility, Skill, SkillContext, SkillInput,
    SkillMetadata, SkillOutput, Tool, ToolContext, ToolMetadata, ToolResult, ToolVisibility,
    Visibility,
};

/// Plugin attribute macro
///
/// Use this macro to mark a struct as a ClawLegion plugin.
/// It generates the necessary boilerplate for plugin registration.
#[macro_export]
macro_rules! plugin {
    ($struct_name:ident) => {
        #[no_mangle]
        pub extern "C" fn _plugin_metadata() -> $crate::PluginMetadata {
            $struct_name::default_metadata()
        }

        #[no_mangle]
        pub extern "C" fn _plugin_create() -> *mut dyn $crate::Plugin {
            Box::into_raw(Box::new($struct_name::new()))
        }

        #[no_mangle]
        pub extern "C" fn _plugin_destroy(ptr: *mut dyn $crate::Plugin) {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
    };
}

/// Skill attribute macro
///
/// Use this macro to define a skill that can be loaded by agents.
#[macro_export]
macro_rules! skill {
    ($struct_name:ident) => {
        #[no_mangle]
        pub extern "C" fn _skill_metadata() -> $crate::SkillMetadata {
            $struct_name::default_metadata()
        }

        #[no_mangle]
        pub extern "C" fn _skill_create() -> *mut dyn $crate::Skill {
            Box::into_raw(Box::new($struct_name::new()))
        }
    };
}

/// Tool attribute macro
///
/// Use this macro to define a tool that can be used by agents.
#[macro_export]
macro_rules! tool {
    ($struct_name:ident) => {
        #[no_mangle]
        pub extern "C" fn _tool_metadata() -> $crate::ToolMetadata {
            $struct_name::default_metadata()
        }

        #[no_mangle]
        pub extern "C" fn _tool_create() -> *mut dyn $crate::Tool {
            Box::into_raw(Box::new($struct_name::new()))
        }
    };
}

/// Plugin builder for constructing plugins with dependencies
pub struct PluginBuilder {
    name: String,
    version: String,
    description: String,
    author: Option<String>,
    tags: Vec<String>,
}

impl PluginBuilder {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: String::new(),
            author: None,
            tags: vec![],
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    pub fn build(self) -> PluginMetadata {
        PluginMetadata {
            name: self.name,
            version: self.version,
            description: self.description,
            author: self.author.unwrap_or_else(|| "unknown".to_string()),
            core_version: env!("CARGO_PKG_VERSION").to_string(),
            dependencies: vec![],
            tags: self.tags,
        }
    }

    pub fn build_manifest(
        self,
        runtime: PluginType,
        entrypoint: impl Into<String>,
    ) -> PluginManifest {
        let metadata = self.build();
        PluginManifest {
            id: metadata.name.clone(),
            version: metadata.version.clone(),
            api_version: "v2".to_string(),
            runtime,
            entrypoint: entrypoint.into(),
            metadata: metadata.clone(),
            capabilities: Vec::<PluginCapabilityDescriptor>::new(),
            permissions: Vec::new(),
            dependencies: metadata.dependencies.clone(),
            compatible_host_versions: vec![metadata.core_version.clone()],
            signature: None,
            healthcheck: None,
            config_schema: None,
            ui_metadata: PluginUiMetadata::default(),
        }
    }
}

/// Skill builder
pub struct SkillBuilder {
    name: String,
    version: String,
    description: String,
    visibility: Visibility,
    tags: Vec<String>,
}

impl SkillBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: "1.0.0".to_string(),
            description: String::new(),
            visibility: Visibility::Public,
            tags: vec![],
        }
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn build(self) -> SkillMetadata {
        SkillMetadata {
            name: self.name,
            version: self.version,
            description: self.description,
            author: None,
            visibility: self.visibility,
            skill_type: clawlegion_capability::skill::SkillType::default(),
            execution_mode: clawlegion_capability::skill::ExecutionMode::default(),
            tags: self.tags,
            required_tools: vec![],
            required_mcps: vec![],
            dependencies: vec![],
            config_path: None,
        }
    }
}

/// Tool builder
pub struct ToolBuilder {
    name: String,
    version: String,
    description: String,
    visibility: ToolVisibility,
    tags: Vec<String>,
    input_schema: serde_json::Value,
}

impl ToolBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: "1.0.0".to_string(),
            description: String::new(),
            visibility: ToolVisibility::Public,
            tags: vec![],
            input_schema: serde_json::json!({"type": "object"}),
        }
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn visibility(mut self, visibility: ToolVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = schema;
        self
    }

    pub fn build(self) -> ToolMetadata {
        ToolMetadata {
            name: self.name,
            version: self.version,
            description: self.description,
            visibility: self.visibility,
            tags: self.tags,
            input_schema: self.input_schema,
            output_schema: None,
            requires_llm: false,
        }
    }
}

/// Helper for creating plugin contexts
pub fn create_plugin_context(
    config: std::collections::HashMap<String, serde_json::Value>,
    data_dir: std::path::PathBuf,
    config_dir: std::path::PathBuf,
) -> PluginContext {
    PluginContext::new(
        "sdk-plugin",
        PluginType::Dynamic,
        config,
        data_dir,
        config_dir,
    )
}

/// Create a minimal plugin scaffold (`plugin.toml` + `src/lib.rs`) for protocol-first v2 plugins.
pub fn scaffold_plugin_new(
    root: impl AsRef<std::path::Path>,
    plugin_name: &str,
    runtime: PluginType,
    entrypoint: &str,
) -> crate::Result<()> {
    let root = root.as_ref();
    std::fs::create_dir_all(root.join("src")).map_err(|e| {
        crate::Error::Plugin(clawlegion_core::PluginError::LoadFailed(e.to_string()))
    })?;

    let manifest = PluginBuilder::new(plugin_name, "0.1.0").build_manifest(runtime, entrypoint);
    let manifest_toml = toml::to_string_pretty(&manifest).map_err(|e| {
        crate::Error::Plugin(clawlegion_core::PluginError::LoadFailed(format!(
            "failed to serialize manifest: {}",
            e
        )))
    })?;
    std::fs::write(root.join("plugin.toml"), manifest_toml).map_err(|e| {
        crate::Error::Plugin(clawlegion_core::PluginError::LoadFailed(e.to_string()))
    })?;

    let lib_rs = r#"use async_trait::async_trait;
use clawlegion_plugin_sdk::{plugin, Plugin, PluginBuilder, PluginContext, PluginMetadata};

pub struct ExamplePlugin {
    metadata: PluginMetadata,
}

impl ExamplePlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginBuilder::new("example-plugin", "0.1.0")
                .description("example plugin")
                .author("clawlegion")
                .build(),
        }
    }

    pub fn default_metadata() -> PluginMetadata {
        PluginBuilder::new("example-plugin", "0.1.0")
            .description("example plugin")
            .author("clawlegion")
            .build()
    }
}

#[async_trait]
impl Plugin for ExamplePlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    async fn init(&mut self, _ctx: PluginContext) -> anyhow::Result<()> {
        Ok(())
    }
}

plugin!(ExamplePlugin);
"#;
    std::fs::write(root.join("src/lib.rs"), lib_rs).map_err(|e| {
        crate::Error::Plugin(clawlegion_core::PluginError::LoadFailed(e.to_string()))
    })?;
    Ok(())
}

/// LLM Provider Plugin trait
///
/// This trait extends the base Plugin trait with LLM provider-specific functionality.
/// Implement this trait to create a dynamic LLM provider plugin.
pub trait LlmProviderPlugin: Plugin {
    /// Get the provider type string (e.g., "openai", "anthropic")
    fn provider_type(&self) -> &str;

    /// Create an LLM provider instance from configuration
    fn create_provider(
        &self,
        config: &LlmProviderConfig,
    ) -> crate::Result<std::sync::Arc<dyn LlmProvider>>;
}

/// LLM Provider Plugin Factory
///
/// Trait for creating LLM provider instances from plugins.
pub trait LlmProviderFactory: Send + Sync {
    /// Get the provider type this factory supports
    fn provider_type(&self) -> &str;

    /// Create a provider instance from configuration
    fn create(&self, config: &LlmProviderConfig) -> crate::Result<std::sync::Arc<dyn LlmProvider>>;
}

/// Macro for registering an LLM provider plugin
#[macro_export]
macro_rules! llm_provider {
    ($struct_name:ident, $provider_type:expr) => {
        #[no_mangle]
        pub extern "C" fn _plugin_metadata() -> $crate::PluginMetadata {
            $struct_name::default_metadata()
        }

        #[no_mangle]
        pub extern "C" fn _plugin_create() -> *mut dyn $crate::Plugin {
            Box::into_raw(Box::new($struct_name::new()))
        }

        #[no_mangle]
        pub extern "C" fn _plugin_destroy(ptr: *mut dyn $crate::Plugin) {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }

        #[no_mangle]
        pub extern "C" fn _llm_provider_type() -> &'static str {
            $provider_type
        }

        #[no_mangle]
        pub extern "C" fn _llm_provider_create(
            config: &$crate::LlmProviderConfig,
        ) -> *mut dyn $crate::LlmProvider {
            let plugin = $struct_name::new();
            match plugin.create_provider(config) {
                Ok(provider) => Box::into_raw(Box::new(provider)),
                Err(_) => std::ptr::null_mut(),
            }
        }
    };
}
