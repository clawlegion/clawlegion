//! Storage system core traits

use crate::{AgentId, MessageId, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Storage key type
pub type StorageKey = String;

/// Memory entry with timestamp and importance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique memory ID
    pub id: Uuid,

    /// Company ID this memory belongs to
    pub company_id: Uuid,

    /// Optional agent ID this memory is associated with
    pub agent_id: Option<AgentId>,

    /// Memory content
    pub content: String,

    /// Memory category
    pub category: MemoryCategory,

    /// Importance score (0.0 - 1.0)
    pub importance: f64,

    /// Access count (for frequency-based retention)
    pub access_count: u64,

    /// Last access time
    pub last_accessed_at: DateTime<Utc>,

    /// Creation time
    pub created_at: DateTime<Utc>,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Related message IDs
    pub related_messages: Vec<MessageId>,

    /// Optional expiration time. Inherits from MessageMetadata.expires_at
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

/// Memory category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    /// Short-term memory (recent conversations)
    ShortTerm,

    /// Long-term memory (important facts)
    LongTerm,

    /// Procedural memory (learned skills)
    Procedural,

    /// Semantic memory (general knowledge)
    Semantic,

    /// Episodic memory (specific events)
    Episodic,
}

impl MemoryEntry {
    /// Calculate retention score based on Ebbinghaus forgetting curve
    pub fn retention_score(&self) -> f64 {
        // Calculate hours since last access (positive value for past times)
        let hours_since_access = Utc::now()
            .signed_duration_since(self.last_accessed_at)
            .num_hours() as f64;

        // Ebbinghaus forgetting curve: R = e^(-t/S)
        // Where S is a constant based on memory strength
        let memory_strength = self.importance * (1.0 + self.access_count as f64 * 0.1);
        let s = 1.0 / (memory_strength + 0.01); // Avoid division by zero

        (-hours_since_access * s).exp()
    }

    /// Determine if this memory should be compressed or forgotten
    pub fn should_compress(&self) -> bool {
        let retention = self.retention_score();
        retention < 0.5 && self.category == MemoryCategory::ShortTerm
    }

    pub fn should_forget(&self) -> bool {
        let retention = self.retention_score();
        retention < 0.1 && !self.tags.iter().any(|t| t == "protected")
    }

    /// Check if this memory has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| exp < Utc::now())
    }

    /// Get category as string
    pub fn category_str(&self) -> &'static str {
        self.category.as_str()
    }
}

impl MemoryCategory {
    /// Convert category to string
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryCategory::ShortTerm => "short_term",
            MemoryCategory::LongTerm => "long_term",
            MemoryCategory::Procedural => "procedural",
            MemoryCategory::Semantic => "semantic",
            MemoryCategory::Episodic => "episodic",
        }
    }
}

/// Compressed memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedMemory {
    /// Original memory IDs that were compressed
    pub source_ids: Vec<Uuid>,

    /// Compressed summary
    pub summary: String,

    /// Key information extracted
    pub key_facts: Vec<String>,

    /// Time range covered
    pub time_range: (DateTime<Utc>, DateTime<Utc>),

    /// Compression timestamp
    pub compressed_at: DateTime<Utc>,
}

/// Search query for memory retrieval
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemorySearchQuery {
    /// Keywords to search
    pub keywords: Option<Vec<String>>,

    /// Category filter
    pub category: Option<MemoryCategory>,

    /// Agent filter
    pub agent_id: Option<AgentId>,

    /// Company filter
    pub company_id: Option<Uuid>,

    /// Time range filter
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,

    /// Minimum importance score
    pub min_importance: Option<f64>,

    /// Tags to filter by
    pub tags: Option<Vec<String>>,

    /// Maximum results
    pub limit: Option<usize>,
}

/// Generic paginated result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePage<T> {
    pub items: Vec<T>,
    pub next_offset: Option<usize>,
}

/// Storage backend capability flags
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StorageCapabilities {
    pub supports_batch_kv: bool,
    pub supports_pagination: bool,
    pub supports_transactions: bool,
    pub supports_memory_filters: bool,
}

