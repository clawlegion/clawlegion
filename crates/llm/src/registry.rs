//! LLM Provider Registry

use clawlegion_core::LlmProviderConfig;
use clawlegion_core::{Error, LlmError, LlmProvider, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Provider factory function type
pub type ProviderFactory = dyn Fn(&LlmProviderConfig) -> Result<Arc<dyn LlmProvider>> + Send + Sync;

/// LLM Provider Registry
///
/// Central registry for all LLM providers in the system.
pub struct LlmRegistry {
    /// Registered providers
    providers: RwLock<HashMap<String, Arc<dyn LlmProvider>>>,

    /// Default provider name
    default_provider: RwLock<Option<String>>,

    /// Provider configurations
    configs: RwLock<HashMap<String, LlmProviderConfig>>,

    /// Provider factories keyed by provider type (e.g., "openai", "anthropic")
    provider_factories: RwLock<HashMap<String, Arc<ProviderFactory>>>,
}

impl LlmRegistry {
    /// Create a new LLM registry
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            default_provider: RwLock::new(None),
            configs: RwLock::new(HashMap::new()),
            provider_factories: RwLock::new(HashMap::new()),
        }
    }

    /// Register a provider factory for a provider type
    ///
    /// This allows dynamic registration of provider implementations
    /// without hardcoding them in the registry.
    pub fn register_provider_factory<F>(&self, provider_type: &str, factory: F) -> Result<()>
    where
        F: Fn(&LlmProviderConfig) -> Result<Arc<dyn LlmProvider>> + Send + Sync + 'static,
    {
        self.provider_factories
            .write()
            .insert(provider_type.to_string(), Arc::new(factory));
        Ok(())
    }

    /// Get a provider factory for a provider type
    pub fn get_provider_factory(&self, provider_type: &str) -> Option<Arc<ProviderFactory>> {
        self.provider_factories.read().get(provider_type).cloned()
    }

    /// List all registered provider types
    pub fn list_provider_types(&self) -> Vec<String> {
        self.provider_factories.read().keys().cloned().collect()
    }

    /// Register an LLM provider
    pub fn register(&self, name: String, provider: Arc<dyn LlmProvider>) -> Result<()> {
        if self.providers.read().contains_key(&name) {
            return Err(Error::Llm(LlmError::ProviderNotFound(format!(
                "Provider '{}' already registered",
                name
            ))));
        }

        self.providers.write().insert(name.clone(), provider);
        Ok(())
    }

    /// Register a provider from configuration
    ///
    /// This method uses registered provider factories to create provider instances.
    /// Make sure to register the provider factory using `register_provider_factory` first.
    pub fn register_from_config(&self, name: String, config: &LlmProviderConfig) -> Result<()> {
        let provider_type = config.provider.as_str();

        // Get the provider factory for this provider type
        let factory = self.get_provider_factory(provider_type).ok_or_else(|| {
            Error::Llm(LlmError::ProviderNotFound(format!(
                "Unknown provider type: '{}'. Make sure to register the provider factory.",
                provider_type
            )))
        })?;

        // Create the provider instance using the factory
        let provider = factory(config)?;

        self.register(name.clone(), provider)?;
        self.configs.write().insert(name, config.clone());

        Ok(())
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn LlmProvider>> {
        self.providers.read().get(name).cloned()
    }

    /// Get the default provider
    pub fn get_default(&self) -> Option<Arc<dyn LlmProvider>> {
        self.default_provider
            .read()
            .as_ref()
            .and_then(|name| self.get(name))
    }

    /// Set the default provider
    pub fn set_default(&self, name: &str) -> Result<()> {
        if !self.providers.read().contains_key(name) {
            return Err(Error::Llm(LlmError::ProviderNotFound(format!(
                "Provider '{}' not found",
                name
            ))));
        }

        *self.default_provider.write() = Some(name.to_string());
        Ok(())
    }

    /// List all registered providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.read().keys().cloned().collect()
    }

    /// List all provider configurations
    pub fn list_configs(&self) -> Vec<LlmProviderConfig> {
        self.configs.read().values().cloned().collect()
    }

    /// Unregister a provider
    pub fn unregister(&self, name: &str) -> Result<()> {
        self.providers
            .write()
            .remove(name)
            .ok_or_else(|| Error::Llm(LlmError::ProviderNotFound(name.to_string())))?;

        self.configs.write().remove(name);

        // Clear default if this was the default provider
        if self.default_provider.read().as_ref().map(|s| s.as_str()) == Some(name) {
            *self.default_provider.write() = None;
        }

        Ok(())
    }

    /// Check if a provider is registered
    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.read().contains_key(name)
    }

    /// Get a provider or the default provider
    pub fn get_or_default(&self, name: Option<&str>) -> Option<Arc<dyn LlmProvider>> {
        if let Some(name) = name {
            self.get(name)
        } else {
            self.get_default()
        }
    }
}

impl Default for LlmRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared LLM registry type
pub type SharedLlmRegistry = Arc<LlmRegistry>;
