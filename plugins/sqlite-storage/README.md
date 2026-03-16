# SQLite Storage Plugin for ClawLegion

基于 SQLite 的持久化存储插件，为 ClawLegion 多智能体系统提供可靠的存储支持。

## 功能特性

- **持久化存储**: 所有数据存储在 SQLite 数据库中
- **连接池**: 使用 r2d2 实现高效的连接管理
- **记忆管理**: 完整支持 Ebbinghaus 遗忘曲线
- **配置存储**: 支持 TOML 格式的配置文件管理
- **异步兼容**: 使用 tokio spawn_blocking 实现非阻塞操作
- **WAL 模式**: 支持 Write-Ahead Logging 提高并发性

## 安装

### 编译插件

```bash
cd plugins/sqlite-storage
cargo build --release
```

编译后的插件位于 `target/release/libsqlite_storage_plugin.so` (Linux/macOS) 或 `target/release/sqlite_storage_plugin.dll` (Windows)。

## 配置

在 ClawLegion 配置文件中添加以下配置：

```toml
[plugins.sqlite-storage]
enabled = true
database_path = "./data/legion.db"  # 数据库文件路径
pool_size = 5                        # 连接池大小
timeout_secs = 30                    # 连接超时（秒）
create_tables = true                 # 自动创建表
wal_mode = true                      # 启用 WAL 模式
cache_size = -2000                   # 缓存大小（KB，负值表示 KB）
```

### 配置选项说明

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `database_path` | string | `./data/legion.db` | SQLite 数据库文件路径 |
| `pool_size` | u32 | `5` | 连接池最大连接数 |
| `timeout_secs` | u64 | `30` | 连接超时时间（秒） |
| `create_tables` | bool | `true` | 是否自动创建数据库表 |
| `wal_mode` | bool | `true` | 是否启用 WAL 模式 |
| `cache_size` | i32 | `-2000` | 页面缓存大小（负值=KB） |

## 数据库 Schema

### 存储表 (storage)

```sql
CREATE TABLE storage (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 记忆表 (memories)

```sql
CREATE TABLE memories (
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
);
```

### 压缩记忆表 (compressed_memories)

```sql
CREATE TABLE compressed_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_ids TEXT NOT NULL,
    summary TEXT NOT NULL,
    key_facts TEXT,
    time_range_start DATETIME,
    time_range_end DATETIME,
    compressed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 配置表 (config)

```sql
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

## 使用示例

### 协议化 Storage 示例（v2）

新增 `protocol_storage_demo.py`，可作为 `ProtocolBackedStorage` 的 Python 协议端点：

```bash
cd plugins/sqlite-storage
python3 protocol_storage_demo.py --execute-storage-json \
  '{"plugin_id":"sqlite-storage","operation":"create","payload":{"key":"k1","value":{"x":1}}}'
python3 protocol_storage_demo.py --execute-storage-json \
  '{"plugin_id":"sqlite-storage","operation":"query","payload":{"prefix":"k"}}'
```

支持操作：`create` / `read` / `update`(由 create upsert 覆盖) / `delete` / `exists` / `query`。

### 作为存储后端

插件加载后，ClawLegion 核心会自动使用 SQLite 存储：

```rust
// 插件内部使用
let storage = SqliteStorage::new(config)?;

// 键值存储
storage.set("my_key", json!({"data": "value"})).await?;
let value = storage.get("my_key").await?;

// 记忆存储
storage.store_memory(memory_entry).await?;
let memories = storage.search_memories(&query).await?;
```

### 配置管理

```rust
use clawlegion_core::ConfigStorage;

// 存储配置
storage.set_config("api_key", json!("secret-key")).await?;
let api_key = storage.get_config("api_key").await?;

// 从文件加载配置
let config = storage.load_from_file(&PathBuf::from("config.toml")).await?;
```

## 性能优化建议

1. **WAL 模式**: 已默认启用，提高并发读写性能
2. **连接池大小**: 根据并发需求调整 `pool_size`
3. **缓存大小**: 增加 `cache_size` 可提高频繁访问的数据性能
4. **索引**: 记忆表已为常用查询字段创建索引

## 开发说明

### 目录结构

```
plugins/sqlite-storage/
├── Cargo.toml          # 依赖配置
├── README.md           # 本文档
└── src/
    ├── lib.rs          # 插件入口和测试
    ├── config.rs       # 配置结构
    ├── schema.rs       # 数据库 Schema
    └── storage.rs      # Storage trait 实现
```

### 运行测试

```bash
cd plugins/sqlite-storage
cargo test
```

### 依赖说明

- `rusqlite`: SQLite 绑定（bundled 模式，无需系统 SQLite）
- `r2d2` / `r2d2_sqlite`: 连接池
- `tokio`: 异步运行时
- `clawlegion-plugin-sdk`: 插件 SDK
- `clawlegion-storage`: 存储抽象层

## 许可证

与 ClawLegion 主项目相同。
