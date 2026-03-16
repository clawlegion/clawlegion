//! Configuration system

use crate::{ConfigError, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Company identifier
pub type CompanyId = uuid::Uuid;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// System configuration
    pub system: SystemConfig,

    /// Company configurations
    pub companies: HashMap<CompanyId, CompanyConfig>,

    /// Plugin configurations
    pub plugins: HashMap<String, PluginConfigEntry>,

    /// LLM provider configurations
    pub llm_providers: HashMap<String, LlmConfig>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            system: SystemConfig::default(),
            companies: HashMap::new(),
            plugins: HashMap::new(),
            llm_providers: HashMap::new(),
        }
    }

    /// Load configuration from a TOML file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|_| {
            Error::Config(ConfigError::NotFound(format!(
                "Config file not found: {}",
                path.display()
            )))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            Error::Config(ConfigError::ParseError(format!(
                "Failed to parse config: {}",
                e
            )))
        })?;

        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            Error::Config(ConfigError::ParseError(format!(
                "Failed to serialize config: {}",
                e
            )))
        })?;

        std::fs::write(path, content).map_err(|e| {
            Error::Config(ConfigError::ParseError(format!(
                "Failed to write config file: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Get a company config
    pub fn get_company(&self, id: &CompanyId) -> Option<&CompanyConfig> {
        self.companies.get(id)
    }

    /// Add or update a company config
    pub fn set_company(&mut self, id: CompanyId, config: CompanyConfig) {
        self.companies.insert(id, config);
    }

    /// Get a plugin config
    pub fn get_plugin(&self, name: &str) -> Option<&PluginConfigEntry> {
        self.plugins.get(name)
    }

    /// Set a plugin config
    pub fn set_plugin(&mut self, name: String, config: PluginConfigEntry) {
        self.plugins.insert(name, config);
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// System-wide configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// System name
    pub name: String,

    /// Data directory path
    pub data_dir: PathBuf,

    /// Config directory path
    pub config_dir: PathBuf,

    /// Log level
    pub log_level: String,

    /// Enable telemetry
    pub telemetry_enabled: bool,

    /// API server configuration
    #[serde(default)]
    pub api_server: ApiServerConfig,

    /// Plugin trust policy
    #[serde(default)]
    pub plugin_trust: PluginTrustPolicy,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            name: "ClawLegion".to_string(),
            data_dir: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("clawlegion"),
            config_dir: dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("clawlegion"),
            log_level: "info".to_string(),
            telemetry_enabled: false,
            api_server: ApiServerConfig::default(),
            plugin_trust: PluginTrustPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginTrustMode {
    Development,
    Production,
}

impl Default for PluginTrustMode {
    fn default() -> Self {
        Self::Development
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTrustPolicy {
    #[serde(default)]
    pub mode: PluginTrustMode,
    #[serde(default)]
    pub public_key_path: Option<PathBuf>,
}

impl Default for PluginTrustPolicy {
    fn default() -> Self {
        Self {
            mode: PluginTrustMode::Development,
            public_key_path: None,
        }
    }
}

/// Company configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConfig {
    /// Company ID
    pub id: CompanyId,

    /// Company name
    pub name: String,

    /// Company description
    pub description: Option<String>,

    /// Issue prefix (e.g., "ACME" for ACME-123)
    pub issue_prefix: String,

    /// Monthly budget in cents
    pub budget_monthly_cents: u64,

    /// Require board approval for new agents
    pub require_approval_for_new_agents: bool,

    /// Brand color (hex)
    pub brand_color: Option<String>,
}

/// Plugin configuration entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigEntry {
    /// Whether the plugin is enabled
    pub enabled: bool,

    /// Plugin-specific configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Plugin load priority (higher = loaded first)
    pub priority: i32,
}

impl PluginConfigEntry {
    pub fn new() -> Self {
        Self {
            enabled: true,
            config: HashMap::new(),
            priority: 0,
        }
    }
}

impl Default for PluginConfigEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider type (e.g., "openai", "anthropic")
    pub provider: String,

    /// Model name
    pub model: String,

    /// API key (can be empty if using environment variable)
    pub api_key: Option<String>,

    /// API base URL (for custom endpoints)
    pub api_base: Option<String>,

    /// Default temperature
    pub default_temperature: Option<f64>,

    /// Default max tokens
    pub default_max_tokens: Option<u64>,
}

/// Hot reload configuration
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    /// Enable hot reload
    pub enabled: bool,

    /// Watch interval in seconds
    pub watch_interval_secs: u64,

    /// Config files to watch
    pub watched_files: Vec<PathBuf>,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            watch_interval_secs: 5,
            watched_files: vec![],
        }
    }
}

/// Configuration manager trait
pub trait ConfigManager: Send + Sync {
    /// Get the current configuration
    fn config(&self) -> &Config;

    /// Reload configuration from file
    fn reload(&mut self) -> Result<()>;

    /// Get a config value with storage override
    /// Storage takes priority over file config
    fn get_value<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned;

    /// Set a config value (stored in storage, takes priority)
    fn set_value(&mut self, key: &str, value: serde_json::Value) -> Result<()>;
}

/// API Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiServerConfig {
    /// Enable API server
    #[serde(default)]
    pub enabled: bool,

    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to bind to
    #[serde(default = "default_port")]
    pub port: u16,

    /// Allowed CORS origins
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_host(),
            port: default_port(),
            cors_origins: vec![],
        }
    }
}
