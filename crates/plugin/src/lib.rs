//! ClawLegion Plugin System
//!
//! Provides plugin registry, loading, and lifecycle management.

mod bridge;
mod capability;
mod loader;
mod manager;
mod manifest;
mod protocol;
mod registry;
mod runtime;
mod signature;

pub use bridge::*;
pub use capability::*;
pub use loader::*;
pub use manager::*;
pub use manifest::*;
pub use protocol::*;
pub use registry::*;
pub use runtime::*;
pub use signature::*;

/// Re-export core plugin types
pub use clawlegion_core::{
    Plugin, PluginCapabilityDescriptor, PluginCapabilityKind, PluginContext, PluginDependency,
    PluginHealthcheck, PluginInfo, PluginManifest, PluginMetadata, PluginPermission,
    PluginPermissionScope, PluginRuntimeFamily, PluginSignature, PluginState, PluginStatus,
    PluginType, PluginUiMetadata,
};
