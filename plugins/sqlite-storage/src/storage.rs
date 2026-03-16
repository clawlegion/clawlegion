//! SQLite storage implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use clawlegion_core::{
    CompressedMemory, ConfigStorage, Error, MemoryCategory, MemoryEntry, MemorySearchQuery, Result,
    Storage, StorageCapabilities, StorageError, StoragePage,
};
use parking_lot::Mutex;
use rusqlite::{params, Connection, ToSql, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, TrySendError};
use std::thread::{self, JoinHandle};
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use crate::config::SqliteStorageConfig;
use crate::schema;

type DbTask = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

struct DbWorker {
    sender: Mutex<Option<mpsc::SyncSender<DbTask>>>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

impl DbWorker {
    fn new(conn: Connection, queue_capacity: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel::<DbTask>(queue_capacity);
        let join_handle = thread::Builder::new()
            .name("sqlite-storage-worker".to_string())
            .spawn(move || {
                let mut conn = conn;
                while let Ok(task) = rx.recv() {
                    task(&mut conn);
                }
            })
            .expect("failed to spawn sqlite-storage worker thread");

        Self {
            sender: Mutex::new(Some(tx)),
            join_handle: Mutex::new(Some(join_handle)),
        }
    }

    fn submit(&self, task: DbTask) -> Result<()> {
        let sender = self.sender.lock().clone().ok_or_else(|| {
            Error::Storage(StorageError::OperationFailed(
                "SQLite storage worker has been shut down".to_string(),
            ))
        })?;

        sender.try_send(task).map_err(|err| match err {
            TrySendError::Full(_) => Error::Storage(StorageError::OperationFailed(
                "SQLite storage worker queue is full".to_string(),
            )),
            TrySendError::Disconnected(_) => Error::Storage(StorageError::OperationFailed(
                "Failed to submit SQLite operation to worker".to_string(),
            )),
        })
    }

