//! Skill Installer - downloads, installs, and uninstalls skills
//!
//! This module provides skill installation functionality:
//! - Download skills from marketplace URLs
//! - Install from GitHub repositories
//! - Install from local ZIP files
//! - Uninstall skills
//! - List installed skills

use clawlegion_core::Result;
use reqwest::Client;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{error, info, warn};
use zip::read::ZipArchive;

use crate::skill::config::RawSkillConfig;
use crate::skill::marketplace::MarketplaceClient;
use crate::skill::metadata::SkillMetadata;

/// Skill installation status
#[derive(Debug, Clone)]
pub struct InstallationStatus {
    /// Skill name
    pub skill_name: String,
    /// Installation path
    pub install_path: PathBuf,
    /// Whether installation was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Whether skill was already installed (updated)
    pub updated: bool,
}

/// Skill Installer configuration
#[derive(Debug, Clone)]
pub struct InstallerConfig {
    /// Base directory for skills (~/.clawlegion/skills/)
    pub skills_dir: PathBuf,
    /// HTTP request timeout (seconds)
    pub timeout_secs: u64,
    /// Temporary directory for downloads
    pub temp_dir: Option<PathBuf>,
}

impl Default for InstallerConfig {
    fn default() -> Self {
        let skills_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".clawlegion")
            .join("skills");

        Self {
            skills_dir,
            timeout_secs: 60,
            temp_dir: None,
        }
    }
}

/// Skill Installer
///
/// Handles downloading, extracting, and installing skills from various sources.
pub struct SkillInstaller {
    config: InstallerConfig,
    http_client: Client,
    marketplace: Option<MarketplaceClient>,
}

