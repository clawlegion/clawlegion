//! Database schema definitions and initialization

use rusqlite::{Connection, OptionalExtension};
use tracing::info;

/// Database schema version
pub const SCHEMA_VERSION: i32 = 1;

/// Initialize database schema
pub fn init_schema(conn: &Connection, create_tables: bool) -> rusqlite::Result<()> {
    if !create_tables {
        return Ok(());
    }

    // Enable foreign keys
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    // Create storage table for key-value pairs
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS storage (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )?;

    // Create memories table for Ebbinghaus forgetting curve
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            company_id TEXT NOT NULL,
            agent_id TEXT,
            content TEXT NOT NULL,
            category TEXT NOT NULL,
            importance REAL DEFAULT 0.5,
            access_count INTEGER DEFAULT 0,
            last_accessed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            expires_at DATETIME,
            tags TEXT
        );",
    )?;

    // Create compressed_memories table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compressed_memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_ids TEXT NOT NULL,
            summary TEXT NOT NULL,
            key_facts TEXT,
            time_range_start DATETIME,
            time_range_end DATETIME,
            compressed_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )?;

    // Create config table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )?;

    // Create schema_version table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            version INTEGER NOT NULL
        );",
    )?;

    // Insert or update schema version
    conn.execute(
        "INSERT OR REPLACE INTO schema_version (id, version) VALUES (1, ?1)",
        [SCHEMA_VERSION],
    )?;

    // Create indexes for better query performance
    create_indexes(conn)?;

    info!("Database schema initialized (version {})", SCHEMA_VERSION);

    Ok(())
}

/// Create database indexes
fn create_indexes(conn: &Connection) -> rusqlite::Result<()> {
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_memories_company ON memories(company_id);",
        "CREATE INDEX IF NOT EXISTS idx_memories_agent ON memories(agent_id);",
        "CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);",
        "CREATE INDEX IF NOT EXISTS idx_memories_expires ON memories(expires_at);",
        "CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);",
        "CREATE INDEX IF NOT EXISTS idx_storage_updated ON storage(updated_at);",
    ];

    for index_sql in indexes {
        conn.execute_batch(index_sql)?;
    }

    Ok(())
}

/// Configure SQLite for optimal performance
pub fn configure_pragmas(conn: &Connection, wal_mode: bool, cache_size: i32) -> rusqlite::Result<()> {
    // Enable WAL mode for better concurrency
    if wal_mode {
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    }

    // Set cache size (negative = KB)
    conn.execute_batch(&format!("PRAGMA cache_size = {};", cache_size))?;

    // Enable synchronous (NORMAL for good balance of safety and speed)
    conn.execute_batch("PRAGMA synchronous = NORMAL;")?;

    // Enable foreign key constraints
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    // Set busy timeout (in milliseconds)
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

    // Optimize temp store (MEMORY is usually faster)
    conn.execute_batch("PRAGMA temp_store = MEMORY;")?;

    // Set mmap size for faster reads (256MB)
    conn.execute_batch("PRAGMA mmap_size = 268435456;")?;

    info!(
        "SQLite configured: WAL={}, cache_size={}",
        wal_mode, cache_size
    );

    Ok(())
}

/// Get current schema version
pub fn get_schema_version(conn: &Connection) -> rusqlite::Result<Option<i32>> {
    // Check if schema_version table exists
    let table_exists: bool = conn.query_row(
        "SELECT EXISTS (
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='schema_version'
        )",
        [],
        |row| row.get(0),
    )?;

    if !table_exists {
        return Ok(None);
    }

    conn.query_row(
        "SELECT version FROM schema_version WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .optional()
}