    fn shutdown(&self) -> Result<()> {
        self.sender.lock().take();

        if let Some(handle) = self.join_handle.lock().take() {
            handle.join().map_err(|_| {
                Error::Storage(StorageError::OperationFailed(
                    "SQLite storage worker thread panicked".to_string(),
                ))
            })?;
        }

        Ok(())
    }
}

impl Drop for DbWorker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// SQLite storage implementation
pub struct SqliteStorage {
    /// Dedicated database worker
    worker: DbWorker,
    /// Timeout for a single database operation
    operation_timeout: Duration,
    /// Configuration
    _config: SqliteStorageConfig,
}

impl SqliteStorage {
    /// Create a new SQLite storage
    pub fn new(config: SqliteStorageConfig) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = config.database_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "Failed to create database directory: {}",
                    e
                )))
            })?;
        }

        let db_path = config.database_path.to_string_lossy().to_string();
        let conn = Connection::open(&db_path).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "Failed to open database: {}",
                e
            )))
        })?;

        // Configure pragmas
        schema::configure_pragmas(&conn, config.wal_mode, config.cache_size).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "Failed to configure SQLite: {}",
                e
            )))
        })?;

        // Initialize schema
        if config.create_tables {
            schema::init_schema(&conn, true).map_err(|e| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "Failed to initialize schema: {}",
                    e
                )))
            })?;
        }

        Ok(Self {
            worker: DbWorker::new(conn, config.worker_queue_capacity),
            operation_timeout: Duration::from_millis(config.operation_timeout_ms),
            _config: config,
        })
    }

    async fn run_db<R, F>(&self, operation: F) -> Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<R> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.worker.submit(Box::new(move |conn| {
            let _ = tx.send(operation(conn));
        }))?;

        timeout(self.operation_timeout, rx)
            .await
            .map_err(|_| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "SQLite operation timed out after {} ms",
                    self.operation_timeout.as_millis()
                )))
            })?
            .map_err(|_| {
                Error::Storage(StorageError::OperationFailed(
                    "SQLite storage worker dropped response channel".to_string(),
                ))
            })?
    }

    async fn run_tx<R, F>(&self, operation: F) -> Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&rusqlite::Transaction<'_>) -> Result<R> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.worker.submit(Box::new(move |conn| {
            let result = (|| {
                let tx_handle = conn.transaction().map_err(|e| {
                    Error::Storage(StorageError::OperationFailed(format!(
                        "Transaction failed: {}",
                        e
                    )))
                })?;
                let result = operation(&tx_handle)?;
                tx_handle.commit().map_err(|e| {
                    Error::Storage(StorageError::OperationFailed(format!(
                        "Commit failed: {}",
                        e
                    )))
                })?;
                Ok(result)
            })();
            let _ = tx.send(result);
        }))?;

        timeout(self.operation_timeout, rx)
            .await
            .map_err(|_| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "SQLite transaction timed out after {} ms",
                    self.operation_timeout.as_millis()
                )))
            })?
            .map_err(|_| {
                Error::Storage(StorageError::OperationFailed(
                    "SQLite storage worker dropped transaction response channel".to_string(),
                ))
            })?
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let key = key.to_string();
        self.run_db(move |conn| {
            let mut stmt = conn
                .prepare_cached("SELECT value FROM storage WHERE key = ?1")
                .map_err(|e| StorageError::OperationFailed(format!("Failed to prepare statement: {}", e)))?;

            let result: Option<String> = stmt
                .query_row(params![key], |row| row.get(0))
                .optional()
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?;

            match result {
                Some(json_str) => {
                    let value: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
                        StorageError::OperationFailed(format!(
                            "Failed to parse JSON value: {}",
                            e
                        ))
                    })?;
                    Ok(Some(value))
                }
                None => Ok(None),
            }
        })
        .await
    }

    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let key = key.to_string();
        let json_str = serde_json::to_string(&value).map_err(|e| {
            StorageError::OperationFailed(format!("Failed to serialize value: {}", e))
        })?;

        self.run_db(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO storage (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
                params![key, json_str],
            )
            .map_err(|e| StorageError::OperationFailed(format!("Failed to insert: {}", e)))?;

            Ok(())
        })
        .await
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let key = key.to_string();
        self.run_db(move |conn| {
            let rows = conn
                .execute("DELETE FROM storage WHERE key = ?1", params![key])
                .map_err(|e| StorageError::OperationFailed(format!("Delete failed: {}", e)))?;

            Ok(rows > 0)
        })
        .await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let key = key.to_string();
        self.run_db(move |conn| {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM storage WHERE key = ?1)",
                    params![key],
                    |row| row.get(0),
                )
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?;

            Ok(exists)
        })
        .await
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let prefix = format!("{}%", prefix);
        self.run_db(move |conn| {
            let mut stmt = conn
                .prepare("SELECT key FROM storage WHERE key LIKE ?1 ORDER BY key")
                .map_err(|e| StorageError::OperationFailed(format!("Failed to prepare statement: {}", e)))?;

            let keys: Vec<String> = stmt
                .query_map(params![prefix], |row| row.get(0))
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(keys)
        })
        .await
    }

    async fn list_keys_paginated(
        &self,
        prefix: &str,
        offset: usize,
        limit: usize,
    ) -> Result<StoragePage<String>> {
        let prefix = format!("{}%", prefix);
        self.run_db(move |conn| {
            let mut stmt = conn
                .prepare("SELECT key FROM storage WHERE key LIKE ?1 ORDER BY key LIMIT ?2 OFFSET ?3")
                .map_err(|e| StorageError::OperationFailed(format!("Failed to prepare statement: {}", e)))?;

            let items = stmt
                .query_map(params![prefix, limit as i64, offset as i64], |row| row.get(0))
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?
                .collect::<std::result::Result<Vec<String>, _>>()
                .map_err(|e| StorageError::OperationFailed(format!("Collect failed: {}", e)))?;

            let next_offset = if items.len() == limit {
                Some(offset + items.len())
            } else {
                None
            };

            Ok(StoragePage { items, next_offset })
        })
        .await
    }

    async fn get_many(
        &self,
        keys: &[String],
    ) -> Result<HashMap<String, Option<serde_json::Value>>> {
        let keys = keys.to_vec();
        self.run_db(move |conn| {
            let mut stmt = conn
                .prepare("SELECT value FROM storage WHERE key = ?1")
                .map_err(|e| StorageError::OperationFailed(format!("Failed to prepare statement: {}", e)))?;

            let mut values = HashMap::with_capacity(keys.len());
            for key in keys {
                let raw: Option<String> = stmt
                    .query_row(params![&key], |row| row.get(0))
                    .optional()
                    .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?;

                let parsed = match raw {
                    Some(value) => Some(serde_json::from_str(&value).map_err(|e| {
                        StorageError::OperationFailed(format!("Deserialize failed: {}", e))
                    })?),
                    None => None,
                };
                values.insert(key, parsed);
            }

            Ok(values)
        })
        .await
    }

    async fn set_many
