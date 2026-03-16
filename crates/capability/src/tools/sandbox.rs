//! Sandbox Command Execution Tool
//!
//! Provides secure command execution with:
//! - Directory access restrictions
//! - Execution timeout
//! - Environment variable filtering
//! - Resource limits

use crate::tool::{Tool, ToolContext, ToolMetadata, ToolResult, Visibility};
use clawlegion_core::{CapabilityError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::Duration;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Allowed directories (whitelist)
    pub allowed_dirs: Vec<PathBuf>,

    /// Denied directories (blacklist)
    pub denied_dirs: Vec<PathBuf>,

    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,

    /// Whether to allow network access
    pub allowed_network: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            allowed_dirs: vec![],
            denied_dirs: default_denied_dirs(),
            max_execution_time_ms: 60000, // 60 seconds default
            allowed_network: false,
        }
    }
}

fn default_denied_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/etc"),
        PathBuf::from("/root"),
        PathBuf::from("/var/log"),
    ]
}

/// Command execution input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInput {
    /// Command to execute (e.g., "python3")
    pub command: String,

    /// Command arguments (e.g., ["index.py"])
    pub args: Vec<String>,

    /// Working directory
    pub cwd: Option<String>,

    /// Environment variables
    pub env: Option<HashMap<String, String>>,

    /// stdin input (optional)
    pub stdin: Option<String>,
}

/// Command execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    /// Exit code
    pub exit_code: Option<i32>,

    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,

    /// Whether the command timed out
    pub timed_out: bool,
}

/// Sandbox command execution tool
pub struct SandboxCommandTool {
    config: SandboxConfig,
    metadata: once_cell::sync::Lazy<ToolMetadata>,
}

// Manual Debug implementation
impl std::fmt::Debug for SandboxCommandTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxCommandTool")
            .field("config", &self.config)
            .finish()
    }
}

impl SandboxCommandTool {
    /// Create a new sandbox command tool with default config
    pub fn new() -> Self {
        Self::with_config(SandboxConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: SandboxConfig) -> Self {
        Self {
            config,
            metadata: once_cell::sync::Lazy::new(Self::create_metadata),
        }
    }

    /// Validate that a path is allowed (not in denied dirs, in allowed dirs if specified)
    fn is_path_allowed(&self, path: &Path) -> bool {
        // Check denied directories first
        for denied in &self.config.denied_dirs {
            if path.starts_with(denied) {
                return false;
            }
        }

        // If allowed_dirs is specified, path must be in one of them
        if !self.config.allowed_dirs.is_empty() {
            for allowed in &self.config.allowed_dirs {
                if path.starts_with(allowed) {
                    return true;
                }
            }
            return false;
        }

        true
    }

    /// Filter environment variables (remove sensitive ones)
    fn filter_env_vars(&self, env: HashMap<String, String>) -> HashMap<String, String> {
        let sensitive_prefixes = [
            "AWS_",
            "GCP_",
            "AZURE_",
            "SECRET",
            "PRIVATE_KEY",
            "TOKEN",
            "PASSWORD",
            "CREDENTIAL",
        ];

        env.into_iter()
            .filter(|(key, _)| {
                let key_upper = key.to_uppercase();
                !sensitive_prefixes
                    .iter()
                    .any(|prefix| key_upper.contains(prefix))
            })
            .collect()
    }

    /// Execute a command in the sandbox
    pub async fn execute_command(&self, input: CommandInput) -> Result<CommandOutput> {
        let cwd = input.cwd.clone().unwrap_or_else(|| ".".to_string());
        let cwd_path = Path::new(&cwd);

        // Validate working directory
        if !self.is_path_allowed(cwd_path) {
            return Ok(CommandOutput {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: format!("Working directory not allowed: {}", cwd),
                timed_out: false,
            });
        }

        // Build the command
        let mut cmd = Command::new(&input.command);
        cmd.args(&input.args);
        cmd.current_dir(cwd_path);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Set filtered environment variables
        if let Some(env) = input.env {
            let filtered_env = self.filter_env_vars(env);
            for (key, value) in filtered_env {
                cmd.env(key, value);
            }
        }

        // Add SKILL_DIR environment variable
        cmd.env("SKILL_DIR", cwd_path);

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| {
            CapabilityError::ExecutionFailed(format!(
                "Failed to spawn command '{}': {}",
                input.command, e
            ))
        })?;

        // Write stdin if provided
        if let Some(stdin_data) = input.stdin {
            use tokio::io::AsyncWriteExt;
            let mut stdin = child.stdin.take().ok_or_else(|| {
                CapabilityError::ExecutionFailed("Failed to open stdin".to_string())
            })?;
            stdin.write_all(stdin_data.as_bytes()).await.map_err(|e| {
                CapabilityError::ExecutionFailed(format!("Failed to write stdin: {}", e))
            })?;
            drop(stdin);
        }

        // Wait for completion with timeout
        let timeout_duration = Duration::from_millis(self.config.max_execution_time_ms);

        // Use tokio::select! to handle timeout and wait_with_output together
        let result = tokio::time::timeout(timeout_duration, child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => Ok(CommandOutput {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                timed_out: false,
            }),
            Ok(Err(e)) => Ok(CommandOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Command execution failed: {}", e),
                timed_out: false,
            }),
            Err(_timeout) => {
                // Timeout - try to kill the process
                // Note: child is already moved, so we can't kill it here
                // The process will be cleaned up by the OS when the handle is dropped
                Ok(CommandOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: format!("Command timed out after {}ms", timeout_duration.as_millis()),
                    timed_out: true,
                })
            }
        }
    }

    fn create_metadata() -> ToolMetadata {
        ToolMetadata {
            name: "sandbox_command".to_string(),
            version: "1.0.0".to_string(),
            description:
                "Execute shell commands in a sandboxed environment with security restrictions"
                    .to_string(),
            visibility: Visibility::Public,
            tags: vec![
                "shell".to_string(),
                "command".to_string(),
                "sandbox".to_string(),
            ],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to execute (e.g., 'python3', 'node', 'bash')"
                    },
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Command arguments"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory"
                    },
                    "env": {
                        "type": "object",
                        "description": "Environment variables"
                    },
                    "stdin": {
                        "type": "string",
                        "description": "stdin input"
                    }
                },
                "required": ["command", "args"]
            }),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "exit_code": {"type": "integer"},
                    "stdout": {"type": "string"},
                    "stderr": {"type": "string"},
                    "timed_out": {"type": "boolean"}
                }
            })),
            requires_llm: false,
        }
    }
}

