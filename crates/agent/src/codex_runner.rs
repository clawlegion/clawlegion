use async_trait::async_trait;
use clawlegion_core::{AgentError, Error, HeartbeatTrigger, LlmProviderConfig, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct CodexInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CodexRunRequest {
    pub prompt: String,
    pub working_directory: Option<PathBuf>,
    pub provider_config: LlmProviderConfig,
    pub profile: Option<String>,
    pub sandbox: Option<String>,
    pub approval_policy: Option<String>,
    pub system_prompt: Option<String>,
    pub web_search: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CodexRunResult {
    pub final_message: String,
    pub raw_events: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct CommandExecutionResult {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait]
pub trait CodexCommandExecutor: Send + Sync {
    async fn execute(&self, invocation: &CodexInvocation) -> Result<CommandExecutionResult>;
}

#[derive(Debug, Default)]
pub struct TokioCodexCommandExecutor;

#[async_trait]
impl CodexCommandExecutor for TokioCodexCommandExecutor {
    async fn execute(&self, invocation: &CodexInvocation) -> Result<CommandExecutionResult> {
        let mut command = Command::new(&invocation.program);
        command.args(&invocation.args);
        if let Some(cwd) = &invocation.cwd {
            command.current_dir(cwd);
        }
        command.envs(&invocation.env);

        let output = command.output().await.map_err(|error| {
            Error::Agent(AgentError::ExecutionFailed(format!(
                "failed to launch codex CLI '{}': {}",
                invocation.program, error
            )))
        })?;

        Ok(CommandExecutionResult {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

pub struct CodexCliRunner {
    executor: Arc<dyn CodexCommandExecutor>,
}

impl Default for CodexCliRunner {
    fn default() -> Self {
        Self::new(Arc::new(TokioCodexCommandExecutor))
    }
}

impl CodexCliRunner {
    pub fn new(executor: Arc<dyn CodexCommandExecutor>) -> Self {
        Self { executor }
    }

    pub async fn run(&self, request: &CodexRunRequest) -> Result<CodexRunResult> {
        let invocation = build_codex_invocation(request)?;
        let output = self.executor.execute(&invocation).await?;

        if !matches!(output.status_code, Some(0)) {
            let detail = if output.stderr.trim().is_empty() {
                output.stdout.trim().to_string()
            } else {
                output.stderr.trim().to_string()
            };
            return Err(Error::Agent(AgentError::ExecutionFailed(format!(
                "codex exec failed{}{}",
                output
                    .status_code
                    .map(|code| format!(" with exit code {}", code))
                    .unwrap_or_default(),
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {}", detail)
                }
            ))));
        }

        parse_codex_jsonl(&output.stdout)
    }
}

pub fn build_codex_prompt(
    agent_name: &str,
    role: &str,
    title: &str,
    capabilities: &str,
    trigger: &HeartbeatTrigger,
) -> String {
    format!(
        "You are agent '{agent_name}' ({title}, role={role}).\nCapabilities: {capabilities}\nTrigger: {}\nRespond with the concrete work you performed and any important outcome.",
        describe_trigger(trigger)
    )
}

pub fn build_codex_invocation(request: &CodexRunRequest) -> Result<CodexInvocation> {
    let mut args = vec!["exec".to_string(), "--json".to_string()];

    if let Some(cwd) = &request.working_directory {
        args.push("-C".to_string());
        args.push(cwd.display().to_string());
    }

    if let Some(profile) = value_as_str(request.profile.as_ref()) {
        args.push("--profile".to_string());
        args.push(profile.to_string());
    }

    if let Some(sandbox) = value_as_str(request.sandbox.as_ref()) {
        args.push("--sandbox".to_string());
        args.push(sandbox.to_string());
    }

    if let Some(approval_policy) = value_as_str(request.approval_policy.as_ref()) {
        args.push("--ask-for-approval".to_string());
        args.push(approval_policy.to_string());
    }

    args.push("--model".to_string());
    args.push(request.provider_config.model.clone());

    args.push("-c".to_string());
    args.push(format!(
        "model_provider={}",
        quoted(&request.provider_config.provider)
    ));

    if let Some(api_base) = &request.provider_config.api_base {
        args.push("-c".to_string());
        args.push(format!(
            "model_providers.{}.base_url={}",
            request.provider_config.provider,
            quoted(api_base)
        ));
    }

    if let Some(reasoning_effort) = request
        .provider_config
        .extra
        .get("reasoning_effort")
        .and_then(Value::as_str)
    {
        args.push("-c".to_string());
        args.push(format!("model_reasoning_effort={reasoning_effort}"));
    }

    if let Some(web_search) = value_from_extra_or_request(&request.provider_config, "web_search")
        .or_else(|| value_as_str(request.web_search.as_ref()).map(str::to_string))
    {
        args.push("-c".to_string());
        args.push(format!("web_search={web_search}"));
    }

    if let Some(system_prompt) =
        value_from_extra_or_request(&request.provider_config, "system_prompt")
            .or_else(|| value_as_str(request.system_prompt.as_ref()).map(str::to_string))
    {
        args.push("-c".to_string());
        args.push(format!("system_prompt={}", quoted(&system_prompt)));
    }

    args.push(request.prompt.clone());

    let mut env = HashMap::new();
    if let Some(api_key) = &request.provider_config.api_key {
        let env_key = provider_env_key(&request.provider_config)?;
        env.insert(env_key, api_key.clone());
    }

    Ok(CodexInvocation {
        program: "codex".to_string(),
        args,
        env,
        cwd: request.working_directory.clone(),
    })
}

fn provider_env_key(config: &LlmProviderConfig) -> Result<String> {
    if let Some(env_key) = config.extra.get("env_key").and_then(Value::as_str) {
        return Ok(env_key.to_string());
    }

    match config.provider.as_str() {
        "openai" => Ok("OPENAI_API_KEY".to_string()),
        "anthropic" => Ok("ANTHROPIC_API_KEY".to_string()),
        other => Err(Error::Agent(AgentError::ExecutionFailed(format!(
            "provider '{}' requires adapter_config.llm_provider.extra.env_key when api_key is set",
            other
        )))),
    }
}

fn parse_codex_jsonl(stdout: &str) -> Result<CodexRunResult> {
    let mut raw_events = Vec::new();
    let mut messages: HashMap<String, String> = HashMap::new();
    let mut latest_message_id: Option<String> = None;

    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let event: Value = serde_json::from_str(line).map_err(|error| {
            Error::Agent(AgentError::ExecutionFailed(format!(
                "failed to parse codex JSONL event: {}",
                error
            )))
        })?;
        raw_events.push(event.clone());

        match event.get("type").and_then(Value::as_str) {
            Some("item.started") | Some("item.updated") | Some("item.completed") => {
                if let Some(item) = event.get("item") {
                    let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
                    if item_type == "agent_message" {
                        let id = item
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        let text = item
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        if !id.is_empty() {
                            latest_message_id = Some(id.clone());
                            merge_message(messages.entry(id).or_default(), &text);
                        }
                    }
                }
            }
            Some("turn.completed") => {}
            Some("turn.failed") => {
                let message = event
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("codex turn failed");
                return Err(Error::Agent(AgentError::ExecutionFailed(
                    message.to_string(),
                )));
            }
            Some("error") => {
                let message = event
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("codex error");
                return Err(Error::Agent(AgentError::ExecutionFailed(
                    message.to_string(),
                )));
            }
            _ => {}
        }
    }

    let final_message = latest_message_id
        .and_then(|id| messages.remove(&id))
        .unwrap_or_default();

    Ok(CodexRunResult {
        final_message,
        raw_events,
    })
}

fn merge_message(existing: &mut String, incoming: &str) {
    if incoming.is_empty() {
        return;
    }
    if existing.is_empty() || incoming.starts_with(existing.as_str()) {
        *existing = incoming.to_string();
    } else if !existing.ends_with(incoming) {
        existing.push_str(incoming);
    }
}

fn describe_trigger(trigger: &HeartbeatTrigger) -> String {
    match trigger {
        HeartbeatTrigger::Scheduled => "scheduled heartbeat".to_string(),
        HeartbeatTrigger::PrivateMessage { message_id } => {
            format!("private message ({message_id})")
        }
        HeartbeatTrigger::TaskAssigned { task_id } => {
            format!("task assigned ({task_id})")
        }
        HeartbeatTrigger::ManagerAssigned {
            task_id,
            manager_id,
        } => {
            format!("task assigned by manager {manager_id} ({task_id})")
        }
        HeartbeatTrigger::Custom { trigger_id, data } => {
            format!("custom trigger {trigger_id} with payload {data}")
        }
    }
}

fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization should not fail")
}

fn value_from_extra_or_request(config: &LlmProviderConfig, key: &str) -> Option<String> {
    config.extra.get(key).map(value_to_cli_string)
}

fn value_to_cli_string(value: &Value) -> String {
    match value {
        Value::String(string) => string.clone(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        other => other.to_string(),
    }
}

fn value_as_str(value: Option<&String>) -> Option<&str> {
    value.map(String::as_str).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider_config() -> LlmProviderConfig {
        LlmProviderConfig {
            provider: "openai".to_string(),
            model: "gpt-5-codex".to_string(),
            api_key: Some("test-key".to_string()),
            api_base: Some("https://example.com/v1".to_string()),
            organization: None,
            timeout_secs: None,
            extra: HashMap::from([
                (
                    "reasoning_effort".to_string(),
                    Value::String("high".to_string()),
                ),
                (
                    "system_prompt".to_string(),
                    Value::String("system".to_string()),
                ),
            ]),
        }
    }

    #[test]
    fn build_invocation_maps_provider_config() {
        let request = CodexRunRequest {
            prompt: "hello".to_string(),
            working_directory: Some(PathBuf::from("/tmp/workspace")),
            provider_config: provider_config(),
            profile: Some("custom".to_string()),
            sandbox: Some("workspace-write".to_string()),
            approval_policy: Some("never".to_string()),
            system_prompt: None,
            web_search: Some("disabled".to_string()),
        };

        let invocation = build_codex_invocation(&request).unwrap();

        assert_eq!(invocation.program, "codex");
        assert!(invocation.args.contains(&"--model".to_string()));
        assert!(invocation.args.contains(&"gpt-5-codex".to_string()));
        assert!(invocation
            .args
            .contains(&"model_provider=\"openai\"".to_string()));
        assert!(invocation
            .args
            .contains(&"model_providers.openai.base_url=\"https://example.com/v1\"".to_string()));
        assert!(invocation.args.contains(&"--profile".to_string()));
        assert_eq!(
            invocation.env.get("OPENAI_API_KEY"),
            Some(&"test-key".to_string())
        );
    }

    #[test]
    fn unknown_provider_requires_explicit_env_key_when_api_key_is_set() {
        let request = CodexRunRequest {
            prompt: "hello".to_string(),
            working_directory: None,
            provider_config: LlmProviderConfig {
                provider: "custom".to_string(),
                model: "custom-model".to_string(),
                api_key: Some("secret".to_string()),
                api_base: None,
                organization: None,
                timeout_secs: None,
                extra: HashMap::new(),
            },
            profile: None,
            sandbox: None,
            approval_policy: None,
            system_prompt: None,
            web_search: None,
        };

        let error = build_codex_invocation(&request).unwrap_err();
        assert!(error.to_string().contains("extra.env_key"));
    }

    #[test]
    fn parse_jsonl_collects_final_message() {
        let stdout = concat!(
            "{\"type\":\"item.started\",\"item\":{\"type\":\"agent_message\",\"id\":\"msg-1\",\"text\":\"\"}}\n",
            "{\"type\":\"item.updated\",\"item\":{\"type\":\"agent_message\",\"id\":\"msg-1\",\"text\":\"Hello\"}}\n",
            "{\"type\":\"item.updated\",\"item\":{\"type\":\"agent_message\",\"id\":\"msg-1\",\"text\":\" world\"}}\n",
            "{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":10,\"cached_input_tokens\":2,\"output_tokens\":7}}\n"
        );

        let result = parse_codex_jsonl(stdout).unwrap();
        assert_eq!(result.final_message, "Hello world");
        assert_eq!(result.raw_events.len(), 4);
    }

    #[test]
    fn parse_jsonl_surfaces_error_events() {
        let stdout = "{\"type\":\"error\",\"message\":\"boom\"}\n";
        let error = parse_codex_jsonl(stdout).unwrap_err();
        assert!(error.to_string().contains("boom"));
    }
}
