//! SQLite Storage Plugin for ClawLegion
//!
//! This plugin provides persistent storage using SQLite with a dedicated worker thread.
//! It implements both `Storage` and `ConfigStorage` traits from ClawLegion core.
//!
//! # Features
//!
//! - **Persistent Storage**: All data is stored in a SQLite database
//! - **Dedicated DB Worker**: A single SQLite connection is isolated to one worker thread
//! - **Memory Management**: Full support for Ebbinghaus forgetting curve
//! - **Configuration Storage**: TOML-based configuration management
//! - **Async Compatible**: Async APIs submit work through a bounded queue
//!
//! # Configuration
//!
//! ```toml
//! [plugins.sqlite-storage]
//! enabled = true
//! database_path = "./data/legion.db"
//! create_tables = true
//! wal_mode = true
//! cache_size = -2000
//! worker_queue_capacity = 1024
//! operation_timeout_ms = 30000
//! ```

mod config;
mod schema;
mod storage;

pub use config::SqliteStorageConfig;
pub use storage::SqliteStorage;

use async_trait::async_trait;
use clawlegion_plugin_sdk::{plugin, Plugin, PluginContext, PluginMetadata};
use std::sync::Arc;
use tracing::info;

/// SQLite Storage Plugin
pub struct SqliteStoragePlugin {
    metadata: PluginMetadata,
    storage: Option<Arc<SqliteStorage>>,
}

impl SqliteStoragePlugin {
    pub fn new() -> Self {
        Self {
            metadata: clawlegion_plugin_sdk::PluginBuilder::new("sqlite-storage", "0.1.0")
                .description("SQLite-based persistent storage plugin for ClawLegion")
                .author("ClawLegion Team")
                .tag("storage")
                .tag("sqlite")
                .tag("persistence")
                .build(),
            storage: None,
        }
    }

    pub fn default_metadata() -> PluginMetadata {
        clawlegion_plugin_sdk::PluginBuilder::new("sqlite-storage", "0.1.0")
            .description("SQLite-based persistent storage plugin for ClawLegion")
            .author("ClawLegion Team")
            .tag("storage")
            .tag("sqlite")
            .tag("persistence")
            .build()
    }

    /// Get the storage instance
    pub fn get_storage(&self) -> Option<Arc<SqliteStorage>> {
        self.storage.clone()
    }
}

impl Default for SqliteStoragePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for SqliteStoragePlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    async fn init(&mut self, ctx: PluginContext) -> anyhow::Result<()> {
        info!("Initializing SQLite storage plugin...");

        // Merge config with defaults
        let config = SqliteStorageConfig::merge_with_defaults(&ctx)?;

        // Ensure data directory is set correctly
        let data_dir = &ctx.data_dir;
        let database_path = if config.database_path.is_relative() {
            data_dir.join(&config.database_path)
        } else {
            config.database_path.clone()
        };

        info!("Database path: {}", database_path.display());

        // Create storage with updated path
        let mut final_config = config;
        final_config.database_path = database_path;

        let storage = SqliteStorage::new(final_config)?;

        self.storage = Some(Arc::new(storage));

        info!("SQLite storage plugin initialized successfully");

        Ok(())
    }

async fn shutdown(&mut self) -> anyhow::Result<()> {
        info!("Shutting down SQLite storage plugin...");

        if let Some(ref storage) = self.storage {
            use clawlegion_core::Storage;
            storage.shutdown().await?;
        }

        info!("SQLite storage plugin shutdown complete");

        Ok(())
    }

async fn enable(&mut self) -> anyhow::Result<()> {
        info!("SQLite storage plugin enabled");
        Ok(())
    }

async fn disable(&mut self) -> anyhow::Result<()> {
        info!("SQLite storage plugin disabled");
        Ok(())
    }

    async fn on_config_reload(
        &mut self,
        _config: std::collections::HashMap<String, serde_json::Value>,
) -> anyhow::Result<()> {
        info!("SQLite storage plugin config reloaded");
        // In a more advanced implementation, you could reinitialize the storage
        // with the new config here
        Ok(())
    }
}

