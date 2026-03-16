use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::manifest::{discover_manifests, DiscoveredPlugin};
use crate::signature::SignatureVerifier;
use clawlegion_core::{PluginManifest, PluginType};

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub manifest_path: PathBuf,
    pub entrypoint_path: PathBuf,
    pub plugin_type: PluginType,
}

#[derive(Debug, Clone)]
pub struct PluginLoadConfig {
    pub plugin_dir: PathBuf,
    pub verify_signatures: bool,
    pub public_key: Option<Vec<u8>>,
    pub trust_private_plugins: bool,
}

impl PluginLoadConfig {
    pub fn new(plugin_dir: PathBuf) -> Self {
        Self {
            plugin_dir,
            verify_signatures: true,
            public_key: None,
            trust_private_plugins: false,
        }
    }

    pub fn with_signature_verification(mut self, public_key: Vec<u8>) -> Self {
        self.verify_signatures = true;
        self.public_key = Some(public_key);
        self
    }

    pub fn without_signature_verification(mut self) -> Self {
        self.verify_signatures = false;
        self.public_key = None;
        self
    }
}

impl Default for PluginLoadConfig {
    fn default() -> Self {
        Self::new(PathBuf::from("./plugins"))
    }
}

pub struct DynamicPluginLoader {
    search_paths: Vec<PathBuf>,
}

impl DynamicPluginLoader {
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self { search_paths }
    }

    pub fn with_defaults() -> Self {
        let mut search_paths = vec![
            PathBuf::from("./plugins"),
            PathBuf::from("./target/plugins"),
        ];
        if let Some(home) = dirs::home_dir() {
            search_paths.push(home.join(".clawlegion/plugins"));
        }
        search_paths.push(PathBuf::from("/usr/local/lib/clawlegion/plugins"));
        Self { search_paths }
    }

    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    pub fn discover(&self) -> Result<Vec<DiscoveredPlugin>> {
        let mut all = Vec::new();
        for path in &self.search_paths {
            all.extend(discover_manifests(path)?);
        }
        let mut dedup = HashMap::new();
        for plugin in all {
            dedup.insert(plugin.manifest.id.clone(), plugin);
        }
        let mut plugins: Vec<_> = dedup.into_values().collect();
        plugins.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
        Ok(plugins)
    }

    pub fn find_plugin(&self, name: &str) -> Option<PathBuf> {
        self.search_paths
            .iter()
            .map(|root| root.join(name))
            .find(|candidate| candidate.exists())
    }

    pub fn load(
        &self,
        plugin_path: &Path,
        verifier: Option<&SignatureVerifier>,
    ) -> Result<LoadedPlugin> {
        let manifest_path = if plugin_path.is_dir() {
            plugin_path.join("plugin.toml")
        } else if plugin_path.file_name().and_then(|name| name.to_str()) == Some("plugin.toml") {
            plugin_path.to_path_buf()
        } else {
            return Err(anyhow!(
                "plugin path {} must be a directory or plugin.toml",
                plugin_path.display()
            ));
        };

        let discovered = DiscoveredPlugin::load(&manifest_path)?;
        if let Some(verifier) = verifier {
            if discovered.manifest.signature.is_some() {
                let verified = verifier
                    .verify_plugin_file(&discovered.entrypoint_path)
                    .with_context(|| {
                        format!(
                            "signature verification failed for {}",
                            discovered.manifest.id
                        )
                    })?;
                if !verified {
                    return Err(anyhow!(
                        "signature verification failed for {}",
                        discovered.manifest.id
                    ));
                }
            }
        }

        Ok(LoadedPlugin {
            plugin_type: discovered.manifest.runtime.clone(),
            manifest: discovered.manifest,
            manifest_path,
            entrypoint_path: discovered.entrypoint_path,
        })
    }

    pub fn plugin_extension() -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "dylib"
        }
        #[cfg(target_os = "linux")]
        {
            "so"
        }
        #[cfg(target_os = "windows")]
        {
            "dll"
        }
    }

    pub fn plugin_filename(name: &str) -> String {
        #[cfg(target_os = "windows")]
        {
            format!("{}.{}", name.replace('-', "_"), Self::plugin_extension())
        }
        #[cfg(not(target_os = "windows"))]
        {
            format!("lib{}.{}", name.replace('-', "_"), Self::plugin_extension())
        }
    }
}