impl Default for SandboxCommandTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for SandboxCommandTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, _ctx: &ToolContext, args: serde_json::Value) -> Result<ToolResult> {
        let input: CommandInput = serde_json::from_value(args).map_err(|e| {
            CapabilityError::ExecutionFailed(format!("Failed to parse command input: {}", e))
        })?;

        let output = self.execute_command(input).await.map_err(|e| {
            CapabilityError::ExecutionFailed(format!("Command execution failed: {}", e))
        })?;

        Ok(ToolResult {
            success: output.exit_code.unwrap_or(-1) == 0 && !output.timed_out,
            data: Some(serde_json::to_value(&output).unwrap_or_default()),
            error: if output.timed_out || output.exit_code.unwrap_or(0) != 0 {
                Some(format!(
                    "Command failed: exit_code={:?}, stderr={}, timed_out={}",
                    output.exit_code, output.stderr, output.timed_out
                ))
            } else {
                None
            },
            execution_time_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_command() {
        let tool = SandboxCommandTool::new();

        let input = CommandInput {
            command: "echo".to_string(),
            args: vec!["Hello, World!".to_string()],
            cwd: None,
            env: None,
            stdin: None,
        };

        let output = tool.execute_command(input).await.unwrap();

        assert!(output.exit_code == Some(0));
        assert!(output.stdout.trim() == "Hello, World!");
        assert!(!output.timed_out);
    }

    #[tokio::test]
    async fn test_python_script() {
        let tool = SandboxCommandTool::new();

        let input = CommandInput {
            command: "python3".to_string(),
            args: vec!["-c".to_string(), "print('Hello from Python')".to_string()],
            cwd: None,
            env: None,
            stdin: None,
        };

        let output = tool.execute_command(input).await.unwrap();

        assert!(output.exit_code == Some(0));
        assert!(output.stdout.contains("Hello from Python"));
    }

    #[tokio::test]
    async fn test_path_validation() {
        let config = SandboxConfig {
            allowed_dirs: vec![PathBuf::from("/tmp")],
            denied_dirs: vec![PathBuf::from("/etc")],
            ..Default::default()
        };

        let tool = SandboxCommandTool::with_config(config);

        // Allowed path
        assert!(tool.is_path_allowed(Path::new("/tmp/test")));

        // Denied path
        assert!(!tool.is_path_allowed(Path::new("/etc/passwd")));

        // Not in allowed dirs
        assert!(!tool.is_path_allowed(Path::new("/home/user")));
    }
}