(&self, entries: HashMap<String, serde_json::Value>) -> Result<()> {
        let entries = entries
            .into_iter()
            .map(|(key, value)| {
                serde_json::to_string(&value)
                    .map(|json| (key, json))
                    .map_err(|e| StorageError::OperationFailed(format!("Serialize failed: {}", e)))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        self.run_tx(move |tx| {
            {
                let mut stmt = tx
                    .prepare(
                        "INSERT OR REPLACE INTO storage (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
                    )
                    .map_err(|e| StorageError::OperationFailed(format!("Prepare failed: {}", e)))?;

                for (key, value) in entries {
                    stmt.execute(params![key, value]).map_err(|e| {
                        StorageError::OperationFailed(format!("Batch insert failed: {}", e))
                    })?;
                }
            }
            Ok(())
        })
        .await
    }

    async fn delete_many(&self, keys: &[String]) -> Result<usize> {
        let keys = keys.to_vec();
        self.run_tx(move |tx| {
            let mut deleted = 0usize;

            {
                let mut stmt = tx
                    .prepare("DELETE FROM storage WHERE key = ?1")
                    .map_err(|e| StorageError::OperationFailed(format!("Prepare failed: {}", e)))?;

                for key in keys {
                    let rows = stmt.execute(params![key]).map_err(|e| {
                        StorageError::OperationFailed(format!("Batch delete failed: {}", e))
                    })?;
                    deleted += rows;
                }
            }
            Ok(deleted)
        })
        .await
    }

    async fn store_memory(&self, memory: MemoryEntry) -> Result<Uuid> {
        let id = memory.id;
        let company_id = memory.company_id;
        let agent_id = memory.agent_id.map(|a| a.to_string());
        let category = memory.category_str().to_string();
        let content = memory.content;
        let importance = memory.importance;
        let access_count = memory.access_count as i64;
        let expires_at = memory.expires_at.map(|dt| dt.to_rfc3339());
        let tags = serde_json::to_string(&memory.tags).map_err(|e| {
            StorageError::OperationFailed(format!("Failed to serialize tags: {}", e))
        })?;

        self.run_db(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO memories
                (id, company_id, agent_id, content, category, importance, access_count,
                 last_accessed_at, created_at, expires_at, tags)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, ?8, ?9)",
                params![
                    id.to_string(),
                    company_id,
                    agent_id,
                    content,
                    category,
                    importance,
                    access_count,
                    expires_at,
                    tags
                ],
            )
            .map_err(|e| StorageError::OperationFailed(format!("Failed to store memory: {}", e)))?;

            Ok(())
        })
        .await?;

        Ok(id)
    }

    async fn get_memory(&self, id: Uuid) -> Result<Option<MemoryEntry>> {
        let id_str = id.to_string();

        self.run_db(move |conn| {
            let mut stmt = conn
                .prepare_cached(
                    "SELECT id, company_id, agent_id, content, category, importance,
                            access_count, last_accessed_at, created_at, expires_at, tags
                     FROM memories WHERE id = ?1",
                )
                .map_err(|e| StorageError::OperationFailed(format!("Failed to prepare statement: {}", e)))?;

            stmt.query_row(params![id_str], parse_memory_row)
                .optional()
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))
        })
        .await
    }

    async fn search_memories(&self, query: &MemorySearchQuery) -> Result<Vec<MemoryEntry>> {
        let query = query.clone();
        self.run_db(move |conn| {
            let base_sql = "SELECT id, company_id, agent_id, content, category, importance,
                            access_count, last_accessed_at, created_at, expires_at, tags
                     FROM memories WHERE 1=1";

            let mut stmt = conn.prepare(base_sql).map_err(|e| {
                StorageError::OperationFailed(format!("Failed to prepare statement: {}", e))
            })?;

            let rows = stmt
                .query_map([], parse_memory_row)
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?;

            let mut memories: Vec<MemoryEntry> = rows.filter_map(|r| r.ok()).collect();

            if let Some(ref company_id) = query.company_id {
                memories.retain(|m| &m.company_id == company_id);
            }

            if let Some(ref agent_id) = query.agent_id {
                memories.retain(|m| m.agent_id.as_ref() == Some(agent_id));
            }

            if let Some(category) = query.category {
                memories.retain(|m| m.category == category);
            }

            if let Some(min_importance) = query.min_importance {
                memories.retain(|m| m.importance >= min_importance);
            }

            if let Some(ref keywords) = query.keywords {
                let keyword_lower = keywords.join(" ").to_lowercase();
                memories.retain(|m| m.content.to_lowercase().contains(&keyword_lower));
            }

            memories.sort_by(|a, b| b.importance.cmp(&a.importance));

            if let Some(limit) = query.limit {
                memories.truncate(limit);
            }

            Ok(memories)
        })
        .await
    }

    async fn touch_memory(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();

        self.run_db(move |conn| {
            conn.execute(
                "UPDATE memories SET last_accessed_at = CURRENT_TIMESTAMP, access_count = access_count + 1 WHERE id = ?1",
                params![id_str],
            )
            .map_err(|e| StorageError::OperationFailed(format!("Failed to touch memory: {}", e)))?;

            Ok(())
        })
        .await
    }

    async fn compress_memories(&self) -> Result<Vec<CompressedMemory>> {
        Ok(vec![])
    }

    async fn forget_memories(&self) -> Result<Vec<Uuid>> {
        self.run_db(move |conn| {
            let mut stmt = conn
                .prepare("SELECT id FROM memories WHERE expires_at < CURRENT_TIMESTAMP")
                .map_err(|e| StorageError::OperationFailed(format!("Failed to prepare statement: {}", e)))?;

            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?;

            let ids = rows
                .collect::<std::result::Result<Vec<String>, _>>()
                .map_err(|e| StorageError::OperationFailed(format!("Collect failed: {}", e)))?;

            let uuids = ids
                .iter()
                .filter_map(|id| Uuid::parse_str(id).ok())
                .collect::<Vec<_>>();

            conn.execute("DELETE FROM memories WHERE expires_at < CURRENT_TIMESTAMP", [])
                .map_err(|e| StorageError::OperationFailed(format!("Failed to delete memories: {}", e)))?;

            Ok(uuids)
        })
        .await
    }

    async fn store_compressed(&self, memory: CompressedMemory) -> Result<Uuid> {
        let source_ids = serde_json::to_string(&memory.source_ids).map_err(|e| {
            StorageError::OperationFailed(format!("Failed to serialize source_ids: {}", e))
        })?;
        let key_facts = serde_json::to_string(&memory.key_facts).map_err(|e| {
            StorageError::OperationFailed(format!("Failed to serialize key_facts: {}", e))
        })?;
        let time_range_start = memory.time_range.0.to_rfc3339();
        let time_range_end = memory.time_range.1.to_rfc3339();
        let id = memory.id;
        let summary = memory.summary;
        let importance = memory.importance;

        self.run_db(move |conn| {
            conn.execute(
                "INSERT INTO compressed_memories
                (id, summary, source_ids, key_facts, time_range_start, time_range_end, importance, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)",
                params![
                    id.to_string(),
                    summary,
                    source_ids,
                    key_facts,
                    time_range_start,
                    time_range_end,
                    importance,
                ],
            )
            .map_err(|e| StorageError::OperationFailed(format!("Failed to store compressed memory: {}", e)))?;

            Ok(())
        })
        .await?;

        Ok(id)
    }

    async fn get_compressed_memories(&self) -> Result<Vec<CompressedMemory>> {
        self.run_db(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, summary, source_ids, key_facts, time_range_start, time_range_end, importance, created_at
                     FROM compressed_memories ORDER BY created_at DESC",
                )
                .map_err(|e| StorageError::OperationFailed(format!("Failed to prepare statement: {}", e)))?;

            let rows = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let summary: String = row.get(1)?;
                    let source_ids_str: String = row.get(2)?;
                    let key_facts_str: String = row.get(3)?;
                    let time_range_start: String = row.get(4)?;
                    let time_range_end: String = row.get(5)?;
                    let importance: i32 = row.get(6)?;
                    let created_at: String = row.get(7)?;

                    let source_ids: Vec<String> = serde_json::from_str(&source_ids_str)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                    let key_facts: Vec<String> = serde_json::from_str(&key_facts_str)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                    let source_ids = source_ids
                        .into_iter()
                        .filter_map(|id| Uuid::parse_str(&id).ok())
                        .collect();

                    let time_range = (
                        DateTime::parse_from_rfc3339(&time_range_start)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        DateTime::parse_from_rfc3339(&time_range_end)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    );

                    let created_at = DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    Ok(CompressedMemory {
                        id: Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4()),
                        summary,
                        source_ids,
                        key_facts,
                        time_range,
                        importance,
                        created_at,
                    })
                })
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?;

            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| StorageError::OperationFailed(format!("Collect failed: {}", e)).into())
        })
        .await
    }

    async fn get_expired_memories(&self) -> Result<Vec<MemoryEntry>> {
        self.run_db(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, company_id, agent_id, content, category, importance,
                            access_count, last_accessed_at, created_at, expires_at, tags
                     FROM memories WHERE expires_at < CURRENT_TIMESTAMP",
                )
                .map_err(|e| StorageError::OperationFailed(format!("Failed to prepare statement: {}", e)))?;

            let rows = stmt
                .query_map([], parse_memory_row)
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?;

            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| StorageError::OperationFailed(format!("Collect failed: {}", e)).into())
        })
        .await
    }

    async fn delete_expired_memories(&self) -> Result<usize> {
        self.run_db(move |conn| {
            let deleted = conn
                .execute("DELETE FROM memories WHERE expires_at < CURRENT_TIMESTAMP", [])
                .map_err(|e| StorageError::OperationFailed(format!("Failed to delete memories: {}", e)))?;

            Ok(deleted)
        })
        .await
    }

    async fn shutdown(&self) -> Result<()> {
        self.worker.shutdown()
    }


}

