use std::collections::HashMap;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use clawlegion_core::PluginCapabilityDescriptor;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisteredCapability {
    pub plugin_id: String,
    pub capability: PluginCapabilityDescriptor,
}

#[derive(Debug, Default)]
pub struct CapabilityRegistry {
    by_plugin: DashMap<String, Vec<PluginCapabilityDescriptor>>,
    by_capability_id: DashMap<String, RegisteredCapability>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_plugin_capabilities(
        &self,
        plugin_id: &str,
        capabilities: &[PluginCapabilityDescriptor],
    ) {
        let capabilities = capabilities.to_vec();
        for capability in &capabilities {
            self.by_capability_id.insert(
                capability.id.clone(),
                RegisteredCapability {
                    plugin_id: plugin_id.to_string(),
                    capability: capability.clone(),
                },
            );
        }
        self.by_plugin.insert(plugin_id.to_string(), capabilities);
    }

    pub fn unregister_plugin(&self, plugin_id: &str) {
        if let Some((_, capabilities)) = self.by_plugin.remove(plugin_id) {
            for capability in capabilities {
                self.by_capability_id.remove(&capability.id);
            }
        }
    }

    pub fn list_for_plugin(&self, plugin_id: &str) -> Vec<PluginCapabilityDescriptor> {
        self.by_plugin
            .get(plugin_id)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }

    pub fn list_all(&self) -> Vec<RegisteredCapability> {
        self.by_capability_id
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn snapshot_by_kind(&self) -> HashMap<String, Vec<String>> {
        let mut snapshot: HashMap<String, Vec<String>> = HashMap::new();
        for entry in self.by_capability_id.iter() {
            let registered = entry.value();
            snapshot
                .entry(format!("{:?}", registered.capability.kind))
                .or_default()
                .push(registered.plugin_id.clone());
        }
        for plugin_ids in snapshot.values_mut() {
            plugin_ids.sort();
            plugin_ids.dedup();
        }
        snapshot
    }
}
