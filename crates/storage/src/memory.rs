//! Memory management with Ebbinghaus forgetting curve

use chrono::Utc;
use clawlegion_core::{
    CompressedMemory, Error, MemoryCategory, MemoryEntry, MemorySearchQuery, Result, StorageError,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use uuid::Uuid;

/// Memory Manager
///
/// Manages memory entries with Ebbinghaus forgetting curve-based compression.
pub struct MemoryManager {
    /// All memory entries
    entries: dashmap::DashMap<Uuid, MemoryEntry>,

    /// Compressed memories
    compressed: parking_lot::RwLock<Vec<CompressedMemory>>,

    /// Retention curve parameters
    retention_params: RetentionParams,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new() -> Self {
        Self {
            entries: dashmap::DashMap::new(),
            compressed: parking_lot::RwLock::new(Vec::new()),
            retention_params: RetentionParams::default(),
        }
    }

    /// Create with custom retention parameters
    pub fn with_params(retention_params: RetentionParams) -> Self {
        Self {
            entries: dashmap::DashMap::new(),
            compressed: parking_lot::RwLock::new(Vec::new()),
            retention_params,
        }
    }

    /// Store a memory entry
    pub fn store(&self, mut entry: MemoryEntry) -> Uuid {
        let id = entry.id;
        entry.created_at = Utc::now();
        entry.last_accessed_at = entry.created_at;
        self.entries.insert(id, entry);
        id
    }

    /// Get a memory entry
    pub fn get(&self, id: Uuid) -> Option<MemoryEntry> {
        self.entries.get(&id).map(|entry| entry.clone())
    }

    /// Update a memory entry
    pub fn update(&self, id: Uuid, f: impl FnOnce(&mut MemoryEntry)) -> Result<()> {
        let mut entry = self
            .entries
            .get_mut(&id)
            .ok_or_else(|| Error::Storage(StorageError::NotFound(id.to_string())))?;

        f(&mut entry);
        Ok(())
    }

    /// Touch a memory (update last accessed time)
    pub fn touch(&self, id: Uuid) -> Result<()> {
        self.update(id, |entry| {
            entry.last_accessed_at = Utc::now();
            entry.access_count += 1;
        })
    }

    /// Delete a memory entry
    pub fn delete(&self, id: Uuid) -> bool {
        self.entries.remove(&id).is_some()
    }

    /// Search memories
    pub fn search(&self, query: &MemorySearchQuery) -> Vec<MemoryEntry> {
        let mut results: Vec<MemoryEntry> = self
            .entries
            .iter()
            .filter(|entry| self.matches_query(entry.value(), query))
            .map(|entry| entry.clone())
            .collect();

        // Sort by retention score (higher first)
        results.sort_by(|a, b| {
            let score_a = self.calculate_retention_score(a);
            let score_b = self.calculate_retention_score(b);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        results
    }

    /// Check if a memory matches a search query
    fn matches_query(&self, entry: &MemoryEntry, query: &MemorySearchQuery) -> bool {
        // Category filter
        if let Some(category) = query.category {
            if entry.category != category {
                return false;
            }
        }

        // Agent filter
        if let Some(agent_id) = query.agent_id {
            if entry.agent_id != Some(agent_id) {
                return false;
            }
        }

        // Company filter
        if let Some(company_id) = query.company_id {
            if entry.company_id != company_id {
                return false;
            }
        }

        // Time range filter
        if let Some((start, end)) = query.time_range {
            if entry.created_at < start || entry.created_at > end {
                return false;
            }
        }

        // Importance filter
        if let Some(min_importance) = query.min_importance {
            if entry.importance < min_importance {
                return false;
            }
        }

        // Tags filter
        if let Some(ref tags) = query.tags {
            if !tags.iter().any(|tag| entry.tags.contains(tag)) {
                return false;
            }
        }

        // Keywords filter
        if let Some(ref keywords) = query.keywords {
            let content_lower = entry.content.to_lowercase();
            if !keywords
                .iter()
                .any(|kw| content_lower.contains(&kw.to_lowercase()))
            {
                return false;
            }
        }

        true
    }

    /// Calculate retention score based on Ebbinghaus forgetting curve
    pub fn calculate_retention_score(&self, entry: &MemoryEntry) -> f64 {
        let hours_since_access = (entry
            .last_accessed_at
            .signed_duration_since(Utc::now())
            .num_hours() as f64)
            .abs();

        // Ebbinghaus forgetting curve: R = e^(-t/S)
        // Modified with memory strength factors
        let memory_strength = self.calculate_memory_strength(entry);
        let s = self.retention_params.strength_multiplier * memory_strength
            + self.retention_params.base_strength;

        let decay = (-hours_since_access / s).exp();

        // Apply custom decay factor
        decay * self.retention_params.custom_decay_factor
    }

    /// Calculate memory strength based on importance and access frequency
    fn calculate_memory_strength(&self, entry: &MemoryEntry) -> f64 {
        let importance_factor = entry.importance.max(0.1);
        let frequency_factor =
            1.0 + (entry.access_count as f64 * self.retention_params.frequency_weight);

        importance_factor * frequency_factor
    }

    /// Get memories that should be compressed
    pub fn get_memories_to_compress(&self) -> Vec<MemoryEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                let score = self.calculate_retention_score(entry.value());
                score < self.retention_params.compress_threshold
                    && entry.value().category == MemoryCategory::ShortTerm
            })
            .map(|entry| entry.clone())
            .collect()
    }

    /// Get memories that should be forgotten
    pub fn get_memories_to_forget(&self) -> Vec<MemoryEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                let score = self.calculate_retention_score(entry.value());
                score < self.retention_params.forget_threshold
                    && !entry.value().tags.iter().any(|t| t == "protected")
            })
            .map(|entry| entry.clone())
            .collect()
    }

    /// Get memories that have expired and should be deleted
    pub fn get_expired_memories(&self) -> Vec<MemoryEntry> {
        let now = Utc::now();
        self.entries
            .iter()
            .filter(|entry| entry.value().expires_at.is_some_and(|exp| exp < now))
            .map(|entry| entry.clone())
            .collect()
    }

    /// Delete expired memories and return their IDs
    pub fn delete_expired(&self) -> Vec<Uuid> {
        let expired = self.get_expired_memories();
        let ids: Vec<Uuid> = expired.iter().map(|e| e.id).collect();
        for entry in expired {
            self.entries.remove(&entry.id);
        }
        ids
    }

    /// Compress memories
    pub fn compress_memories(&self, entries: Vec<MemoryEntry>) -> CompressedMemory {
        let source_ids: Vec<Uuid> = entries.iter().map(|e| e.id).collect();

        let time_range = entries
            .iter()
            .map(|e| (e.created_at, e.last_accessed_at))
            .fold(
                (Utc::now(), Utc::now()),
                |(min_time, max_time), (created, accessed)| {
                    (
                        if created < min_time {
                            created
                        } else {
                            min_time
                        },
                        if accessed > max_time {
                            accessed
                        } else {
                            max_time
                        },
                    )
                },
            );

        // Create summary (in real implementation, this would use LLM)
        let summary = format!(
            "Compressed {} memories from {} to {}",
            entries.len(),
            time_range.0.format("%Y-%m-%d %H:%M"),
            time_range.1.format("%Y-%m-%d %H:%M")
        );

        // Extract key facts
        let key_facts: Vec<String> = entries
            .iter()
            .filter(|e| e.importance > 0.7)
            .map(|e| e.content.chars().take(100).collect())
            .collect();

        let compressed = CompressedMemory {
            source_ids,
            summary,
            key_facts,
            time_range,
            compressed_at: Utc::now(),
        };

        self.compressed.write().push(compressed.clone());

        // Remove original entries
        for entry in &entries {
            self.entries.remove(&entry.id);
        }

        compressed
    }

    /// Forget memories (permanently delete)
    pub fn forget_memories(&self, entries: Vec<MemoryEntry>) -> Vec<Uuid> {
        let mut deleted_ids = Vec::new();

        for entry in entries {
            if self.entries.remove(&entry.id).is_some() {
                deleted_ids.push(entry.id);
            }
        }

        deleted_ids
    }

    /// Get compressed memories
    pub fn get_compressed(&self) -> Vec<CompressedMemory> {
        self.compressed.read().clone()
    }

    /// Get all entries
    pub fn all_entries(&self) -> Vec<MemoryEntry> {
        self.entries.iter().map(|entry| entry.clone()).collect()
    }

    /// Get entry count
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Clear all memories (for testing)
    pub fn clear(&self) {
        self.entries.clear();
        self.compressed.write().clear();
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Retention curve parameters
#[derive(Debug, Clone)]
pub struct RetentionParams {
    /// Base strength constant
    pub base_strength: f64,

    /// Strength multiplier for memory importance
    pub strength_multiplier: f64,

    /// Weight for access frequency
    pub frequency_weight: f64,

    /// Custom decay factor
    pub custom_decay_factor: f64,

    /// Threshold below which memories should be compressed
    pub compress_threshold: f64,

    /// Threshold below which memories should be forgotten
    pub forget_threshold: f64,
}

impl Default for RetentionParams {
    fn default() -> Self {
        Self {
            base_strength: 1.0,
            strength_multiplier: 2.0,
            frequency_weight: 0.1,
            custom_decay_factor: 1.0,
            compress_threshold: 0.5, // Compress when retention drops below 50%
            forget_threshold: 0.1,   // Forget when retention drops below 10%
        }
    }
}

impl RetentionParams {
    /// Create parameters for aggressive compression (faster forgetting)
    pub fn aggressive() -> Self {
        Self {
            base_strength: 0.5,
            strength_multiplier: 1.5,
            frequency_weight: 0.05,
            custom_decay_factor: 0.8,
            compress_threshold: 0.7,
            forget_threshold: 0.2,
        }
    }

    /// Create parameters for conservative compression (slower forgetting)
    pub fn conservative() -> Self {
        Self {
            base_strength: 2.0,
            strength_multiplier: 3.0,
            frequency_weight: 0.15,
            custom_decay_factor: 1.2,
            compress_threshold: 0.3,
            forget_threshold: 0.05,
        }
    }
}

/// Background task for periodic memory cleanup
pub struct MemoryCleanupTask {
    running: Arc<AtomicBool>,
    shutdown_tx: parking_lot::RwLock<Option<tokio::sync::broadcast::Sender<()>>>,
}

impl MemoryCleanupTask {
    /// Create a new cleanup task
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: parking_lot::RwLock::new(None),
        }
    }

    /// Start the cleanup task
    pub fn start(&self, memory_manager: Arc<MemoryManager>, interval_secs: u64) {
        if self.running.load(std::sync::atomic::Ordering::Relaxed) {
            return; // Already running
        }

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);
        *self.shutdown_tx.write() = Some(shutdown_tx.clone());
        self.running
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let expired = memory_manager.delete_expired();
                        if !expired.is_empty() {
                            tracing::info!("Cleaned up {} expired memories", expired.len());
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        running.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                }
            }
        });
    }

    /// Stop the cleanup task
    pub fn stop(&self) {
        if let Some(tx) = self.shutdown_tx.read().as_ref() {
            let _ = tx.send(());
        }
    }

    /// Check if the cleanup task is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for MemoryCleanupTask {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use clawlegion_core::MemoryCategory;

    #[test]
    fn test_get_expired_memories() {
        let manager = MemoryManager::new();
        let now = Utc::now();

        // Create an expired memory
        let expired_entry = MemoryEntry {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            agent_id: None,
            content: "Expired memory".to_string(),
            category: MemoryCategory::ShortTerm,
            importance: 0.5,
            access_count: 0,
            last_accessed_at: now,
            created_at: now,
            tags: vec![],
            related_messages: vec![],
            expires_at: Some(now - Duration::hours(1)),
        };

        // Create a non-expired memory
        let valid_entry = MemoryEntry {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            agent_id: None,
            content: "Valid memory".to_string(),
            category: MemoryCategory::ShortTerm,
            importance: 0.5,
            access_count: 0,
            last_accessed_at: now,
            created_at: now,
            tags: vec![],
            related_messages: vec![],
            expires_at: Some(now + Duration::hours(1)),
        };

        // Create a memory without expiration
        let no_expiry_entry = MemoryEntry {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            agent_id: None,
            content: "No expiry memory".to_string(),
            category: MemoryCategory::LongTerm,
            importance: 0.5,
            access_count: 0,
            last_accessed_at: now,
            created_at: now,
            tags: vec![],
            related_messages: vec![],
            expires_at: None,
        };

        manager.store(expired_entry);
        manager.store(valid_entry);
        manager.store(no_expiry_entry);

        let expired = manager.get_expired_memories();

        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].content, "Expired memory");
    }

    #[test]
    fn test_delete_expired() {
        let manager = MemoryManager::new();
        let now = Utc::now();

        // Create expired memories
        for i in 0..3 {
            let entry = MemoryEntry {
                id: Uuid::new_v4(),
                company_id: Uuid::new_v4(),
                agent_id: None,
                content: format!("Expired memory {}", i),
                category: MemoryCategory::ShortTerm,
                importance: 0.5,
                access_count: 0,
                last_accessed_at: now,
                created_at: now,
                tags: vec![],
                related_messages: vec![],
                expires_at: Some(now - Duration::minutes(1)),
            };
            manager.store(entry);
        }

        // Create valid memories
        for i in 0..2 {
            let entry = MemoryEntry {
                id: Uuid::new_v4(),
                company_id: Uuid::new_v4(),
                agent_id: None,
                content: format!("Valid memory {}", i),
                category: MemoryCategory::ShortTerm,
                importance: 0.5,
                access_count: 0,
                last_accessed_at: now,
                created_at: now,
                tags: vec![],
                related_messages: vec![],
                expires_at: Some(now + Duration::hours(1)),
            };
            manager.store(entry);
        }

        let deleted_ids = manager.delete_expired();

        assert_eq!(deleted_ids.len(), 3);
        assert_eq!(manager.count(), 2); // Only valid memories remain
    }

    #[tokio::test]
    async fn test_memory_cleanup_task() {
        let manager = Arc::new(MemoryManager::new());
        let cleanup_task = MemoryCleanupTask::new();

        assert!(!cleanup_task.is_running());

        cleanup_task.start(manager.clone(), 60);

        // Give it a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert!(cleanup_task.is_running());

        cleanup_task.stop();

        // Give it a moment to stop
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert!(!cleanup_task.is_running());
    }
}
