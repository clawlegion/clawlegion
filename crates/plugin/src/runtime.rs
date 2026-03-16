use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};

use clawlegion_core::{PluginManifest, PluginRuntimeFamily, PluginState, PluginType};

#[derive(Debug, Clone)]
pub struct RuntimeProbe {
    pub state: PluginState,
    pub health: String,
    pub detail: Option<String>,
}

pub trait PluginRuntimeAdapter: Send + Sync {
    fn runtime_family(&self) -> PluginRuntimeFamily;
    fn probe(&self, manifest: &PluginManifest, entrypoint: Option<&Path>) -> Result<RuntimeProbe>;
}

pub struct NativeRuntimeAdapter;
pub struct ProcessRuntimeAdapter;
pub struct RemoteRuntimeAdapter;
pub struct ConfigRuntimeAdapter;

#[derive(Debug, Clone)]
pub struct PythonSupervisorConfig {
    pub enabled: bool,
    pub max_restarts: usize,
    pub restart_backoff_ms: u64,
    pub log_capacity: usize,
    pub serve_args: Vec<String>,
}

impl Default for PythonSupervisorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_restarts: 3,
            restart_backoff_ms: 600,
            log_capacity: 200,
            serve_args: vec!["--serve".to_string()],
        }
    }
}

struct PythonProcessRecord {
    child: Arc<Mutex<Child>>,
    logs: Arc<Mutex<VecDeque<String>>>,
    restart_count: Arc<Mutex<usize>>,
}

#[derive(Clone, Default)]
pub struct PythonPluginSupervisor {
    processes: Arc<Mutex<HashMap<String, PythonProcessRecord>>>,
}

impl PythonPluginSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_started(
        &self,
        plugin_id: &str,
        entrypoint: &Path,
        config: &PythonSupervisorConfig,
    ) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }
        let mut guard = self
            .processes
            .lock()
            .map_err(|_| anyhow!("python supervisor lock poisoned"))?;
        if guard.contains_key(plugin_id) {
            return Ok(());
        }

        let record = spawn_record(
            plugin_id,
            entrypoint.to_path_buf(),
            config.clone(),
            Arc::clone(&self.processes),
        )?;
        guard.insert(plugin_id.to_string(), record);
        Ok(())
    }

    pub fn stop(&self, plugin_id: &str) -> Result<()> {
        let mut guard = self
            .processes
            .lock()
            .map_err(|_| anyhow!("python supervisor lock poisoned"))?;
        if let Some(record) = guard.remove(plugin_id) {
            if let Ok(mut child) = record.child.lock() {
                let _ = child.kill();
            }
        }
        Ok(())
    }

    pub fn logs(&self, plugin_id: &str) -> Vec<String> {
        let logs_ref = {
            let guard = match self.processes.lock() {
                Ok(g) => g,
                Err(_) => return vec![],
            };
            guard.get(plugin_id).map(|record| Arc::clone(&record.logs))
        };
        let Some(logs_ref) = logs_ref else {
            return vec![];
        };
        let output = match logs_ref.lock() {
            Ok(logs) => logs.iter().cloned().collect(),
            Err(_) => vec![],
        };
        output
    }

    pub fn restart_count(&self, plugin_id: &str) -> usize {
        let counter_ref = {
            let guard = match self.processes.lock() {
                Ok(g) => g,
                Err(_) => return 0,
            };
            guard
                .get(plugin_id)
                .map(|record| Arc::clone(&record.restart_count))
        };
        let Some(counter_ref) = counter_ref else {
            return 0;
        };
        let output = match counter_ref.lock() {
            Ok(count) => *count,
            Err(_) => 0,
        };
        output
    }
}