// Register the plugin
plugin!(SqliteStoragePlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use clawlegion_core::{MemoryCategory, MemoryEntry, MemorySearchQuery, Storage};
    use std::collections::HashMap;
    use tempfile::TempDir;
    use uuid::Uuid;
    use chrono::Utc;

    fn create_test_storage() -> (TempDir, SqliteStorage) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SqliteStorageConfig {
            database_path: db_path,
            create_tables: true,
            wal_mode: true,
            cache_size: -100,
        };

        let storage = SqliteStorage::new(config).unwrap();
        (temp_dir, storage)
    }

    #[tokio::test]
    async fn test_key_value_storage() {
        let (_temp_dir, storage) = create_test_storage();

        // Test set and get
        let value = serde_json::json!({"key": "value", "number": 42});
        storage.set("test_key", value.clone()).await.unwrap();

        let retrieved = storage.get("test_key").await.unwrap();
        assert_eq!(retrieved, Some(value));

        // Test exists
        assert!(storage.exists("test_key").await.unwrap());
        assert!(!storage.exists("nonexistent").await.unwrap());

        // Test delete
        assert!(storage.delete("test_key").await.unwrap());
        assert!(!storage.exists("test_key").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_keys() {
        let (_temp_dir, storage) = create_test_storage();

        storage.set("prefix_key1", serde_json::json!("value1")).await.unwrap();
        storage.set("prefix_key2", serde_json::json!("value2")).await.unwrap();
        storage.set("other_key", serde_json::json!("value3")).await.unwrap();

        let keys = storage.list_keys("prefix_").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"prefix_key1".to_string()));
        assert!(keys.contains(&"prefix_key2".to_string()));
    }

    #[tokio::test]
    async fn test_memory_storage() {
        let (_temp_dir, storage) = create_test_storage();

        let memory = MemoryEntry {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            agent_id: Some(clawlegion_core::AgentId::from(Uuid::new_v4())),
            content: "Test memory content".to_string(),
            category: MemoryCategory::ShortTerm,
            importance: 0.8,
            access_count: 0,
            last_accessed_at: Utc::now(),
            created_at: Utc::now(),
            tags: vec!["test".to_string()],
            related_messages: vec![],
            expires_at: None,
        };

        let id = storage.store_memory(memory.clone()).await.unwrap();
        assert_eq!(id, memory.id);

        let retrieved = storage.get_memory(id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.content, memory.content);
        assert_eq!(retrieved.importance, memory.importance);
    }

    #[tokio::test]
    async fn test_memory_search() {
        let (_temp_dir, storage) = create_test_storage();

        let company_id = Uuid::new_v4();

        // Store multiple memories
        for i in 0..5 {
            let memory = MemoryEntry {
                id: Uuid::new_v4(),
                company_id,
                agent_id: None,
                content: format!("Test memory {}", i),
                category: MemoryCategory::ShortTerm,
                importance: 0.5 + (i as f64 * 0.1),
                access_count: 0,
                last_accessed_at: Utc::now(),
                created_at: Utc::now(),
                tags: vec![],
                related_messages: vec![],
                expires_at: None,
            };
            storage.store_memory(memory).await.unwrap();
        }

        // Search by company - use helper method
        let results = storage.search_memories(&MemorySearchQuery::default()).await.unwrap();
        assert!(results.len() >= 5);
    }

    #[tokio::test]
    async fn test_config_storage() {
        use clawlegion_core::ConfigStorage;

        let (_temp_dir, storage) = create_test_storage();

        // Test set and get config
        let value = serde_json::json!({"setting": "value"});
        storage.set_config("test_setting", value.clone()).await.unwrap();

        let retrieved = storage.get_config("test_setting").await.unwrap();
        assert_eq!(retrieved, Some(value));

        // Test list keys
        storage.set_config("another_setting", serde_json::json!("value")).await.unwrap();
        let keys = storage.list_config_keys().await.unwrap();
        assert!(keys.contains(&"test_setting".to_string()));
        assert!(keys.contains(&"another_setting".to_string()));

        // Test delete config
        assert!(storage.delete_config("test_setting").await.unwrap());
        assert!(storage.get_config("test_setting").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_config_file_operations() {
        use clawlegion_core::ConfigStorage;
        use std::path::PathBuf;

        let (temp_dir, storage) = create_test_storage();
        let config_file = temp_dir.path().join("config.toml");

        let mut config = HashMap::new();
        config.insert("key1".to_string(), serde_json::json!("value1"));
        config.insert("key2".to_string(), serde_json::json!("value2"));

        // Save to file
        storage.save_to_file(&config_file, &config).await.unwrap();
        assert!(config_file.exists());

        // Load from file
        let loaded = storage.load_from_file(&config_file).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get("key1"), config.get("key1"));
        assert_eq!(loaded.get("key2"), config.get("key2"));
    }
}
