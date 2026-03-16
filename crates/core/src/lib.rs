//! ClawLegion Core
//!
//! Core traits and types for the ClawLegion Multi-Agent System.
//! This crate provides the foundational abstractions used throughout the system.

mod agent;
mod config;
mod error;
mod llm;
mod message;
mod plugin;
mod storage;

pub use agent::*;
pub use config::*;
pub use error::*;
pub use llm::*;
pub use message::*;
pub use plugin::*;
pub use storage::*;

pub use chrono::{DateTime, Utc};
/// Re-export commonly used types
pub use uuid::Uuid;