fn spawn_record(
    plugin_id: &str,
    entrypoint: PathBuf,
    config: PythonSupervisorConfig,
    all_processes: Arc<Mutex<HashMap<String, PythonProcessRecord>>>,
) -> Result<PythonProcessRecord> {
    let mut command = Command::new("python3");
    command.arg(&entrypoint);
    for arg in &config.serve_args {
        command.arg(arg);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| {
        anyhow!(
            "failed to start supervised python plugin {} at {}: {}",
            plugin_id,
            entrypoint.display(),
            e
        )
    })?;

    let logs = Arc::new(Mutex::new(VecDeque::with_capacity(config.log_capacity)));
    if let Some(stdout) = child.stdout.take() {
        let logs_ref = Arc::clone(&logs);
        let pid = plugin_id.to_string();
        let cap = config.log_capacity;
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(|line| line.ok()) {
                push_log(&logs_ref, cap, format!("[{}][stdout] {}", pid, line));
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let logs_ref = Arc::clone(&logs);
        let pid = plugin_id.to_string();
        let cap = config.log_capacity;
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(|line| line.ok()) {
                push_log(&logs_ref, cap, format!("[{}][stderr] {}", pid, line));
            }
        });
    }

    let child_arc = Arc::new(Mutex::new(child));
    let restart_count = Arc::new(Mutex::new(0_usize));

    {
        let child_ref = Arc::clone(&child_arc);
        let logs_ref = Arc::clone(&logs);
        let restart_ref = Arc::clone(&restart_count);
        let plugin_id_owned = plugin_id.to_string();
        let entrypoint_owned = entrypoint.clone();
        let config_owned = config.clone();
        thread::spawn(move || loop {
            let exit = {
                let mut child_guard = match child_ref.lock() {
                    Ok(g) => g,
                    Err(_) => break,
                };
                child_guard.wait()
            };
            let status = match exit {
                Ok(s) => s,
                Err(_) => break,
            };
            push_log(
                &logs_ref,
                config_owned.log_capacity,
                format!("[{}] exited with status {}", plugin_id_owned, status),
            );

            let mut restart_guard = match restart_ref.lock() {
                Ok(g) => g,
                Err(_) => break,
            };
            if *restart_guard >= config_owned.max_restarts {
                push_log(
                    &logs_ref,
                    config_owned.log_capacity,
                    format!(
                        "[{}] restart budget exhausted ({})",
                        plugin_id_owned, config_owned.max_restarts
                    ),
                );
                break;
            }
            *restart_guard += 1;
            drop(restart_guard);
            thread::sleep(Duration::from_millis(config_owned.restart_backoff_ms));

            match spawn_record(
                &plugin_id_owned,
                entrypoint_owned.clone(),
                config_owned.clone(),
                Arc::clone(&all_processes),
            ) {
                Ok(new_record) => {
                    if let Ok(mut all) = all_processes.lock() {
                        all.insert(plugin_id_owned.clone(), new_record);
                    }
                    break;
                }
                Err(e) => {
                    push_log(
                        &logs_ref,
                        config_owned.log_capacity,
                        format!("[{}] restart failed: {}", plugin_id_owned, e),
                    );
                }
            }
        });
    }

    Ok(PythonProcessRecord {
        child: child_arc,
        logs,
        restart_count,
    })
}

fn push_log(logs: &Arc<Mutex<VecDeque<String>>>, cap: usize, line: String) {
    if let Ok(mut guard) = logs.lock() {
        if guard.len() >= cap {
            let _ = guard.pop_front();
        }
        guard.push_back(line);
    }
}

impl PluginRuntimeAdapter for NativeRuntimeAdapter {
    fn runtime_family(&self) -> PluginRuntimeFamily {
        PluginRuntimeFamily::NativeAbi
    }

    fn probe(&self, manifest: &PluginManifest, entrypoint: Option<&Path>) -> Result<RuntimeProbe> {
        let entrypoint = entrypoint.ok_or_else(|| anyhow!("missing native plugin entrypoint"))?;
        if entrypoint.exists() {
            Ok(RuntimeProbe {
                state: PluginState::Initialized,
                health: "healthy".to_string(),
                detail: Some(format!("native artifact found for {}", manifest.id)),
            })
        } else {
            Ok(RuntimeProbe {
                state: PluginState::Degraded,
                health: "missing-artifact".to_string(),
                detail: Some(format!("entrypoint {} not found", entrypoint.display())),
            })
        }
    }
}