impl Default for StorageCapabilities {
    fn default() -> Self {
        Self {
            supports_batch_kv: false,
            supports_pagination: false,
            supports_transactions: false,
            supports_memory_filters: true,
        }
    }
}

/// Storage trait for pluggable storage implementations
#[async_trait]
pub trait Storage: Send + Sync {
    /// Get a value by key
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>>;

    /// Set a value
    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()>;

    /// Delete a value
    async fn delete(&self, key: &str) -> Result<bool>;

    /// Check if a key exists
    async fn exists(&self, key: &str) -> Result<bool>;

    /// List all keys with a prefix
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;

    /// List keys with offset pagination
    async fn list_keys_paginated(
        &self,
        prefix: &str,
        offset: usize,
        limit: usize,
    ) -> Result<StoragePage<String>> {
        let keys = self.list_keys(prefix).await?;
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

    /// Fetch multiple keys in one call
    async fn get_many(
        &self,
        keys: &[String],
    ) -> Result<HashMap<String, Option<serde_json::Value>>> {
        let mut values = HashMap::with_capacity(keys.len());
        for key in keys {
            values.insert(key.clone(), self.get(key).await?);
        }
        Ok(values)
    }

    /// Persist multiple key-value pairs
    async fn set_many(&self, entries: HashMap<String, serde_json::Value>) -> Result<()> {
        for (key, value) in entries {
            self.set(&key, value).await?;
        }
        Ok(())
    }

    /// Delete multiple keys and return the count of removed keys
    async fn delete_many(&self, keys: &[String]) -> Result<usize> {
        let mut deleted = 0;
        for key in keys {
            if self.delete(key).await? {
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    /// Store a memory entry
    async fn store_memory(&self, memory: MemoryEntry) -> Result<Uuid>;

    /// Get a memory entry by ID
    async fn get_memory(&self, id: Uuid) -> Result<Option<MemoryEntry>>;

    /// Search memories
    async fn search_memories(&self, query: &MemorySearchQuery) -> Result<Vec<MemoryEntry>>;

    /// Search memories with offset pagination
    async fn search_memories_paginated(
        &self,
        query: &MemorySearchQuery,
        offset: usize,
        limit: usize,
    ) -> Result<StoragePage<MemoryEntry>> {
        let memories = self.search_memories(query).await?;
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

    /// Update memory access time (called when memory is accessed)
    async fn touch_memory(&self, id: Uuid) -> Result<()>;

    /// Compress memories based on Ebbinghaus curve
    async fn compress_memories(&self) -> Result<Vec<CompressedMemory>>;

    /// Forget memories that have exceeded retention threshold
    async fn forget_memories(&self) -> Result<Vec<Uuid>>;

    /// Store a compressed memory
    async fn store_compressed(&self, memory: CompressedMemory) -> Result<Uuid>;

    /// Get compressed memories
    async fn get_compressed_memories(&self) -> Result<Vec<CompressedMemory>>;

    /// Get expired memories
    async fn get_expired_memories(&self) -> Result<Vec<MemoryEntry>>;

    /// Delete expired memories and return their IDs
    async fn delete_expired_memories(&self) -> Result<Vec<Uuid>>;

    /// Shutdown storage gracefully
    async fn shutdown(&self) -> Result<()>;

    /// Report storage backend capabilities
    fn capabilities(&self) -> StorageCapabilities {
        StorageCapabilities::default()
    }
}

/// Configuration storage trait
#[async_trait]
pub trait ConfigStorage: Send + Sync {
    /// Get configuration value (returns None if not found)
    async fn get_config(&self, key: &str) -> Result<Option<serde_json::Value>>;

    /// Set configuration value
    async fn set_config(&self, key: &str, value: serde_json::Value) -> Result<()>;

    /// Delete configuration value
    async fn delete_config(&self, key: &str) -> Result<bool>;

    /// Get all configuration keys
    async fn list_config_keys(&self) -> Result<Vec<String>>;

    /// Load configuration from file
    async fn load_from_file(
        &self,
        path: &std::path::Path,
    ) -> Result<HashMap<String, serde_json::Value>>;

    /// Save configuration to file
    async fn save_to_file(
        &self,
        path: &std::path::Path,
        config: &HashMap<String, serde_json::Value>,
    ) -> Result<()>;
}
