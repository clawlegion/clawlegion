use std::path::PathBuf;
use std::process::Command;

use async_trait::async_trait;
use clawlegion_core::{
    CompressedMemory, Error, MemoryEntry, MemorySearchQuery, Result, Storage, StorageCapabilities,
    StorageError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolStorageRuntime {
    Python,
    Remote,
}

#[derive(Debug, Clone)]
pub struct ProtocolBackedStorage {
    runtime: ProtocolStorageRuntime,
    entrypoint: String,
    plugin_id: String,
}

impl ProtocolBackedStorage {
    pub fn new(
        runtime: ProtocolStorageRuntime,
        entrypoint: impl Into<String>,
        plugin_id: impl Into<String>,
    ) -> Self {
        Self {
            runtime,
            entrypoint: entrypoint.into(),
            plugin_id: plugin_id.into(),
        }
    }

    async fn execute(
        &self,
        operation: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<ProtocolStorageResponse> {
        let request = ProtocolStorageRequest {
            plugin_id: self.plugin_id.clone(),
            operation: operation.into(),
            payload,
        };
        match self.runtime {
            ProtocolStorageRuntime::Python => self.execute_python(&request),
            ProtocolStorageRuntime::Remote => self.execute_remote(&request).await,
        }
    }

    fn execute_python(&self, request: &ProtocolStorageRequest) -> Result<ProtocolStorageResponse> {
        let payload = serde_json::to_string(request).map_err(map_storage_error)?;
        let script_path = if PathBuf::from(&self.entrypoint).is_absolute() {
            PathBuf::from(&self.entrypoint)
        } else {
            PathBuf::from("./plugins")
                .join(&self.plugin_id)
                .join(&self.entrypoint)
        };
        let output = Command::new("python3")
            .arg(script_path)
            .arg("--execute-storage-json")
            .arg(payload)
            .output()
            .map_err(|e| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "python storage protocol execution failed: {}",
                    e
                )))
            })?;
        if !output.status.success() {
            return Err(Error::Storage(StorageError::OperationFailed(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            )));
        }
        let response: ProtocolStorageResponse =
            serde_json::from_slice(&output.stdout).map_err(map_storage_error)?;
        response.ensure_ok()?;
        Ok(response)
    }

    async fn execute_remote(
        &self,
        request: &ProtocolStorageRequest,
    ) -> Result<ProtocolStorageResponse> {
        let endpoint = format!("{}/{}", self.entrypoint.trim_end_matches('/'), "storage");
        let client = if self.entrypoint.contains("127.0.0.1") || self.entrypoint.contains("localhost") {
            reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        } else {
            reqwest::Client::new()
        };
        let response = client
            .post(endpoint)
            .json(request)
            .send()
            .await
            .map_err(|e| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "remote storage request failed: {}",
                    e
                )))
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "remote storage response read failed: {}",
                e
            )))
        })?;
        if !status.is_success() {
            return Err(Error::Storage(StorageError::OperationFailed(format!(
                "remote storage returned {}: {}",
                status, body
            ))));
        }
        let response: ProtocolStorageResponse =
            serde_json::from_str(&body).map_err(map_storage_error)?;
        response.ensure_ok()?;
        Ok(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStorageRequest {
    pub plugin_id: String,
    pub operation: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStorageResponse {
    pub ok: bool,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

impl ProtocolStorageResponse {
    fn ensure_ok(&self) -> Result<()> {
        if self.ok {
            return Ok(());
        }
        Err(Error::Storage(StorageError::OperationFailed(
            self.error
                .clone()
                .unwrap_or_else(|| "storage protocol returned ok=false".to_string()),
        )))
    }
}

fn map_storage_error(error: impl std::fmt::Display) -> Error {
    Error::Storage(StorageError::OperationFailed(format!(
        "invalid storage protocol payload: {}",
        error
    )))
}

#[async_trait]
impl Storage for ProtocolBackedStorage {
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let response = self
            .execute("read", serde_json::json!({ "key": key }))
            .await?;
        Ok(response.data)
    }

    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.execute("create", serde_json::json!({ "key": key, "value": value }))
            .await?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let response = self
            .execute("delete", serde_json::json!({ "key": key }))
            .await?;
        Ok(response
            .data
            .as_ref()
            .and_then(|item| item.get("deleted"))
            .and_then(|item| item.as_bool())
            .unwrap_or(true))
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let response = self
            .execute("exists", serde_json::json!({ "key": key }))
            .await?;
        Ok(response
            .data
            .as_ref()
            .and_then(|item| item.get("exists"))
            .and_then(|item| item.as_bool())
            .unwrap_or(false))
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let response = self
            .execute("query", serde_json::json!({ "prefix": prefix }))
            .await?;
        let keys = response
            .data
            .as_ref()
            .and_then(|item| item.get("keys"))
            .and_then(|item| item.as_array())
            .ok_or_else(|| {
                Error::Storage(StorageError::OperationFailed(
                    "query response missing keys array".to_string(),
                ))
            })?;
        Ok(keys
            .iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect())
    }

    async fn store_memory(&self, memory: MemoryEntry) -> Result<uuid::Uuid> {
        let response = self
            .execute(
                "store_memory",
                serde_json::to_value(memory).map_err(map_storage_error)?,
            )
            .await?;
        let id = response
            .data
            .and_then(|item| item.get("id").cloned())
            .and_then(|item| item.as_str().map(ToString::to_string))
            .ok_or_else(|| {
                Error::Storage(StorageError::OperationFailed(
                    "store_memory response missing id".to_string(),
                ))
            })?;
        uuid::Uuid::parse_str(&id).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "invalid memory id in protocol response: {}",
                e
            )))
        })
    }

    async fn get_memory(&self, id: uuid::Uuid) -> Result<Option<MemoryEntry>> {
        let response = self
            .execute("get_memory", serde_json::json!({ "id": id }))
            .await?;
        match response.data {
            Some(value) if !value.is_null() => serde_json::from_value(value)
                .map(Some)
                .map_err(map_storage_error),
            _ => Ok(None),
        }
    }

    async fn search_memories(&self, query: &MemorySearchQuery) -> Result<Vec<MemoryEntry>> {
        let response = self
            .execute(
                "search_memories",
                serde_json::to_value(query).map_err(map_storage_error)?,
            )
            .await?;
        let arr = response.data.unwrap_or_else(|| serde_json::json!([]));
        serde_json::from_value(arr).map_err(map_storage_error)
    }

    async fn touch_memory(&self, id: uuid::Uuid) -> Result<()> {
        self.execute("touch_memory", serde_json::json!({ "id": id }))
            .await?;
        Ok(())
    }

    async fn compress_memories(&self) -> Result<Vec<CompressedMemory>> {
        let response = self
            .execute("compress_memories", serde_json::json!({}))
            .await?;
        serde_json::from_value(response.data.unwrap_or_else(|| serde_json::json!([])))
            .map_err(map_storage_error)
    }

    async fn forget_memories(&self) -> Result<Vec<uuid::Uuid>> {
        let response = self
            .execute("forget_memories", serde_json::json!({}))
            .await?;
        let values: Vec<String> =
            serde_json::from_value(response.data.unwrap_or_else(|| serde_json::json!([])))
                .map_err(map_storage_error)?;
        let mut ids = Vec::with_capacity(values.len());
        for value in values {
            ids.push(uuid::Uuid::parse_str(&value).map_err(|e| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "invalid forget_memories id: {}",
                    e
                )))
            })?);
        }
        Ok(ids)
    }

    async fn store_compressed(&self, memory: CompressedMemory) -> Result<uuid::Uuid> {
        let response = self
            .execute(
                "store_compressed",
                serde_json::to_value(memory).map_err(map_storage_error)?,
            )
            .await?;
        let id = response
            .data
            .and_then(|item| item.get("id").cloned())
            .and_then(|item| item.as_str().map(ToString::to_string))
            .ok_or_else(|| {
                Error::Storage(StorageError::OperationFailed(
                    "store_compressed response missing id".to_string(),
                ))
            })?;
        uuid::Uuid::parse_str(&id).map_err(|e| {
            Error::Storage(StorageError::OperationFailed(format!(
                "invalid compressed memory id in protocol response: {}",
                e
            )))
        })
    }

    async fn get_compressed_memories(&self) -> Result<Vec<CompressedMemory>> {
        let response = self
            .execute("get_compressed_memories", serde_json::json!({}))
            .await?;
        serde_json::from_value(response.data.unwrap_or_else(|| serde_json::json!([])))
            .map_err(map_storage_error)
    }

    async fn get_expired_memories(&self) -> Result<Vec<MemoryEntry>> {
        let response = self
            .execute("get_expired_memories", serde_json::json!({}))
            .await?;
        serde_json::from_value(response.data.unwrap_or_else(|| serde_json::json!([])))
            .map_err(map_storage_error)
    }

    async fn delete_expired_memories(&self) -> Result<Vec<uuid::Uuid>> {
        let response = self
            .execute("delete_expired_memories", serde_json::json!({}))
            .await?;
        let values: Vec<String> =
            serde_json::from_value(response.data.unwrap_or_else(|| serde_json::json!([])))
                .map_err(map_storage_error)?;
        let mut ids = Vec::with_capacity(values.len());
        for value in values {
            ids.push(uuid::Uuid::parse_str(&value).map_err(|e| {
                Error::Storage(StorageError::OperationFailed(format!(
                    "invalid delete_expired_memories id: {}",
                    e
                )))
            })?);
        }
        Ok(ids)
    }

    async fn shutdown(&self) -> Result<()> {
        self.execute("shutdown", serde_json::json!({})).await?;
        Ok(())
    }

    fn capabilities(&self) -> StorageCapabilities {
        StorageCapabilities {
            supports_batch_kv: false,
            supports_pagination: false,
            supports_transactions: false,
            supports_memory_filters: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    #[tokio::test]
    async fn python_protocol_storage_crud() {
        let temp_root = std::env::temp_dir().join(format!(
            "clawlegion-storage-python-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&temp_root).expect("create temp dir");
        let script_path = temp_root.join("storage_main.py");
        fs::write(
            &script_path,
            r#"#!/usr/bin/env python3
import json
import os
import sys

DB_FILE = os.path.join(os.path.dirname(__file__), "db.json")

def load_db():
    if not os.path.exists(DB_FILE):
        return {}
    with open(DB_FILE, "r", encoding="utf-8") as f:
        return json.load(f)

def save_db(db):
    with open(DB_FILE, "w", encoding="utf-8") as f:
        json.dump(db, f)

if "--execute-storage-json" in sys.argv:
    payload = json.loads(sys.argv[sys.argv.index("--execute-storage-json") + 1])
    op = payload["operation"]
    data = payload.get("payload", {})
    db = load_db()
    if op == "create":
        db[data["key"]] = data["value"]
        save_db(db)
        print(json.dumps({"ok": True, "data": {"written": True}, "error": None}))
    elif op == "read":
        print(json.dumps({"ok": True, "data": db.get(data["key"]), "error": None}))
    elif op == "delete":
        deleted = data["key"] in db
        db.pop(data["key"], None)
        save_db(db)
        print(json.dumps({"ok": True, "data": {"deleted": deleted}, "error": None}))
    elif op == "exists":
        print(json.dumps({"ok": True, "data": {"exists": data["key"] in db}, "error": None}))
    elif op == "query":
        prefix = data.get("prefix", "")
        keys = [k for k in db.keys() if k.startswith(prefix)]
        print(json.dumps({"ok": True, "data": {"keys": keys}, "error": None}))
    else:
        print(json.dumps({"ok": False, "data": None, "error": f"unsupported op: {op}"}))
        sys.exit(1)
"#,
        )
        .expect("write script");

        let storage = ProtocolBackedStorage::new(
            ProtocolStorageRuntime::Python,
            script_path.to_string_lossy().to_string(),
            "storage-python-test",
        );

        storage
            .set("k1", serde_json::json!({"name":"demo"}))
            .await
            .expect("set");
        let got = storage.get("k1").await.expect("get").expect("value");
        assert_eq!(got["name"], "demo");
        assert!(storage.exists("k1").await.expect("exists"));
        let keys = storage.list_keys("k").await.expect("list_keys");
        assert_eq!(keys.len(), 1);
        assert!(storage.delete("k1").await.expect("delete"));
        assert!(!storage.exists("k1").await.expect("exists after delete"));

        fs::remove_dir_all(&temp_root).expect("cleanup");
    }

    #[tokio::test]
    async fn remote_protocol_storage_query() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("local addr");
        let server = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer);
                let body = r#"{"ok":true,"data":{"keys":["a1","a2"]},"error":null}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let storage = ProtocolBackedStorage::new(
            ProtocolStorageRuntime::Remote,
            format!("http://{}", addr),
            "storage-remote-test",
        );
        let keys = storage.list_keys("a").await.expect("list keys");
        assert_eq!(keys, vec!["a1".to_string(), "a2".to_string()]);
        server.join().expect("join server");
    }
}
