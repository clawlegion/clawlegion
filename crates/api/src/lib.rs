//! HTTP API server for the ClawLegion multi-agent system.

pub mod dto;
pub mod routes;
pub mod server;
pub mod services;
pub mod state;

pub use server::{build_router, ApiServer, ApiServerConfig};
pub use services::application_service::{AppServices, SystemSnapshot};
pub use state::ApiState;
