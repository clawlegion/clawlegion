//! ClawLegion Multi-Agent System
//!
//! A highly plugin-based, fully automated Multi-Agent collaboration system
//! that simulates real company organizational structure.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use clawlegion_agent::AgentRegistry;
use clawlegion_api::{ApiServer, ApiServerConfig, ApiState};
use clawlegion_core::PluginTrustMode;
use clawlegion_core::{AgentTypeDef, Config};
use clawlegion_org::{OrgConfig, OrgTree};
use clawlegion_plugin::{PluginLoadConfig, PluginManager, SharedPluginManager};
use parking_lot::RwLock;
use tokio::sync::Notify;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// ClawLegion CLI
#[derive(Parser)]
#[command(name = "clawlegion")]
#[command(author = "ClawLegion Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Multi-Agent Collaboration System", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Config file path
    #[arg(short, long, default_value = "clawlegion.toml")]
    config: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the ClawLegion system
    Start {
        /// Run in daemon mode
        #[arg(short, long)]
        daemon: bool,

        /// Enable HTTP API server for monitoring interface
        #[arg(long)]
        with_api: bool,
    },

    /// Stop the running system
    Stop,

    /// Show system status
    Status,

    /// Manage agents
    Agent {
        #[command(subcommand)]
        action: AgentCommands,
    },

    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },

    /// Manage organization
    Org {
        #[command(subcommand)]
        action: OrgCommands,
    },

    /// Initialize a new configuration
    Init {
        /// Company name
        #[arg(short, long)]
        name: Option<String>,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    /// List all agents
    List,

    /// Create a new agent
    Create {
        /// Agent name
        name: String,

        /// Agent role
        #[arg(short, long)]
        role: String,

        /// Agent type (react, flow, normal, codex, claude-code, open-code)
        #[arg(short, long, value_enum, default_value_t = AgentCliType::React)]
        type_: AgentCliType,

        /// Manager agent ID
        #[arg(short, long)]
        reports_to: Option<String>,
    },

    /// Get agent details
    Get {
        /// Agent ID
        id: String,
    },

    /// Remove an agent
    Remove {
        /// Agent ID
        id: String,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum AgentCliType {
    React,
    Flow,
    Normal,
    Codex,
    ClaudeCode,
    OpenCode,
}

impl From<AgentCliType> for AgentTypeDef {
    fn from(value: AgentCliType) -> Self {
        match value {
            AgentCliType::React => AgentTypeDef::React,
            AgentCliType::Flow => AgentTypeDef::Flow,
            AgentCliType::Normal => AgentTypeDef::Normal,
            AgentCliType::Codex => AgentTypeDef::Codex,
            AgentCliType::ClaudeCode => AgentTypeDef::ClaudeCode,
            AgentCliType::OpenCode => AgentTypeDef::OpenCode,
        }
    }
}

#[derive(Subcommand)]
enum PluginCommands {
    /// List all plugins
    List,

    /// Enable a plugin
    Enable {
        /// Plugin name
        name: String,
    },

    /// Disable a plugin
    Disable {
        /// Plugin name
        name: String,
    },

    /// Reload plugin configuration
    Reload {
        /// Plugin name
        name: String,
    },

    /// Install a plugin from a local path
    Install {
        /// Source directory containing plugin.toml
        path: String,
    },

    /// Uninstall a plugin
    Uninstall {
        /// Plugin name
        name: String,
    },

    /// Inspect plugin manifest and runtime state
    Inspect {
        /// Plugin name
        name: String,
    },

    /// Trust a public key for plugin verification
    Trust {
        /// Alias for the key
        alias: String,
        /// Public key path
        key_path: String,
    },

    /// Sign a plugin artifact with a private key
    Sign {
        /// Plugin name
        name: String,
        /// Private key path
        key_path: String,
    },

    /// Run plugin diagnostics
    Doctor,
}

#[derive(Subcommand)]
enum OrgCommands {
    /// Show organization tree
    Show,

    /// Export org tree to JSON
    Export {
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("clawlegion={}", cli.log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Run command
    match cli.command {
        Some(Commands::Start { daemon, with_api }) => {
            cmd_start(&cli.config, daemon, with_api).await?;
        }
        Some(Commands::Stop) => {
            cmd_stop().await?;
        }
        Some(Commands::Status) => {
            cmd_status(&cli.config).await?;
        }
        Some(Commands::Agent { action }) => {
            cmd_agent(&cli.config, action).await?;
        }
        Some(Commands::Plugin { action }) => {
            cmd_plugin(&cli.config, action).await?;
        }
        Some(Commands::Org { action }) => {
            cmd_org(&cli.config, action).await?;
        }
        Some(Commands::Init { name, output }) => {
            cmd_init(name, &output).await?;
        }
        None => {
            // Default: start the system (API disabled by default)
            cmd_start(&cli.config, false, false).await?;
        }
    }

    Ok(())
}

async fn cmd_start(config_path: &str, daemon: bool, with_api: bool) -> Result<()> {
    tracing::info!("Starting ClawLegion system...");
    tracing::info!("Config file: {}", config_path);
    tracing::info!("Daemon mode: {}", daemon);

    let config = Config::load_from_file(Path::new(config_path))?;

    println!("ClawLegion system started from {}", config_path);
    println!("System name: {}", config.system.name);
    println!("Config dir: {}", config.system.config_dir.display());
    println!("Data dir: {}", config.system.data_dir.display());

    // Start API server if enabled
    let api_runtime = if with_api {
        let (handle, shutdown, addr) = start_api_server(&config)?;
        tracing::info!("API server started at http://{}", addr);
        println!("API server started at http://{}", addr);
        println!("Endpoints:");
        println!("  GET  /api/agents          - List all agents");
        println!("  GET  /api/agents/:id      - Get agent details");
        println!("  GET  /api/org/tree        - Get organization tree");
        println!("  GET  /api/org/company     - Get company info");
        println!("  GET  /api/system/status   - Get system status");
        println!("  GET  /api/system/health   - Health check");
        Some((handle, shutdown))
    } else {
        None
    };

    println!("Press Ctrl+C to stop");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    if let Some((handle, shutdown)) = api_runtime {
        shutdown.notify_waiters();
        handle.await??;
    }

    tracing::info!("Shutting down ClawLegion system...");
    println!("\nClawLegion system stopped");

    Ok(())
}

/// Start the API server with real HTTP bind and graceful shutdown.
fn start_api_server(
    config: &Config,
) -> Result<(tokio::task::JoinHandle<Result<()>>, Arc<Notify>, String)> {
    let api_config = ApiServerConfig {
        host: config.system.api_server.host.clone(),
        port: config.system.api_server.port,
        cors_origins: config.system.api_server.cors_origins.clone(),
    };

    let addr = format!("{}:{}", api_config.host, api_config.port);
    let agent_registry = Arc::new(AgentRegistry::new());
    let (org_tree, org_config) = load_org_tree(config)?;
    let plugin_manager = build_plugin_manager(config)?;
    let state = ApiState::new(agent_registry, org_tree, org_config, plugin_manager);
    let server = ApiServer::new(api_config, state);
    let shutdown = server.shutdown_notifier();

    let handle = tokio::spawn(async move { server.run().await });
    Ok((handle, shutdown, addr))
}

fn build_plugin_manager(config: &Config) -> Result<SharedPluginManager> {
    let load_config = match config.system.plugin_trust.mode {
        PluginTrustMode::Development => {
            PluginLoadConfig::default().without_signature_verification()
        }
        PluginTrustMode::Production => {
            let key_path = config
                .system
                .plugin_trust
                .public_key_path
                .as_ref()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "production mode requires [system.plugin_trust].public_key_path"
                    )
                })?;
            let public_key = std::fs::read(key_path)?;
            PluginLoadConfig::default().with_signature_verification(public_key)
        }
    };
    let mut manager = PluginManager::with_load_config(load_config);

    for (plugin_id, entry) in &config.plugins {
        manager.set_plugin_config(plugin_id.clone(), entry.config.clone());
    }

    manager.discover()?;
    for (plugin_id, entry) in &config.plugins {
        if !entry.enabled && manager.inspect(plugin_id).is_ok() {
            manager.registry().set_enabled(plugin_id, false)?;
            manager.registry().set_plugin_state(
                plugin_id,
                clawlegion_core::PluginState::Disabled,
                Some("disabled by config".to_string()),
            )?;
        }
    }
    futures_executor::block_on(manager.initialize_all())?;

    Ok(Arc::new(RwLock::new(manager)))
}

fn load_org_tree(config: &Config) -> Result<(Arc<OrgTree>, Arc<OrgConfig>)> {
    let org_config_path = resolve_org_config_path(config);
    let org_config = OrgConfig::load_from_file(&org_config_path)?;
    let company = org_config.to_company();
    let org_tree = OrgTree::new(company.id);

    for agent in org_config.to_agents(company.id)? {
        org_tree.add_agent(agent)?;
    }

    Ok((Arc::new(org_tree), Arc::new(org_config)))
}

fn resolve_org_config_path(config: &Config) -> std::path::PathBuf {
    let config_dir = &config.system.config_dir;
    if config_dir.is_absolute() {
        config_dir.join("org.toml")
    } else {
        Path::new(".").join(config_dir).join("org.toml")
    }
}

async fn cmd_stop() -> Result<()> {
    tracing::info!("Stopping ClawLegion system...");
    println!(
        "Stop requested. Use the running daemon or service manager to terminate the live process."
    );
    Ok(())
}

async fn cmd_status(config_path: &str) -> Result<()> {
    let config = Config::load_from_file(Path::new(config_path))?;
    let company = config
        .companies
        .values()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no company configured in clawlegion.toml"))?;
    let org_path = Path::new(config_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("config")
        .join("org.toml");
    let org_config = OrgConfig::load_from_file(&org_path)?;
    println!("ClawLegion System Status");
    println!("========================");
    println!("Config: {}", config_path);
    println!("Company: {}", company.name);
    println!("Issue prefix: {}", company.issue_prefix);
    println!("Agents configured: {}", org_config.agents.len());
    println!("Plugins configured: {}", config.plugins.len());
    Ok(())
}

async fn cmd_agent(config_path: &str, action: AgentCommands) -> Result<()> {
    let config = Config::load_from_file(Path::new(config_path))?;
    let company = config
        .companies
        .values()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no company configured in clawlegion.toml"))?;
    let org_path = Path::new(config_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("config")
        .join("org.toml");
    let org_config = OrgConfig::load_from_file(&org_path)?;
    match action {
        AgentCommands::List => {
            println!("Configured agents for {}:", company.name);
            for agent in &org_config.agents {
                println!(
                    "- {} ({}, title={}, type={:?}, reports_to={:?})",
                    agent.name, agent.role, agent.title, agent.agent_type, agent.reports_to
                );
            }
        }
        AgentCommands::Create {
            name,
            role,
            type_,
            reports_to,
        } => {
            println!("Create request received for agent:");
            println!("  name: {}", name);
            println!("  role: {}", role);
            println!("  type: {:?}", type_);
            println!("  reports_to: {:?}", reports_to);
            println!(
                "Edit {} to persist the new agent into the organization.",
                config_path
            );
        }
        AgentCommands::Get { id } => {
            println!("Getting agent: {}", id);
        }
        AgentCommands::Remove { id } => {
            println!("Removing agent: {}", id);
        }
    }
    Ok(())
}

#[allow(clippy::await_holding_lock)]
async fn cmd_plugin(config_path: &str, action: PluginCommands) -> Result<()> {
    let config = Config::load_from_file(Path::new(config_path))?;
    let plugin_manager = build_plugin_manager(&config)?;

    match action {
        PluginCommands::List => {
            for plugin in plugin_manager.read().list_plugins() {
                println!(
                    "plugin\t{}\ttype={:?}\tstate={:?}\tenabled={}",
                    plugin.id, plugin.plugin_type, plugin.state, plugin.enabled
                );
            }
        }
        PluginCommands::Enable { name } => {
            plugin_manager.write().enable(&name).await?;
            println!("plugin\t{}\tenabled", name);
        }
        PluginCommands::Disable { name } => {
            plugin_manager.write().disable(&name).await?;
            println!("plugin\t{}\tdisabled", name);
        }
        PluginCommands::Reload { name } => {
            plugin_manager.write().reload_config(&name).await?;
            println!("plugin\t{}\treloaded", name);
        }
        PluginCommands::Install { path } => {
            let plugin = plugin_manager.write().install(Path::new(&path))?;
            println!("plugin\t{}\tinstalled_from\t{}", plugin.id, path);
        }
        PluginCommands::Uninstall { name } => {
            plugin_manager.write().uninstall(&name).await?;
            println!("plugin\t{}\tuninstalled", name);
        }
        PluginCommands::Inspect { name } => {
            let plugin = plugin_manager.read().inspect(&name)?;
            println!("{}", serde_json::to_string_pretty(&plugin)?);
        }
        PluginCommands::Trust { alias, key_path } => {
            let stored = plugin_manager
                .read()
                .trust_key(&alias, Path::new(&key_path))?;
            println!("trust_key	{}	stored={}", alias, stored.display());
        }
        PluginCommands::Sign { name, key_path } => {
            let signature = plugin_manager
                .read()
                .sign_plugin(&name, Path::new(&key_path))?;
            println!("signature	{}	path={}", name, signature.display());
        }
        PluginCommands::Doctor => {
            for (plugin_id, state, health) in plugin_manager.read().health_report() {
                println!(
                    "{}\t{:?}\t{}",
                    plugin_id,
                    state,
                    health.unwrap_or_else(|| "unknown".to_string())
                );
            }
        }
    }

    Ok(())
}

async fn cmd_org(config_path: &str, action: OrgCommands) -> Result<()> {
    let config = Config::load_from_file(Path::new(config_path))?;
    let company = config
        .companies
        .values()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no company configured in clawlegion.toml"))?;
    let org_path = Path::new(config_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("config")
        .join("org.toml");
    let org_config = OrgConfig::load_from_file(&org_path)?;
    match action {
        OrgCommands::Show => {
            println!("Organization summary for {}:", company.name);
            println!("  issue_prefix: {}", company.issue_prefix);
            println!(
                "  require_approval_for_new_agents: {}",
                company.require_approval_for_new_agents
            );
            println!("  brand_color: {:?}", company.brand_color);
            println!("  agent_count: {}", org_config.agents.len());
        }
        OrgCommands::Export { output } => {
            let output = output.unwrap_or_else(|| "org-export.toml".to_string());
            println!("Exporting organization template to {}", output);
            println!("Use the API or the org config file to generate a full runtime export.");
        }
    }
    Ok(())
}

async fn cmd_init(name: Option<String>, output: &str) -> Result<()> {
    use std::fs;
    use std::path::Path;

    let output_path = Path::new(output);

    fs::create_dir_all(output_path.join("config"))?;
    fs::create_dir_all(output_path.join("plugins"))?;
    fs::create_dir_all(output_path.join("data"))?;

    let config_content = format!(
        r#"[system]
name = "{}"
data_dir = "data"
config_dir = "config"
log_level = "info"

[system.plugin_trust]
mode = "development"

[llm_providers.default]
provider = "your_provider"
model = "your_model"
"#,
        name.unwrap_or_else(|| "ClawLegion Demo".to_string())
    );

    fs::write(output_path.join("clawlegion.toml"), config_content)?;

    let org_content = r#"[company]
name = "ClawLegion Demo"
issue_prefix = "CL"

[[agents]]
name = "CEO"
role = "ceo"
title = "首席执行官"
# Add researcher and executor agents as needed.
"#;

    fs::write(output_path.join("config").join("org.toml"), org_content)?;

    println!("Initialized ClawLegion configuration in {}", output);
    println!("\nNext steps:");
    println!("  1. Edit clawlegion.toml to configure your LLM provider");
    println!("  2. Edit config/org.toml to define your organization");
    println!("  3. Run 'clawlegion start' to start the system");

    Ok(())
}
