//! Storage implementations

use async_trait::async_trait;
use clawlegion_core::{
    CompressedMemory, ConfigStorage, Error, MemoryEntry, MemorySearchQuery, Result, Storage,
    StorageCapabilities, StorageError, StoragePage,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::Path;

/// In-memory storage implementation
pub struct InMemoryStorage {
    /// Key-value store
    data: DashMap<String, serde_json::Value>,

    /// Memory manager
    memory_manager: crate::MemoryManager,

    /// Compressed memories
    compressed_memories: RwLock<Vec<CompressedMemory>>,

    /// Configuration store
    config: RwLock<HashMap<String, serde_json::Value>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
            memory_manager: crate::MemoryManager::new(),
            compressed_memories: RwLock::new(Vec::new()),
            config: RwLock::new(HashMap::new()),
        }
    }

    /// Get the memory manager
    pub fn memory_manager(&self) -> &crate::MemoryManager {
        &self.memory_manager
    }

    /// Run memory compression
    pub fn run_compression(&self) -> Result<Vec<CompressedMemory>> {
        let to_compress = self.memory_manager.get_memories_to_compress();

        if to_compress.is_empty() {
            return Ok(vec![]);
        }

        let compressor = crate::MemoryCompressor::new(crate::CompressionStrategy::Simple);

        // For simplicity, compress all into one group
        // In production, you might want to group by category or time range
        let compressed = futures_executor::block_on(compressor.compress(to_compress))?;

        self.compressed_memories.write().push(compressed.clone());

        Ok(vec![compressed])
    }

    /// Run memory forgetting
    pub fn run_forgetting(&self) -> Result<Vec<uuid::Uuid>> {
        let to_forget = self.memory_manager.get_memories_to_forget();
        let ids: Vec<uuid::Uuid> = to_forget.iter().map(|e| e.id).collect();

        self.memory_manager.forget_memories(to_forget);

        Ok(ids)
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        Ok(self.data.get(key).map(|entry| entry.clone()))
    }

    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.data.insert(key.to_string(), value);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        Ok(self.data.remove(key).is_some())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.data.contains_key(key))
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        Ok(self
            .data
            .iter()
            .filter(|entry| entry.key().starts_with(prefix))
            .map(|entry| entry.key().clone())
            .collect())
    }

    async fn list_keys_paginated(
        &self,
        prefix: &str,
        offset: usize,
        limit: usize,
    ) -> Result<StoragePage<String>> {
        let mut keys = self.list_keys(prefix).await?;
        keys.sort();
        let items = keys
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        let next_offset = if items.len() == limit {
            Some(offset + items.len())
        } else {
            None
        };
        Ok(StoragePage { items, next_offset })
    }

    async fn get_many(
        &self,
        keys: &[String],
    ) -> Result<HashMap<String, Option<serde_json::Value>>> {
        Ok(keys
            .iter()
            .map(|key| (key.clone(), self.data.get(key).map(|entry| entry.clone())))
            .collect())
    }

    async fn set_many(&self, entries: HashMap<String, serde_json::Value>) -> Result<()> {
        for (key, value) in entries {
            self.data.insert(key, value);
        }
        Ok(())
    }

    async fn delete_many(&self, keys: &[String]) -> Result<usize> {
        let mut deleted = 0;
        for key in keys {
            if self.data.remove(key).is_some() {
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    async fn store_memory(&self, memory: MemoryEntry) -> Result<uuid::Uuid> {
        let id = self.memory_manager.store(memory);
        Ok(id)
    }

    async fn get_memory(&self, id: uuid::Uuid) -> Result<Option<MemoryEntry>> {
        Ok(self.memory_manager.get(id))
    }

    async fn search_memories(&self, query: &MemorySearchQuery) -> Result<Vec<MemoryEntry>> {
        Ok(self.memory_manager.search(query))
    }

    async fn search_memories_paginated(
        &self,
        query: &MemorySearchQuery,
        offset: usize,
        limit: usize,
    ) -> Result<StoragePage<MemoryEntry>> {
        let memories = self.memory_manager.search(query);
        let items = memories
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        let next_offset = if items.len() == limit {
            Some(offset + items.len())
        } else {
            None
        };

        Ok(StoragePage { items, next_offset })
    }

    async fn touch_memory(&self, id: uuid::Uuid) -> Result<()> {
        self.memory_manager.touch(id)
    }

    async fn compress_memories(&self) -> Result<Vec<CompressedMemory>> {
        self.run_compression()
    }

    async fn forget_memories(&self) -> Result<Vec<uuid::Uuid>> {
        self.run_forgetting()
    }

    async fn store_compressed(&self, memory: CompressedMemory) -> Result<uuid::Uuid> {
        self.compressed_memories.write().push(memory);
        Ok(uuid::Uuid::new_v4())
    }

    async fn get_compressed_memories(&self) -> Result<Vec<CompressedMemory>> {
        Ok(self.compressed_memories.read().clone())
    }

    async fn get_expired_memories(&self) -> Result<Vec<MemoryEntry>> {
        Ok(self.memory_manager.get_expired_memories())
    }

    async fn delete_expired_memories(&self) -> Result<Vec<uuid::Uuid>> {
        Ok(self.memory_manager.delete_expired())
    }

    async fn shutdown(&self) -> Result<()> {
        // Nothing to do for in-memory storage
        Ok(())
    }

    fn capabilities(&self) -> StorageCapabilities {
        StorageCapabilities {
            supports_batch_kv: true,
            supports_pagination: true,
            supports_transactions: false,
            supports_memory_filters: true,
        }
    }
}

#[async_trait]
impl ConfigStorage for InMemoryStorage {
    async fn get_config(&self, key: &str) -> Result<Option<serde_json::Value>> {
        Ok(self.config.read().get(key).cloned())
    }

    async fn set_config(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.config.write().insert(key.to_string(), value);
        Ok(())
    }

    async fn delete_config(&self, key: &str) -> Result<bool> {
        Ok(self.config.write().remove(key).is_some())
    }

    async fn list_config_keys(&self) -> Result<Vec<String>> {
        Ok(self.config.read().keys().cloned().collect())
    }

    async fn load_from_file(&self, path: &Path) -> Result<HashMap<String, serde_json::Value>> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "Failed to read config file: {}",
                e
            )))
        })?;

        let config: HashMap<String, serde_json::Value> = toml::from_str(&content).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "Failed to parse config file: {}",
                e
            )))
        })?;

        // Merge with existing config
        for (key, value) in &config {
            self.config.write().insert(key.clone(), value.clone());
        }

        Ok(config)
    }

    async fn save_to_file(
        &self,
        path: &Path,
        config: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let content = toml::to_string_pretty(config).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "Failed to serialize config: {}",
                e
            )))
        })?;

        std::fs::write(path, content).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "Failed to write config file: {}",
                e
            )))
        })?;

        Ok(())
    }
}

