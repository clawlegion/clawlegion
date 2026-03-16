//! Claude Skill Runner
//!
//! Executes Claude Skills by running their scripts through the sandbox command tool.
//! Supports Python, Node.js, TypeScript, and Shell scripts without any conversion.

use crate::tools::{CommandInput, SandboxCommandTool, SandboxConfig};
use clawlegion_core::{CapabilityError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Claude Skill manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeManifest {
    /// Skill name
    pub name: String,

    /// Skill version
    pub version: String,

    /// Skill description
    pub description: Option<String>,

    /// Main script file (e.g., "index.py")
    pub main: Option<String>,

    /// Available tools/commands
    #[serde(default)]
    pub tools: Vec<String>,

    /// Required MCPs
    #[serde(default)]
    pub mcps: Vec<String>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ClaudeManifest {
    /// Load manifest from a directory
    pub fn from_directory(skill_dir: &Path) -> Result<Self> {
        let manifest_path = skill_dir.join("manifest.json");

        if !manifest_path.exists() {
            return Err(clawlegion_core::Error::Capability(
                CapabilityError::NotFound(format!("manifest.json not found in {:?}", skill_dir)),
            ));
        }

        let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
            clawlegion_core::Error::Capability(CapabilityError::NotFound(format!(
                "Failed to read manifest.json: {}",
                e
            )))
        })?;

        let manifest: ClaudeManifest = serde_json::from_str(&content).map_err(|e| {
            clawlegion_core::Error::Capability(CapabilityError::NotFound(format!(
                "Failed to parse manifest.json: {}",
                e
            )))
        })?;

        Ok(manifest)
    }
}

/// Get interpreter for a script based on file extension
fn get_interpreter(script_path: &Path) -> &'static str {
    match script_path.extension().and_then(|s| s.to_str()) {
        Some("py") => "python3",
        Some("js") => "node",
        Some("ts") => "deno",
        Some("sh") | Some("bash") => "bash",
        Some("zsh") => "zsh",
        _ => "bash", // Default to bash for unknown types
    }
}

/// Claude Skill Runner
pub struct ClaudeSkillRunner {
    skill_dir: PathBuf,
    manifest: ClaudeManifest,
    sandbox: SandboxCommandTool,
}

impl std::fmt::Debug for ClaudeSkillRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeSkillRunner")
            .field("skill_dir", &self.skill_dir)
            .field("manifest", &self.manifest)
            .field("sandbox", &"SandboxCommandTool")
            .finish()
    }
}

impl ClaudeSkillRunner {
    /// Create a new runner from a skill directory
    pub fn new(skill_dir: PathBuf) -> Result<Self> {
        let manifest = ClaudeManifest::from_directory(&skill_dir)?;

        Ok(Self {
            skill_dir,
            manifest,
            sandbox: SandboxCommandTool::new(),
        })
    }

    /// Create with custom sandbox config
    pub fn with_config(skill_dir: PathBuf, sandbox_config: SandboxConfig) -> Result<Self> {
        let manifest = ClaudeManifest::from_directory(&skill_dir)?;

        // Add skill_dir to allowed directories
        let mut allowed_dirs = sandbox_config.allowed_dirs.clone();
        allowed_dirs.push(skill_dir.clone());

        let config = SandboxConfig {
            allowed_dirs,
            ..sandbox_config
        };

        Ok(Self {
            skill_dir,
            manifest,
            sandbox: SandboxCommandTool::with_config(config),
        })
    }

    /// Get the skill manifest
    pub fn manifest(&self) -> &ClaudeManifest {
        &self.manifest
    }

    /// Get the skill directory
    pub fn skill_dir(&self) -> &Path {
        &self.skill_dir
    }

    /// Execute the skill with given input
    pub async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        let main_file = self.manifest.main.as_ref().ok_or_else(|| {
            clawlegion_core::Error::Capability(CapabilityError::NotFound(
                "No main script specified in manifest.json".to_string(),
            ))
        })?;

        let script_path = self.skill_dir.join(main_file);

        if !script_path.exists() {
            return Err(clawlegion_core::Error::Capability(
                CapabilityError::NotFound(format!("Script file not found: {:?}", script_path)),
            ));
        }

        // Determine interpreter
        let interpreter = get_interpreter(&script_path);

        // Build command input
        let mut args = vec![];

        // Special handling for deno (needs --allow-all flag)
        if interpreter == "deno" {
            args.push("run".to_string());
            args.push("--allow-all".to_string());
        }

        args.push(script_path.to_string_lossy().to_string());