fn parse_memory_row
(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryEntry> {
    use clawlegion_core::AgentId;

    let id_str: String = row.get(0)?;
    let company_id_str: String = row.get(1)?;
    let agent_id_str: Option<String> = row.get(2)?;
    let content: String = row.get(3)?;
    let category_str: String = row.get(4)?;
    let importance: f64 = row.get(5)?;
    let access_count: i64 = row.get(6)?;
    let last_accessed_at: String = row.get(7)?;
    let created_at: String = row.get(8)?;
    let expires_at: Option<String> = row.get(9)?;
    let tags_str: String = row.get(10)?;

    let id = Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4());
    let company_id = Uuid::parse_str(&company_id_str).unwrap_or_else(|_| Uuid::new_v4());
    let agent_id = agent_id_str
        .and_then(|s| uuid::Uuid::parse_str(&s).ok())
        .map(AgentId::from);

    let category = match category_str.as_str() {
        "short_term" => MemoryCategory::ShortTerm,
        "long_term" => MemoryCategory::LongTerm,
        "procedural" => MemoryCategory::Procedural,
        "semantic" => MemoryCategory::Semantic,
        "episodic" => MemoryCategory::Episodic,
        _ => MemoryCategory::ShortTerm,
    };

    let last_accessed_at = DateTime::parse_from_rfc3339(&last_accessed_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let expires_at = expires_at
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();

    Ok(MemoryEntry {
        id,
        company_id,
        agent_id,
        content,
        category,
        importance,
        access_count: access_count as u64,
        last_accessed_at,
        created_at,
        tags,
        related_messages: vec![],
        expires_at,
    })
}

#[async_trait]
impl ConfigStorage for SqliteStorage {
    async fn get_config(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let key = key.to_string();
        self.run_db(move |conn| {
            let result: Option<String> = conn
                .query_row(
                    "SELECT value FROM config WHERE key = ?1",
                    params![key],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?;

            result
                .map(|json_str| {
                    serde_json::from_str(&json_str).map_err(|e| {
                        Error::Storage(StorageError::OperationFailed(format!(
                            "Failed to parse JSON value: {}",
                            e
                        )))
                    })
                })
                .transpose()
        })
        .await
    }

    async fn set_config(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let key = key.to_string();
        let json_str = serde_json::to_string(&value).map_err(|e| {
            StorageError::OperationFailed(format!("Failed to serialize config value: {}", e))
        })?;

        self.run_db(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO config (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
                params![key, json_str],
            )
            .map_err(|e| StorageError::OperationFailed(format!("Failed to set config: {}", e)))?;

            Ok(())
        })
        .await
    }

    async fn delete_config(&self, key: &str) -> Result<bool> {
        let key = key.to_string();
        self.run_db(move |conn| {
            let rows = conn
                .execute("DELETE FROM config WHERE key = ?1", params![key])
                .map_err(|e| StorageError::OperationFailed(format!("Failed to delete config: {}", e)))?;

            Ok(rows > 0)
        })
        .await
    }

    async fn list_config_keys(&self) -> Result<Vec<String>> {
        self.run_db(move |conn| {
            let mut stmt = conn
                .prepare("SELECT key FROM config ORDER BY key")
                .map_err(|e| StorageError::OperationFailed(format!("Failed to prepare statement: {}", e)))?;

            let rows = stmt
                .query_map([], |row| row.get(0))
                .map_err(|e| StorageError::OperationFailed(format!("Query failed: {}", e)))?;

            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| StorageError::OperationFailed(format!("Collect failed: {}", e)).into())
        })
        .await
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

        // Store each config entry
        for (key, value) in &config {
            let _ = self.set_config(key, value.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn test_config(name: &str) -> SqliteStorageConfig {
        SqliteStorageConfig {
            database_path: std::env::temp_dir().join(format!(
                "clawlegion-sqlite-storage-{}-{}.db",
                name,
                Uuid::new_v4()
            )),
            ..SqliteStorageConfig::default()
        }
    }

    #[tokio::test]
    async fn run_db_respects_operation_timeout() {
        let mut config = test_config("timeout");
        config.operation_timeout_ms = 25;
        let storage = SqliteStorage::new(config).expect("storage should initialize");

        let err = storage
            .run_db(|_| {
                thread::sleep(Duration::from_millis(100));
                Ok(())
            })
            .await
            .expect_err("operation should time out");

        assert!(err.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn shutdown_rejects_new_operations() {
        let storage = SqliteStorage::new(test_config("shutdown")).expect("storage should initialize");

        storage.shutdown().await.expect("shutdown should succeed");

        let err = storage
            .get("after-shutdown")
            .await
            .expect_err("operations after shutdown should fail");

        assert!(err
            .to_string()
            .contains("SQLite storage worker has been shut down"));
    }
}
