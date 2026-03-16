//! Memory Compressor - handles memory compression using Ebbinghaus curve

use chrono::{DateTime, Utc};
use clawlegion_core::{CompressedMemory, MemoryEntry, Result};
use std::sync::Arc;
use uuid::Uuid;

/// Memory Compressor
///
/// Compresses memories based on the Ebbinghaus forgetting curve.
/// Can use either simple summarization or an LLM-based approach.
pub struct MemoryCompressor {
    /// Compression strategy
    strategy: CompressionStrategy,

    /// Optional LLM client for intelligent compression
    llm_client: Option<Arc<dyn LlmCompressionClient>>,
}

impl MemoryCompressor {
    /// Create a new compressor with a specific strategy
    pub fn new(strategy: CompressionStrategy) -> Self {
        Self {
            strategy,
            llm_client: None,
        }
    }

    /// Set LLM client for intelligent compression
    pub fn with_llm_client(mut self, client: Arc<dyn LlmCompressionClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    /// Compress a list of memory entries
    pub async fn compress(&self, entries: Vec<MemoryEntry>) -> Result<CompressedMemory> {
        if entries.is_empty() {
            return Err(clawlegion_core::Error::Storage(
                clawlegion_core::StorageError::CompressionFailed(
                    "No entries to compress".to_string(),
                ),
            ));
        }

        match self.strategy {
            CompressionStrategy::Simple => self.compress_simple(&entries),
            CompressionStrategy::Intelligent => {
                if let Some(ref llm) = self.llm_client {
                    self.compress_intelligent(llm.as_ref(), &entries).await
                } else {
                    // Fallback to simple compression if no LLM client
                    self.compress_simple(&entries)
                }
            }
            CompressionStrategy::Hybrid => {
                // Use simple compression for small batches, LLM for large ones
                if entries.len() <= 5 || self.llm_client.is_none() {
                    self.compress_simple(&entries)
                } else if let Some(ref llm) = self.llm_client {
                    self.compress_intelligent(llm.as_ref(), &entries).await
                } else {
                    self.compress_simple(&entries)
                }
            }
        }
    }

    /// Simple compression - concatenates and truncates
    fn compress_simple(&self, entries: &[MemoryEntry]) -> Result<CompressedMemory> {
        let source_ids: Vec<Uuid> = entries.iter().map(|e| e.id).collect();

        let time_range = Self::calculate_time_range(entries);

        // Create a simple summary
        let summary = format!(
            "Compressed {} memory entries from {} to {}. Categories: {:?}",
            entries.len(),
            time_range.0.format("%Y-%m-%d %H:%M:%S"),
            time_range.1.format("%Y-%m-%d %H:%M:%S"),
            entries
                .iter()
                .map(|e| e.category)
                .collect::<std::collections::HashSet<_>>()
        );

        // Extract key facts (high importance entries)
        let key_facts: Vec<String> = entries
            .iter()
            .filter(|e| e.importance > 0.7)
            .map(|e| {
                // Truncate long content
                if e.content.len() > 200 {
                    format!("{}...", &e.content.chars().take(200).collect::<String>())
                } else {
                    e.content.clone()
                }
            })
            .collect();

        Ok(CompressedMemory {
            source_ids,
            summary,
            key_facts,
            time_range,
            compressed_at: Utc::now(),
        })
    }

    /// Intelligent compression using LLM
    async fn compress_intelligent(
        &self,
        llm: &dyn LlmCompressionClient,
        entries: &[MemoryEntry],
    ) -> Result<CompressedMemory> {
        let source_ids: Vec<Uuid> = entries.iter().map(|e| e.id).collect();
        let time_range = Self::calculate_time_range(entries);

        // Prepare content for LLM
        let content: Vec<String> = entries
            .iter()
            .map(|e| format!("[{}: {:.2}] {}", e.category_str(), e.importance, e.content))
            .collect();

        let combined_content = content.join("\n---\n");

        // Request compression from LLM
        let compression_result = llm.compress_memories(&combined_content).await?;

        // Parse the result
        let (summary, key_facts) = Self::parse_llm_compression_result(&compression_result);

        Ok(CompressedMemory {
            source_ids,
            summary,
            key_facts,
            time_range,
            compressed_at: Utc::now(),
        })
    }

    /// Calculate time range from entries
    fn calculate_time_range(entries: &[MemoryEntry]) -> (DateTime<Utc>, DateTime<Utc>) {
        entries
            .iter()
            .fold((Utc::now(), Utc::now()), |(min_time, max_time), entry| {
                (
                    if entry.created_at < min_time {
                        entry.created_at
                    } else {
                        min_time
                    },
                    if entry.last_accessed_at > max_time {
                        entry.last_accessed_at
                    } else {
                        max_time
                    },
                )
            })
    }

    /// Parse LLM compression result
    fn parse_llm_compression_result(result: &str) -> (String, Vec<String>) {
        // Simple parsing - in production, use structured output
        let lines: Vec<&str> = result.lines().collect();

        let mut summary = String::new();
        let mut key_facts = Vec::new();
        let mut in_summary = true;

        for line in lines {
            if line.starts_with("Key Facts:") || line.starts_with("## Key Facts") {
                in_summary = false;
                continue;
            }

            if in_summary {
                if !summary.is_empty() {
                    summary.push(' ');
                }
                summary.push_str(line.trim());
            } else if line.trim().starts_with('-') || line.trim().starts_with('*') {
                key_facts.push(
                    line.trim()
                        .trim_start_matches(['-', '*'])
                        .trim()
                        .to_string(),
                );
            }
        }

        if key_facts.is_empty() {
            // If no key facts were parsed, return the whole result as summary
            return (result.to_string(), vec![]);
        }

        (summary, key_facts)
    }

    /// Get the compression strategy
    pub fn strategy(&self) -> &CompressionStrategy {
        &self.strategy
    }
}

/// Compression strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionStrategy {
    /// Simple concatenation and truncation
    Simple,

    /// LLM-based intelligent compression
    Intelligent,

    /// Hybrid - uses simple for small batches, LLM for large
    Hybrid,
}

/// Trait for LLM-based compression clients
#[async_trait::async_trait]
pub trait LlmCompressionClient: Send + Sync {
    /// Compress memories and return a summary
    async fn compress_memories(&self, content: &str) -> Result<String>;

    /// Extract key facts from content
    async fn extract_key_facts(&self, content: &str) -> Result<Vec<String>>;

    /// Generate a summary title for compressed memories
    async fn generate_summary_title(&self, content: &str) -> Result<String>;
}