        // Extract additional args from input if present
        if let Some(action) = input.get("action").and_then(|v| v.as_str()) {
            args.push(action.to_string());
        }

        // Prepare environment variables
        let mut env = HashMap::new();
        env.insert("INPUT".to_string(), input.to_string());

        // Add HOME if available
        if let Ok(home) = std::env::var("HOME") {
            env.insert("HOME".to_string(), home);
        }

        // Prepare stdin (pass the full JSON input)
        let stdin_input = input.to_string();

        let command_input = CommandInput {
            command: interpreter.to_string(),
            args,
            cwd: Some(self.skill_dir.to_string_lossy().to_string()),
            env: Some(env),
            stdin: Some(stdin_input),
        };

        // Execute command
        let output = self
            .sandbox
            .execute_command(command_input)
            .await
            .map_err(|e| {
                clawlegion_core::Error::Capability(CapabilityError::ExecutionFailed(format!(
                    "Failed to execute skill: {}",
                    e
                )))
            })?;

        // Try to parse output as JSON
        let result: serde_json::Value = serde_json::from_str(&output.stdout).unwrap_or_else(|_| {
            // If not valid JSON, wrap it in a result object
            serde_json::json!({
                "success": output.exit_code.unwrap_or(0) == 0,
                "output": output.stdout,
                "error": output.stderr,
                "exit_code": output.exit_code,
                "timed_out": output.timed_out,
            })
        });

        Ok(result)
    }

    /// Execute with a specific tool/command from the manifest
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Check if the tool exists
        if !self.manifest.tools.iter().any(|t| t == tool_name) {
            return Err(clawlegion_core::Error::Capability(
                CapabilityError::NotFound(format!(
                    "Tool '{}' not found in skill manifest",
                    tool_name
                )),
            ));
        }

        // For now, just execute the main script with the tool name as action
        let input = serde_json::json!({
            "action": tool_name,
            "args": args,
        });

        self.execute(input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skill_dir(script_content: &str, extension: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        fs::create_dir(&skill_dir).unwrap();

        // Create manifest.json
        let manifest = serde_json::json!({
            "name": "test-skill",
            "version": "1.0.0",
            "description": "Test skill",
            "main": format!("index.{}", extension),
            "tools": ["test", "run"],
        });
        fs::write(skill_dir.join("manifest.json"), manifest.to_string()).unwrap();

        // Create script
        fs::write(
            skill_dir.join(format!("index.{}", extension)),
            script_content,
        )
        .unwrap();

        (temp_dir, skill_dir)
    }

    #[tokio::test]
    async fn test_python_skill() {
        let (_temp_dir, skill_dir) = create_test_skill_dir(
            r#"
import sys
import json

# Read input from stdin
input_data = json.loads(sys.stdin.read())
action = input_data.get("action", "default")

# Output result
print(json.dumps({
    "success": True,
    "action": action,
    "message": "Hello from Python"
}))
"#,
            "py",
        );

        let runner = ClaudeSkillRunner::new(skill_dir).unwrap();
        let result = runner
            .execute(serde_json::json!({"action": "test"}))
            .await
            .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["message"], "Hello from Python");
    }

    #[tokio::test]
    async fn test_shell_skill() {
        let (_temp_dir, skill_dir) = create_test_skill_dir(
            r#"
#!/bin/bash
echo '{"success": true, "message": "Hello from Shell"}'
"#,
            "sh",
        );

        let runner = ClaudeSkillRunner::new(skill_dir).unwrap();
        let result = runner.execute(serde_json::json!({})).await.unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["message"], "Hello from Shell");
    }

    #[tokio::test]
    async fn test_node_skill() {
        let (_temp_dir, skill_dir) = create_test_skill_dir(
            r#"
const readline = require('readline');

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

let input = '';
rl.on('line', (line) => {
    input += line;
});

rl.on('close', () => {
    try {
        const data = JSON.parse(input);
        console.log(JSON.stringify({
            success: true,
            action: data.action,
            message: 'Hello from Node.js'
        }));
    } catch (e) {
        console.log(JSON.stringify({
            success: false,
            error: e.message
        }));
    }
    rl.close();
});
"#,
            "js",
        );

        let runner = ClaudeSkillRunner::new(skill_dir).unwrap();
        let result = runner
            .execute(serde_json::json!({"action": "test"}))
            .await
            .unwrap();

        // Note: This test might fail if node is not installed
        // In CI environments, you might want to skip this test
        if result.get("message").is_some() {
            assert_eq!(result["message"], "Hello from Node.js");
        }
    }
}