impl SkillInstaller {
    /// Create a new skill installer
    pub fn new(config: InstallerConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to create HTTP client: {}", e),
                ))
            })?;

        // Ensure skills directory exists
        if !config.skills_dir.exists() {
            fs::create_dir_all(&config.skills_dir).map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to create skills directory: {}", e),
                ))
            })?;
        }

        Ok(Self {
            config,
            http_client,
            marketplace: None,
        })
    }

    /// Create with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(InstallerConfig::default())
    }

    /// Set the marketplace client for marketplace-based installations
    pub fn with_marketplace(mut self, marketplace: MarketplaceClient) -> Self {
        self.marketplace = Some(marketplace);
        self
    }

    /// Install a skill from a marketplace
    pub async fn install_from_marketplace(&self, skill_id: &str) -> Result<InstallationStatus> {
        info!("Installing skill from marketplace: {}", skill_id);

        // Get skill info from marketplace
        let marketplace = self.marketplace.as_ref().ok_or_else(|| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                "Marketplace client not configured".to_string(),
            ))
        })?;

        let skill = marketplace.get_skill(skill_id).await?;

        // Download the skill
        let zip_path = self.download_to_temp(&skill.download_url).await?;

        // Extract and install
        let install_path = self.config.skills_dir.join(&skill.name);
        let status = self
            .extract_and_install(&zip_path, &install_path, &skill.name)
            .await?;

        // Clean up temp file
        let _ = fs::remove_file(&zip_path);

        Ok(status)
    }

    /// Install a skill from a local ZIP file
    pub async fn install_from_zip(&self, zip_path: &Path) -> Result<InstallationStatus> {
        info!("Installing skill from ZIP: {:?}", zip_path);

        if !zip_path.exists() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "ZIP file not found: {:?}",
                    zip_path
                )),
            ));
        }

        // Read skill name from ZIP or use filename
        let skill_name = self
            .extract_skill_name_from_zip(zip_path)
            .unwrap_or_else(|| {
                zip_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown-skill")
                    .to_string()
            });

        let install_path = self.config.skills_dir.join(&skill_name);

        self.extract_and_install(zip_path, &install_path, &skill_name)
            .await
    }

    /// Install a skill from a directory (for local development)
    pub async fn install_from_directory(
        &self,
        source_dir: &Path,
        skill_name: Option<&str>,
    ) -> Result<InstallationStatus> {
        info!("Installing skill from directory: {:?}", source_dir);

        if !source_dir.exists() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Directory not found: {:?}",
                    source_dir
                )),
            ));
        }

        // Determine skill name
        let skill_name = skill_name
            .map(|s| s.to_string())
            .or_else(|| {
                source_dir
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown-skill".to_string());

        let install_path = self.config.skills_dir.join(&skill_name);

        // Copy directory to skills folder
        if install_path.exists() {
            fs::remove_dir_all(&install_path).ok();
        }

        self.copy_directory(source_dir, &install_path)?;

        // Validate skill.toml exists
        let skill_toml = install_path.join("skill.toml");
        if !skill_toml.exists() {
            warn!("No skill.toml found in {:?}", install_path);
        }

        Ok(InstallationStatus {
            skill_name: skill_name.clone(),
            install_path,
            success: true,
            error: None,
            updated: false,
        })
    }

    /// Uninstall a skill
    pub async fn uninstall(&self, skill_name: &str) -> Result<InstallationStatus> {
        info!("Uninstalling skill: {}", skill_name);

        let install_path = self.config.skills_dir.join(skill_name);

        if !install_path.exists() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Skill '{}' not found at {:?}",
                    skill_name, install_path
                )),
            ));
        }

        fs::remove_dir_all(&install_path).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to remove skill directory: {}",
                e
            )))
        })?;

        info!("Successfully uninstalled skill: {}", skill_name);

        Ok(InstallationStatus {
            skill_name: skill_name.to_string(),
            install_path,
            success: true,
            error: None,
            updated: false,
        })
    }

    /// List all installed skills
    pub fn list_installed(&self) -> Vec<SkillMetadata> {
        let mut skills = Vec::new();

        if !self.config.skills_dir.exists() {
            return skills;
        }

        let entries = match fs::read_dir(&self.config.skills_dir) {
            Ok(entries) => entries,
            Err(e) => {
                error!("Failed to read skills directory: {}", e);
                return skills;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(metadata) = self.read_skill_metadata(&path) {
                    skills.push(metadata);
                }
            }
        }

        skills
    }

    /// Check if a skill is installed
    pub fn is_installed(&self, skill_name: &str) -> bool {
        let install_path = self.config.skills_dir.join(skill_name);
        install_path.exists()
    }

    /// Get the skills directory path
    pub fn skills_dir(&self) -> &Path {
        &self.config.skills_dir
    }

    /// Download a file to a temporary location
    async fn download_to_temp(&self, url: &str) -> Result<PathBuf> {
        let temp_dir = self
            .config
            .temp_dir
            .clone()
            .unwrap_or_else(std::env::temp_dir);

        fs::create_dir_all(&temp_dir).ok();

        let file_name = url.split('/').next_back().unwrap_or("skill.zip");

        let temp_path = temp_dir.join(file_name);

        info!("Downloading to: {:?}", temp_path);

        let response = self.http_client.get(url).send().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Download failed: {}",
                e
            )))
        })?;

        if !response.status().is_success() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Download returned status {}",
                    response.status()
                )),
            ));
        }

        let bytes = response.bytes().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to read response: {}",
                e
            )))
        })?;

        let mut file = File::create(&temp_path).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to create temp file: {}",
                e
            )))
        })?;

        file.write_all(&bytes).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to write temp file: {}",
                e
            )))
        })?;

        Ok(temp_path)
    }

    /// Extract ZIP and install skill
    async fn extract_and_install(
        &self,
        zip_path: &Path,
        install_path: &Path,
        skill_name: &str,
    ) -> Result<InstallationStatus> {
        info!("Extracting skill to: {:?}", install_path);

        // Remove existing installation
        if install_path.exists() {
            fs::remove_dir_all(install_path).ok();
        }

        fs::create_dir_all(install_path).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to create install directory: {}",
                e
            )))
        })?;

        // Extract ZIP
        let zip_file = File::open(zip_path).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to open ZIP: {}",
                e
            )))
        })?;

        let mut archive = ZipArchive::new(zip_file).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Invalid ZIP archive: {}",
                e
            )))
        })?;

        // Handle GitHub-style ZIP (extracts to repo-branch/)
        let mut prefix_to_strip: Option<String> = None;
        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to read ZIP entry: {}", e),
                ))
            })?;

            let entry_path = file.name().to_string();

            // Detect prefix from first entry
            if i == 0 {
                if let Some(pos) = entry_path.find('/') {
                    let prefix = &entry_path[..pos];
                    if prefix != skill_name {
                        prefix_to_strip = Some(prefix.to_string());
                    }
                }
            }

            // Strip prefix if detected
            let relative_path = if let Some(ref prefix) = prefix_to_strip {
                entry_path
                    .strip_prefix(prefix)
                    .map(|s| s.trim_start_matches('/'))
                    .unwrap_or(&entry_path)
            } else {
                &entry_path
            };

            let out_path = install_path.join(relative_path);

            if file.is_dir() {
                fs::create_dir_all(&out_path).ok();
            } else {
                // Create parent directories
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent).ok();
                }

                let mut outfile = File::create(&out_path).map_err(|e| {
                    clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                        format!("Failed to create file: {}", e),
                    ))
                })?;

                io::copy(&mut file.take(100_000_000), &mut outfile).map_err(|e| {
                    clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                        format!("Failed to extract file: {}", e),
                    ))
                })?;
            }
        }

        // Validate installation
        let skill_toml = install_path.join("skill.toml");
        if !skill_toml.exists() {
            warn!("No skill.toml found after installation");
        }

        Ok(InstallationStatus {
            skill_name: skill_name.to_string(),
            install_path: install_path.to_path_buf(),
            success: true,
            error: None,
            updated: false,
        })
    }

    /// Read skill metadata from an installed skill
    pub fn read_skill_metadata(&self, install_path: &Path) -> Result<SkillMetadata> {
        let skill_toml = install_path.join("skill.toml");

        if !skill_toml.exists() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "skill.toml not found at {:?}",
                    install_path
                )),
            ));
        }

        let content = fs::read_to_string(&skill_toml).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to read skill.toml: {}",
                e
            )))
        })?;

        let config: RawSkillConfig = toml::from_str(&content).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to parse skill.toml: {}",
                e
            )))
        })?;

        let config_path = Some(
            install_path
                .join("skill.toml")
                .to_string_lossy()
                .to_string(),
        );
        Ok(config.to_metadata(config_path))
    }

    /// Extract skill name from ZIP file
    fn extract_skill_name_from_zip(&self, zip_path: &Path) -> Option<String> {
        let zip_file = File::open(zip_path).ok()?;
        let mut archive = ZipArchive::new(zip_file).ok()?;

        // Look for skill.toml in the archive
        for i in 0..archive.len() {
            let file = archive.by_index(i).ok()?;
            let name = file.name();

            if name.ends_with("skill.toml") {
                // Extract directory name from path
                let parts: Vec<&str> = name.split('/').collect();
                if parts.len() > 1 {
                    return Some(parts[0].to_string());
                }
            }
        }

        None
    }

    /// Copy a directory recursively
    fn copy_directory(&self, src: &Path, dst: &Path) -> Result<()> {
        if !src.exists() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Source directory not found: {:?}",
                    src
                )),
            ));
        }

        fs::create_dir_all(dst).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to create destination: {}",
                e
            )))
        })?;

        for entry in fs::read_dir(src).map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to read source directory: {}",
                e
            )))
        })? {
            let entry = entry.map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to read directory entry: {}", e),
                ))
            })?;

            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_directory(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path).map_err(|e| {
                    clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                        format!("Failed to copy file: {}", e),
                    ))
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installer_config_default() {
        let config = InstallerConfig::default();
        assert!(config.skills_dir.ends_with(".clawlegion/skills"));
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_installer_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = InstallerConfig {
            skills_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let installer = SkillInstaller::new(config).unwrap();
        assert!(installer.skills_dir().exists());
    }
}