/// File-based storage implementation
pub struct FileStorage {
    /// Base directory for storage
    base_dir: std::path::PathBuf,

    /// In-memory cache
    cache: InMemoryStorage,

    /// Auto-sync to disk
    auto_sync: bool,
}

impl FileStorage {
    /// Create a new file-based storage
    pub fn new(base_dir: std::path::PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_dir).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "Failed to create storage directory: {}",
                e
            )))
        })?;

        Ok(Self {
            base_dir,
            cache: InMemoryStorage::new(),
            auto_sync: true,
        })
    }

    /// Set auto-sync mode
    pub fn with_auto_sync(mut self, enabled: bool) -> Self {
        self.auto_sync = enabled;
        self
    }

    /// Get file path for a key
    fn key_to_path(&self, key: &str) -> std::path::PathBuf {
        // Replace invalid filename characters
        let safe_key = key.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        self.base_dir.join(format!("{}.json", safe_key))
    }

    /// Sync data to disk
    pub fn sync_to_disk(&self) -> Result<()> {
        for entry in self.cache.data.iter() {
            let path = self.key_to_path(entry.key());
            let content = serde_json::to_string_pretty(entry.value()).map_err(|e| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "Failed to serialize data: {}",
                    e
                )))
            })?;

            std::fs::write(&path, content).map_err(|e| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "Failed to write file: {}",
                    e
                )))
            })?;
        }

        Ok(())
    }

    /// Load data from disk
    pub fn load_from_disk(&self) -> Result<()> {
        for entry in std::fs::read_dir(&self.base_dir).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "Failed to read storage directory: {}",
                e
            )))
        })? {
            let entry = entry.map_err(|e| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "Failed to read directory entry: {}",
                    e
                )))
            })?;

            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = std::fs::read_to_string(&path).map_err(|e| {
                    Error::Storage(StorageError::OperationFailed(format!(
                        "Failed to read file: {}",
                        e
                    )))
                })?;

                let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                    Error::Storage(StorageError::OperationFailed(format!(
                        "Failed to parse file: {}",
                        e
                    )))
                })?;

                // Extract key from filename
                if let Some(key) = path.file_stem().and_then(|s| s.to_str()) {
                    self.cache.data.insert(key.to_string(), value);
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        self.cache.get(key).await
    }

    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.cache.set(key, value).await?;

        if self.auto_sync {
            self.sync_to_disk()?;
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let result = self.cache.delete(key).await?;

        if self.auto_sync {
            let path = self.key_to_path(key);
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| {
                    Error::Storage(StorageError::OperationFailed(format!(
                        "Failed to delete file: {}",
                        e
                    )))
                })?;
            }
        }

        Ok(result)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.cache.exists(key).await
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        self.cache.list_keys(prefix).await
    }

    async fn store_memory(&self, memory: MemoryEntry) -> Result<uuid::Uuid> {
        self.cache.store_memory(memory).await
    }

    async fn get_memory(&self, id: uuid::Uuid) -> Result<Option<MemoryEntry>> {
        self.cache.get_memory(id).await
    }

    async fn search_memories(&self, query: &MemorySearchQuery) -> Result<Vec<MemoryEntry>> {
        self.cache.search_memories(query).await
    }

    async fn touch_memory(&self, id: uuid::Uuid) -> Result<()> {
        self.cache.touch_memory(id).await
    }

    async fn compress_memories(&self) -> Result<Vec<CompressedMemory>> {
        self.cache.compress_memories().await
    }

    async fn forget_memories(&self) -> Result<Vec<uuid::Uuid>> {
        self.cache.forget_memories().await
    }

    async fn store_compressed(&self, memory: CompressedMemory) -> Result<uuid::Uuid> {
        self.cache.store_compressed(memory).await
    }

    async fn get_compressed_memories(&self) -> Result<Vec<CompressedMemory>> {
        self.cache.get_compressed_memories().await
    }

    async fn get_expired_memories(&self) -> Result<Vec<MemoryEntry>> {
        self.cache.get_expired_memories().await
    }

    async fn delete_expired_memories(&self) -> Result<Vec<uuid::Uuid>> {
        self.cache.delete_expired_memories().await
    }

    async fn shutdown(&self) -> Result<()> {
        self.sync_to_disk()
    }
}

