//! Error types for ClawLegion

use thiserror::Error;

/// Main error type for ClawLegion
#[derive(Error, Debug)]
pub enum Error {
    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),

    #[error("Agent error: {0}")]
    Agent(#[from] AgentError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    #[error("Org error: {0}")]
    Org(#[from] OrgError),

    #[error("Sentinel error: {0}")]
    Sentinel(#[from] SentinelError),

    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Plugin-specific errors
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin load failed: {0}")]
    LoadFailed(String),

    #[error("Plugin init failed: {0}")]
    InitFailed(String),

    #[error("Plugin signature verification failed")]
    SignatureInvalid,

    #[error("Plugin version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    #[error("Plugin dependency missing: {0}")]
    DependencyMissing(String),
}

/// Agent-specific errors
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Agent already exists: {0}")]
    AlreadyExists(String),

    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Agent wakeup failed: {0}")]
    WakeupFailed(String),
}

/// Storage-specific errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Storage operation failed: {0}")]
    OperationFailed(String),

    #[error("Memory compression failed: {0}")]
    CompressionFailed(String),
}

/// Capability-specific errors
#[derive(Error, Debug)]
pub enum CapabilityError {
    #[error("Capability not found: {0}")]
    NotFound(String),

    #[error("Capability access denied: {0}")]
    AccessDenied(String),

    #[error("Capability execution failed: {0}")]
    ExecutionFailed(String),
}

/// Org-specific errors
#[derive(Error, Debug)]
pub enum OrgError {
    #[error("Company not found: {0}")]
    CompanyNotFound(String),

    #[error("Agent not in org: {0}")]
    AgentNotFound(String),

    #[error("Invalid org structure: {0}")]
    InvalidStructure(String),

    #[error("Cycle detected in reporting hierarchy")]
    CycleDetected,
}

/// Sentinel-specific errors
#[derive(Error, Debug)]
pub enum SentinelError {
    #[error("Watchdog error: {0}")]
    Watchdog(String),

    #[error("Trigger registration failed: {0}")]
    TriggerRegistration(String),

    #[error("Wakeup condition not met")]
    ConditionNotMet,
}

/// LLM-specific errors
#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("LLM request failed: {0}")]
    RequestFailed(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

/// Config-specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config not found: {0}")]
    NotFound(String),

    #[error("Config parse error: {0}")]
    ParseError(String),

    #[error("Config validation failed: {0}")]
    ValidationError(String),
}
