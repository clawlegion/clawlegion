use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};
use tokio::net::TcpListener;
use tokio::sync::Notify;
use axum::http::{HeaderValue, Method};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::routes::{agents, messages, org, system};
use crate::state::ApiState;

pub fn build_router(state: ApiState, cors_origins: &[String]) -> Router {
    let allowlist = cors_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any)
        .allow_origin(if allowlist.is_empty() {
            AllowOrigin::list([HeaderValue::from_static("http://127.0.0.1:3000")])
        } else {
            AllowOrigin::list(allowlist)
        });

    let api_routes = Router::new()
        .route("/system/status", get(system::get_status))
        .route("/system/health", get(system::get_health))
        .route("/system/plugins", get(system::list_plugins))
        .route("/system/plugins/install", post(system::install_plugin))
        .route("/system/plugins/doctor", get(system::plugin_doctor))
        .route("/system/plugins/trust", post(system::trust_plugin_key))
        .route("/system/plugins/:id", get(system::get_plugin))
        .route("/system/plugins/:id/enable", post(system::enable_plugin))
        .route("/system/plugins/:id/disable", post(system::disable_plugin))
        .route("/system/plugins/:id/reload", post(system::reload_plugin))
        .route("/system/plugins/:id/sign", post(system::sign_plugin))
        .route("/system/plugins/:id/logs", get(system::plugin_logs))
        .route("/system/plugins/:id", delete(system::uninstall_plugin))
        .route("/agents", get(agents::list_agents))
        .route("/agents/:id", get(agents::get_agent))
        .route("/agents/:id/status", get(agents::get_agent_status))
        .route("/agents/:id/skills", get(agents::get_agent_skills))
        .route("/messages/conversations", get(messages::list_conversations))
        .route(
            "/messages/conversations",
            post(messages::create_conversation),
        )
        .route(
            "/messages/conversations/:id",
            get(messages::get_conversation),
        )
        .route(
            "/messages/conversations/:id/messages",
            get(messages::list_messages),
        )
        .route("/messages", post(messages::send_message))
        .route("/messages/poll", get(messages::poll_updates))
        .route("/org/company", get(org::get_company))
        .route("/org/tree", get(org::get_org_tree))
        .route("/org/agents", get(org::list_org_agents));

    Router::new()
        .nest("/api", api_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[derive(Debug, Clone)]
pub struct ApiServerConfig {
    /// Host to bind to
    pub host: String,

    /// Port to bind to
    pub port: u16,

    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            cors_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:5173".to_string(),
            ],
        }
    }
}

/// API Server
pub struct ApiServer {
    config: ApiServerConfig,
    state: ApiState,
    shutdown_notify: Arc<Notify>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(config: ApiServerConfig, state: ApiState) -> Self {
        Self {
            config,
            state,
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    pub fn shutdown_notifier(&self) -> Arc<Notify> {
        Arc::clone(&self.shutdown_notify)
    }

    pub async fn run(self) -> anyhow::Result<()> {
        self.run_with_shutdown(async {
            std::future::pending::<()>().await;
        })
        .await
    }

    pub async fn run_with_shutdown<F>(self, shutdown: F) -> anyhow::Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port).parse()?;
        let listener = TcpListener::bind(addr).await?;
        let router = build_router(self.state, &self.config.cors_origins);
        let notify = Arc::clone(&self.shutdown_notify);

        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                tokio::select! {
                    _ = shutdown => {},
                    _ = notify.notified() => {},
                }
            })
            .await?;

        Ok(())
    }
}
