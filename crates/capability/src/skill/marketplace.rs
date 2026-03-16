//! Skill Marketplace - browse and search skills from remote marketplaces
//!
//! This module provides marketplace client functionality for:
//! - Browsing available skills from remote marketplaces
//! - Searching skills by keyword, category, or tags
//! - Fetching skill metadata and download URLs

use clawlegion_core::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Marketplace skill metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSkill {
    /// Unique skill identifier
    pub id: String,
    /// Skill name
    pub name: String,
    /// Skill version
    pub version: String,
    /// Skill description
    pub description: String,
    /// Author name
    pub author: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Download URL (ZIP archive)
    pub download_url: String,
    /// Category (e.g., "development", "data", "automation")
    pub category: Option<String>,
    /// Tags for search and filtering
    pub tags: Vec<String>,
    /// Required tools for this skill
    pub required_tools: Option<Vec<String>>,
    /// Required MCPs for this skill
    pub required_mcps: Option<Vec<String>>,
    /// Minimum ClawLegion version required
    pub min_version: Option<String>,
    /// Download count
    pub downloads: Option<u64>,
    /// Rating (0.0 - 5.0)
    pub rating: Option<f64>,
}

/// Marketplace search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSearchResponse {
    /// Total number of results
    pub total: u64,
    /// Page number
    pub page: u64,
    /// Page size
    pub page_size: u64,
    /// Search results
    pub skills: Vec<MarketplaceSkill>,
}

/// Marketplace categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketplaceCategory {
    Development,
    Data,
    Automation,
    Communication,
    Productivity,
    Entertainment,
    Utilities,
    Other,
}

impl MarketplaceCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Data => "data",
            Self::Automation => "automation",
            Self::Communication => "communication",
            Self::Productivity => "productivity",
            Self::Entertainment => "entertainment",
            Self::Utilities => "utilities",
            Self::Other => "other",
        }
    }
}

/// Marketplace client configuration
#[derive(Debug, Clone)]
pub struct MarketplaceConfig {
    /// Base URL of the marketplace API
    pub base_url: String,
    /// HTTP request timeout (seconds)
    pub timeout_secs: u64,
    /// API key for authenticated marketplaces (optional)
    pub api_key: Option<String>,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            // Default to Claude Smithery-like API endpoint
            base_url: "https://api.claude-skills.example.com/v1".to_string(),
            timeout_secs: 30,
            api_key: None,
        }
    }
}

/// Skill Marketplace Client
///
/// Provides methods to browse, search, and fetch skills from remote marketplaces.
pub struct MarketplaceClient {
    config: MarketplaceConfig,
    http_client: Client,
}