#[async_trait]
impl ConfigStorage for FileStorage {
    async fn get_config(&self, key: &str) -> Result<Option<serde_json::Value>> {
        self.cache.get_config(key).await
    }

    async fn set_config(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.cache.set_config(key, value).await
    }

    async fn delete_config(&self, key: &str) -> Result<bool> {
        self.cache.delete_config(key).await
    }

    async fn list_config_keys(&self) -> Result<Vec<String>> {
        self.cache.list_config_keys().await
    }

    async fn load_from_file(&self, path: &Path) -> Result<HashMap<String, serde_json::Value>> {
        self.cache.load_from_file(path).await
    }

    async fn save_to_file(
        &self,
        path: &Path,
        config: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        self.cache.save_to_file(path, config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawlegion_core::{MemoryCategory, MemorySearchQuery};

    #[tokio::test]
    async fn in_memory_storage_supports_batch_kv_and_pagination() {
        let storage = InMemoryStorage::new();
        storage
            .set_many(HashMap::from([
                ("alpha".to_string(), serde_json::json!(1)),
                ("beta".to_string(), serde_json::json!(2)),
                ("prefix:one".to_string(), serde_json::json!(3)),
                ("prefix:two".to_string(), serde_json::json!(4)),
            ]))
            .await
            .expect("set many");

        let fetched = storage
            .get_many(&[
                "alpha".to_string(),
                "prefix:one".to_string(),
                "missing".to_string(),
            ])
            .await
            .expect("get many");
        assert_eq!(fetched["alpha"], Some(serde_json::json!(1)));
        assert_eq!(fetched["missing"], None);

        let page = storage
            .list_keys_paginated("prefix:", 0, 1)
            .await
            .expect("page");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.next_offset, Some(1));

        let deleted = storage
            .delete_many(&["alpha".to_string(), "missing".to_string()])
            .await
            .expect("delete many");
        assert_eq!(deleted, 1);
        assert!(!storage.exists("alpha").await.expect("exists"));
        assert!(storage.capabilities().supports_batch_kv);
        assert!(storage.capabilities().supports_pagination);
    }

    #[tokio::test]
    async fn paginates_memory_search_results() {
        let storage = InMemoryStorage::new();

        for idx in 0..3 {
            storage
                .store_memory(MemoryEntry {
                    id: uuid::Uuid::new_v4(),
                    company_id: uuid::Uuid::new_v4(),
                    agent_id: None,
                    content: format!("memory-{idx}"),
                    category: MemoryCategory::ShortTerm,
                    importance: 0.8,
                    access_count: 0,
                    created_at: chrono::Utc::now(),
                    last_accessed_at: chrono::Utc::now(),
                    tags: vec!["test".to_string()],
                    related_messages: vec![],
                    expires_at: None,
                })
                .await
                .expect("store memory");
        }

        let page = storage
            .search_memories_paginated(
                &MemorySearchQuery {
                    category: Some(MemoryCategory::ShortTerm),
                    ..MemorySearchQuery::default()
                },
                0,
                2,
            )
            .await
            .expect("search page");

        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next_offset, Some(2));
    }
}
