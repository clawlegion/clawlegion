//! ClawLegion Sentinel
//!
//! Watchdog system for monitoring and waking up agents.

mod builtin_triggers;
mod manager;
mod trigger;
mod watcher;

pub use builtin_triggers::*;
pub use manager::*;
pub use trigger::*;
pub use watcher::*;

// Re-export from trigger
pub use crate::trigger::CustomConditionHandler;
