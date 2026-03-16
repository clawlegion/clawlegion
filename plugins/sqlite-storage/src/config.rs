//! SQLite storage plugin configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SQLite storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteStorageConfig {
    /// Path to the SQLite database file
    #[serde(default = "default_database_path")]
    pub database_path: PathBuf,

    /// Whether to automatically create tables on init
    #[serde(default = "default_create_tables")]
    pub create_tables: bool,

    /// Enable WAL mode for better concurrency
    #[serde(default = "default_wal_mode")]
    pub wal_mode: bool,

    /// Cache size in pages (negative = KB)
    #[serde(default = "default_cache_size")]
    pub cache_size: i32,

    /// Maximum queued database operations in the worker
    #[serde(default = "default_worker_queue_capacity")]
    pub worker_queue_capacity: usize,

    /// Timeout for a single database operation in milliseconds
    #[serde(default = "default_operation_timeout_ms")]
    pub operation_timeout_ms: u64,
}

fn default_database_path() -> PathBuf {
    PathBuf::from("./data/legion.db")
}

fn default_create_tables() -> bool {
    true
}

fn default_wal_mode() -> bool {
    true
}

fn default_cache_size() -> i32 {
    -2000 // 2MB cache
}

fn default_worker_queue_capacity() -> usize {
    1024
}

fn default_operation_timeout_ms() -> u64 {
    30_000
}

impl Default for SqliteStorageConfig {
    fn default() -> Self {
        Self {
            database_path: default_database_path(),
            create_tables: default_create_tables(),
            wal_mode: default_wal_mode(),
            cache_size: default_cache_size(),
            worker_queue_capacity: default_worker_queue_capacity(),
            operation_timeout_ms: default_operation_timeout_ms(),
        }
    }
}

impl SqliteStorageConfig {
    /// Create config from JSON value
    pub fn from_value(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value.clone())
    }

    /// Merge with default config
    pub fn merge_with_defaults(
        config: &clawlegion_plugin_sdk::PluginContext,
    ) -> clawlegion_plugin_sdk::Result<Self> {
        let mut result = Self::default();

        // Override with provided config
    if let Some(db_path) = config.get_config::<String>("database_path") {
        result.database_path = PathBuf::from(db_path);
    }

    if let Some(create_tables) = config.get_config::<bool>("create_tables") {
        result.create_tables = create_tables;
    }

    if let Some(wal_mode) = config.get_config::<bool>("wal_mode") {
            result.wal_mode = wal_mode;
        }

    if let Some(cache_size) = config.get_config::<i32>("cache_size") {
        result.cache_size = cache_size;
    }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteStorageConfig;

    #[test]
    fn default_config_includes_worker_limits() {
        let config = SqliteStorageConfig::default();

        assert_eq!(config.worker_queue_capacity, 1024);
        assert_eq!(config.operation_timeout_ms, 30_000);
    }
}