impl MarketplaceClient {
    /// Create a new marketplace client
    pub fn new(config: MarketplaceConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to create HTTP client: {}", e),
                ))
            })?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Create with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(MarketplaceConfig::default())
    }

    /// List all available skills from the marketplace
    pub async fn list(&self) -> Result<MarketplaceSearchResponse> {
        self.search_internal(None, None, None, 1, 50).await
    }

    /// Search skills by keyword
    pub async fn search(&self, query: &str) -> Result<MarketplaceSearchResponse> {
        self.search_internal(Some(query), None, None, 1, 50).await
    }

    /// Search skills by category
    pub async fn search_by_category(&self, category: &str) -> Result<MarketplaceSearchResponse> {
        self.search_internal(None, Some(category), None, 1, 50)
            .await
    }

    /// Search skills by tag
    pub async fn search_by_tag(&self, tag: &str) -> Result<MarketplaceSearchResponse> {
        self.search_internal(None, None, Some(tag), 1, 50).await
    }

    /// Get detailed information about a specific skill
    pub async fn get_skill(&self, skill_id: &str) -> Result<MarketplaceSkill> {
        let url = format!("{}/skills/{}", self.config.base_url, skill_id);

        let mut request = self.http_client.get(&url);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to fetch skill '{}': {}",
                skill_id, e
            )))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Marketplace API returned status {}: {}",
                    status, body
                )),
            ));
        }

        let skill = response.json::<MarketplaceSkill>().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to parse skill response: {}",
                e
            )))
        })?;

        Ok(skill)
    }

    /// Get the download URL for a skill
    pub async fn get_download_url(&self, skill_id: &str) -> Result<String> {
        let skill = self.get_skill(skill_id).await?;
        Ok(skill.download_url)
    }

    /// Internal search implementation
    async fn search_internal(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tag: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<MarketplaceSearchResponse> {
        let mut url = format!("{}/skills", self.config.base_url);
        let mut params = Vec::new();

        if let Some(q) = query {
            params.push(("q", q.to_string()));
        }
        if let Some(cat) = category {
            params.push(("category", cat.to_string()));
        }
        if let Some(t) = tag {
            params.push(("tag", t.to_string()));
        }

        params.push(("page", page.to_string()));
        params.push(("page_size", page_size.to_string()));

        if !params.is_empty() {
            url.push('?');
            let query_string = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            url.push_str(&query_string);
        }

        info!("Searching marketplace: {}", url);

        let mut request = self.http_client.get(&url);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Marketplace search failed: {}",
                e
            )))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Marketplace API returned status {}: {}",
                    status, body
                )),
            ));
        }

        let result = response
            .json::<MarketplaceSearchResponse>()
            .await
            .map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to parse search response: {}", e),
                ))
            })?;

        Ok(result)
    }

    /// List available categories
    pub async fn list_categories(&self) -> Result<Vec<String>> {
        let url = format!("{}/categories", self.config.base_url);

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to fetch categories: {}",
                e
            )))
        })?;

        if !response.status().is_success() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Marketplace API returned status {}",
                    response.status()
                )),
            ));
        }

        let result = response.json::<Vec<String>>().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to parse categories: {}",
                e
            )))
        })?;

        Ok(result)
    }

    /// List available tags
    pub async fn list_tags(&self) -> Result<Vec<String>> {
        let url = format!("{}/tags", self.config.base_url);

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to fetch tags: {}",
                e
            )))
        })?;

        if !response.status().is_success() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Marketplace API returned status {}",
                    response.status()
                )),
            ));
        }

        let result = response.json::<Vec<String>>().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to parse tags: {}",
                e
            )))
        })?;

        Ok(result)
    }

    /// Get featured/trending skills
    pub async fn get_featured(&self) -> Result<Vec<MarketplaceSkill>> {
        let url = format!("{}/featured", self.config.base_url);

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(format!(
                "Failed to fetch featured skills: {}",
                e
            )))
        })?;

        if !response.status().is_success() {
            return Err(clawlegion_core::Error::Capability(
                clawlegion_core::CapabilityError::NotFound(format!(
                    "Marketplace API returned status {}",
                    response.status()
                )),
            ));
        }

        let result = response
            .json::<Vec<MarketplaceSkill>>()
            .await
            .map_err(|e| {
                clawlegion_core::Error::Capability(clawlegion_core::CapabilityError::NotFound(
                    format!("Failed to parse featured skills: {}", e),
                ))
            })?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_config_default() {
        let config = MarketplaceConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_marketplace_category_strings() {
        assert_eq!(MarketplaceCategory::Development.as_str(), "development");
        assert_eq!(MarketplaceCategory::Data.as_str(), "data");
        assert_eq!(MarketplaceCategory::Automation.as_str(), "automation");
    }

    #[test]
    fn test_parse_skill_toml() {
        let toml_content = r#"
[skill]
name = "test-skill"
version = "1.0.0"
description = "A test skill"
author = "Test Author"
"#;

        let parsed: toml::Value = toml_content.parse().unwrap();
        let skill_table = parsed.get("skill").unwrap().as_table().unwrap();

        let name = skill_table.get("name").unwrap().as_str().unwrap();
        let version = skill_table.get("version").unwrap().as_str().unwrap();
        let description = skill_table.get("description").unwrap().as_str().unwrap();
        let author = skill_table.get("author").unwrap().as_str().unwrap();

        assert_eq!(name, "test-skill");
        assert_eq!(version, "1.0.0");
        assert_eq!(description, "A test skill");
        assert_eq!(author, "Test Author");
    }
}
