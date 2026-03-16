use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use clawlegion_core::{PluginManifest, PluginType};

pub const MANIFEST_FILE_NAME: &str = "plugin.toml";
pub const REQUIRED_PLUGIN_API_VERSION: &str = "v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPlugin {
    pub manifest: PluginManifest,
    pub manifest_path: PathBuf,
    pub plugin_dir: PathBuf,
    pub entrypoint_path: PathBuf,
}

impl DiscoveredPlugin {
    pub fn load(manifest_path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(manifest_path)
            .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
        let manifest: PluginManifest = toml::from_str(&raw)
            .with_context(|| format!("failed to parse manifest {}", manifest_path.display()))?;
        if manifest.api_version != REQUIRED_PLUGIN_API_VERSION {
            return Err(anyhow!(
                "plugin {} requires api_version {}, got {}",
                manifest.id,
                REQUIRED_PLUGIN_API_VERSION,
                manifest.api_version
            ));
        }
        let plugin_dir = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let entrypoint_path = match manifest.runtime {
            PluginType::Remote => PathBuf::from(&manifest.entrypoint),
            _ => plugin_dir.join(&manifest.entrypoint),
        };

        Ok(Self {
            manifest,
            manifest_path: manifest_path.to_path_buf(),
            plugin_dir,
            entrypoint_path,
        })
    }
}

pub fn discover_manifests(root: &Path) -> Result<Vec<DiscoveredPlugin>> {
    let mut manifests = Vec::new();
    if !root.exists() {
        return Ok(manifests);
    }

    for entry in fs::read_dir(root)
        .with_context(|| format!("failed to read plugin root {}", root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let manifest_path = path.join(MANIFEST_FILE_NAME);
            if manifest_path.exists() {
                manifests.push(DiscoveredPlugin::load(&manifest_path)?);
            }
        } else if path.file_name().and_then(|name| name.to_str()) == Some(MANIFEST_FILE_NAME) {
            manifests.push(DiscoveredPlugin::load(&path)?);
        }
    }

    manifests.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
    Ok(manifests)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn discovers_manifest_from_plugin_directory() {
        let root =
            std::env::temp_dir().join(format!("clawlegion-manifest-{}", uuid::Uuid::new_v4()));
        let plugin_dir = root.join("demo-plugin");
        fs::create_dir_all(&plugin_dir).expect("create plugin dir");
        fs::write(
            plugin_dir.join("plugin.toml"),
            r#"
id = "demo-plugin"
version = "0.1.0"
api_version = "v2"
runtime = "config"
entrypoint = "plugin.toml"

[metadata]
name = "demo-plugin"
version = "0.1.0"
description = "demo"
author = "test"
core_version = "0.1.0"
dependencies = []
tags = []
"#,
        )
        .expect("write manifest");

        let manifests = discover_manifests(&root).expect("discover manifests");
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].manifest.id, "demo-plugin");

        fs::remove_dir_all(&root).expect("cleanup");
    }
}