impl PluginRuntimeAdapter for ProcessRuntimeAdapter {
    fn runtime_family(&self) -> PluginRuntimeFamily {
        PluginRuntimeFamily::HostProcess
    }

    fn probe(&self, manifest: &PluginManifest, entrypoint: Option<&Path>) -> Result<RuntimeProbe> {
        let entrypoint = entrypoint.ok_or_else(|| anyhow!("missing process plugin entrypoint"))?;
        if !entrypoint.exists() {
            return Ok(RuntimeProbe {
                state: PluginState::Degraded,
                health: "missing-entrypoint".to_string(),
                detail: Some(format!(
                    "python entrypoint {} not found",
                    entrypoint.display()
                )),
            });
        }

        let health_output = Command::new("python3")
            .arg(entrypoint)
            .arg("--health")
            .output();

        match health_output {
            Ok(output) if output.status.success() => Ok(RuntimeProbe {
                state: PluginState::Initialized,
                health: "healthy".to_string(),
                detail: Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
            }),
            Ok(output) => Ok(RuntimeProbe {
                state: PluginState::Degraded,
                health: "probe-failed".to_string(),
                detail: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            }),
            Err(_) => Ok(RuntimeProbe {
                state: PluginState::Initialized,
                health: "unverified".to_string(),
                detail: Some(format!(
                    "python health probe unavailable for {}",
                    manifest.id
                )),
            }),
        }
    }
}

impl PluginRuntimeAdapter for RemoteRuntimeAdapter {
    fn runtime_family(&self) -> PluginRuntimeFamily {
        PluginRuntimeFamily::RemoteProtocol
    }

    fn probe(&self, manifest: &PluginManifest, _entrypoint: Option<&Path>) -> Result<RuntimeProbe> {
        let endpoint = manifest.entrypoint.to_lowercase();
        let healthy = endpoint.starts_with("http://")
            || endpoint.starts_with("https://")
            || endpoint.starts_with("ws://")
            || endpoint.starts_with("wss://");

        Ok(RuntimeProbe {
            state: if healthy {
                PluginState::Initialized
            } else {
                PluginState::Degraded
            },
            health: if healthy {
                "configured".to_string()
            } else {
                "invalid-endpoint".to_string()
            },
            detail: Some(format!(
                "remote runtime endpoint registered for {} (timeout hint: {:?})",
                manifest.id,
                Duration::from_secs(
                    manifest
                        .healthcheck
                        .as_ref()
                        .and_then(|health| health.timeout_secs)
                        .unwrap_or(10)
                )
            )),
        })
    }
}

impl PluginRuntimeAdapter for ConfigRuntimeAdapter {
    fn runtime_family(&self) -> PluginRuntimeFamily {
        PluginRuntimeFamily::Config
    }

    fn probe(&self, manifest: &PluginManifest, _entrypoint: Option<&Path>) -> Result<RuntimeProbe> {
        Ok(RuntimeProbe {
            state: PluginState::Initialized,
            health: "healthy".to_string(),
            detail: Some(format!(
                "config plugin {} loaded from manifest",
                manifest.id
            )),
        })
    }
}

pub fn runtime_adapter(plugin_type: &PluginType) -> Box<dyn PluginRuntimeAdapter> {
    match plugin_type.runtime_family() {
        PluginRuntimeFamily::NativeAbi => Box::new(NativeRuntimeAdapter),
        PluginRuntimeFamily::HostProcess => Box::new(ProcessRuntimeAdapter),
        PluginRuntimeFamily::RemoteProtocol => Box::new(RemoteRuntimeAdapter),
        PluginRuntimeFamily::Config => Box::new(ConfigRuntimeAdapter),
    }
}
