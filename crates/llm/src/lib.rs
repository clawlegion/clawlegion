//! ClawLegion LLM Layer
//!
//! LLM provider abstraction with plugin support.

mod client;
mod registry;

pub use client::*;
pub use registry::*;
