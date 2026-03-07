//! Plugin manager for discovery, loading, and lifecycle management
//!
//! This module provides the central plugin management functionality.

use anyhow::{Context, Result, bail};
use glob::Pattern;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::traits::{Plugin, PluginMetadata, PluginType};
use super::stage::PipelineStagePlugin;
use super::CustomStageConfig;

/// Plugin manager handles discovery, loading, and lifecycle of plugins
pub struct PluginManager {
    /// Loaded plugins by ID
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,

    /// Pipeline stage plugins
    stage_plugins: Arc<RwLock<HashMap<String, Arc<dyn PipelineStagePlugin>>>>,

    /// Custom stage configurations loaded from config files
    custom_stages: Arc<RwLock<HashMap<String, CustomStageConfig>>>,

    /// Plugin discovery paths
    plugin_paths: Vec<PathBuf>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        let mut manager = Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            stage_plugins: Arc::new(RwLock::new(HashMap::new())),
            custom_stages: Arc::new(RwLock::new(HashMap::new())),
            plugin_paths: Vec::new(),
        };

        // Add default plugin paths
        manager.add_default_paths();

        manager
    }

    /// Add default plugin discovery paths
    fn add_default_paths(&mut self) {
        // User plugins: ~/.ltmatrix/plugins/
        if let Some(home) = dirs::home_dir() {
            let user_plugins = home.join(".ltmatrix").join("plugins");
            self.plugin_paths.push(user_plugins);
        }

        // Project plugins: ./.ltmatrix/plugins/
        let project_plugins = PathBuf::from(".ltmatrix").join("plugins");
        self.plugin_paths.push(project_plugins);

        // Environment variable path
        if let Ok(path) = std::env::var("LTMATRIX_PLUGIN_PATH") {
            for p in path.split(':') {
                self.plugin_paths.push(PathBuf::from(p));
            }
        }
    }

    /// Add a custom plugin path
    pub fn add_plugin_path(&mut self, path: impl Into<PathBuf>) {
        self.plugin_paths.push(path.into());
    }

    /// Get plugin paths
    pub fn plugin_paths(&self) -> &[PathBuf] {
        &self.plugin_paths
    }

    /// Discover plugins from configured paths
    pub async fn discover_plugins(&self) -> Result<Vec<PluginDiscovery>> {
        let mut discoveries = Vec::new();

        for path in &self.plugin_paths {
            if !path.exists() {
                debug!("Plugin path does not exist: {}", path.display());
                continue;
            }

            let discovered = self.discover_in_path(path).await?;
            discoveries.extend(discovered);
        }

        info!("Discovered {} potential plugins", discoveries.len());
        Ok(discoveries)
    }

    /// Discover plugins in a specific path
    async fn discover_in_path(&self, path: &Path) -> Result<Vec<PluginDiscovery>> {
        let mut discoveries = Vec::new();

        let entries = std::fs::read_dir(path)
            .with_context(|| format!("Failed to read plugin directory: {}", path.display()))?;

        for entry in entries {
            let entry = entry?;
            let plugin_path = entry.path();

            // Check for plugin manifest or shared library
            if let Some(discovery) = self.check_plugin_path(&plugin_path).await? {
                discoveries.push(discovery);
            }
        }

        Ok(discoveries)
    }

    /// Check if a path contains a plugin
    async fn check_plugin_path(&self, path: &Path) -> Result<Option<PluginDiscovery>> {
        // Check for plugin manifest (ltmatrix-plugin.toml)
        let manifest_path = path.join("ltmatrix-plugin.toml");
        if manifest_path.exists() {
            return self.load_manifest(&manifest_path).await.map(Some);
        }

        // Check for shared library
        #[cfg(target_os = "linux")]
        let lib_pattern = "*.so";
        #[cfg(target_os = "macos")]
        let lib_pattern = "*.dylib";
        #[cfg(target_os = "windows")]
        let lib_pattern = "*.dll";

        if let Some(lib_path) = self.find_library(path, lib_pattern)? {
            return Ok(Some(PluginDiscovery {
                path: lib_path,
                plugin_type: PluginType::PipelineStage, // Default assumption
                manifest: None,
            }));
        }

        // Check for WASM module
        let wasm_path = path.join("plugin.wasm");
        if wasm_path.exists() {
            return Ok(Some(PluginDiscovery {
                path: wasm_path,
                plugin_type: PluginType::PipelineStage,
                manifest: None,
            }));
        }

        Ok(None)
    }

    /// Load a plugin manifest
    async fn load_manifest(&self, path: &Path) -> Result<PluginDiscovery> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read plugin manifest: {}", path.display()))?;

        let manifest: PluginManifest = toml::from_str(&content)
            .with_context(|| format!("Failed to parse plugin manifest: {}", path.display()))?;

        let plugin_type = manifest.plugin.plugin_type.clone();

        Ok(PluginDiscovery {
            path: path.parent().unwrap().to_path_buf(),
            plugin_type,
            manifest: Some(manifest),
        })
    }

    /// Find a shared library in a directory
    fn find_library(&self, dir: &Path, pattern: &str) -> Result<Option<PathBuf>> {
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

        let glob_pattern = Pattern::new(pattern)
            .with_context(|| format!("Invalid glob pattern: {}", pattern))?;

        for entry in entries {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if glob_pattern.matches(&file_name_str) {
                return Ok(Some(entry.path()));
            }
        }

        Ok(None)
    }

    /// Register a built-in plugin
    pub async fn register_builtin<P: Plugin + 'static>(&self, plugin: P) -> Result<()> {
        let metadata = plugin.metadata().clone();
        let id = metadata.id.clone();
        let plugin_type = metadata.plugin_type;

        info!("Registering built-in plugin: {}", id);

        let mut plugins = self.plugins.write().await;
        plugins.insert(id.clone(), Box::new(plugin));

        Ok(())
    }

    /// Register a pipeline stage plugin
    pub async fn register_stage_plugin<P: PipelineStagePlugin + 'static>(&self, plugin: P) -> Result<()> {
        let metadata = plugin.metadata().clone();
        let id = metadata.id.clone();
        let stage_id = plugin.config().id.clone();

        info!("Registering stage plugin: {} (stage: {})", id, stage_id);

        // Store in general plugins map
        let mut plugins = self.plugins.write().await;
        plugins.insert(id, Box::new(plugin));

        Ok(())
    }

    /// Register a custom stage from configuration
    pub async fn register_custom_stage(&self, config: CustomStageConfig) -> Result<()> {
        let id = config.id.clone();
        info!("Registering custom stage: {}", id);

        let mut stages = self.custom_stages.write().await;
        stages.insert(id, config);

        Ok(())
    }

    /// Load plugins from configuration
    ///
    /// Note: This method is reserved for future use when the Settings type
    /// is fully implemented. Currently returns Ok(()) as a no-op.
    pub async fn load_from_config(&self, _config: &crate::config::settings::Config) -> Result<()> {
        // TODO: Implement loading custom stages from config when Settings type is ready
        // For now, this is a no-op that allows the plugin system to compile
        Ok(())
    }

    /// Get all registered custom stage configurations
    pub async fn get_custom_stages(&self) -> Vec<CustomStageConfig> {
        let stages = self.custom_stages.read().await;
        stages.values().cloned().collect()
    }

    /// Get a specific custom stage by ID
    pub async fn get_custom_stage(&self, id: &str) -> Option<CustomStageConfig> {
        let stages = self.custom_stages.read().await;
        stages.get(id).cloned()
    }

    /// Load plugins of a specific type
    pub async fn load_plugins(&self, plugin_type: PluginType) -> Result<Vec<String>> {
        let discoveries = self.discover_plugins().await?;

        let mut loaded = Vec::new();
        for discovery in discoveries {
            if discovery.plugin_type == plugin_type {
                // For now, we only support manifest-based plugins
                if let Some(manifest) = &discovery.manifest {
                    info!("Would load plugin: {} from {}", manifest.plugin.id, discovery.path.display());
                    loaded.push(manifest.plugin.id.clone());
                }
            }
        }

        Ok(loaded)
    }

    /// Get a plugin by ID
    pub async fn get_plugin(&self, id: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        // Note: This is a simplified implementation
        // In a full implementation, we'd need to handle the trait object properly
        None
    }

    /// Initialize all plugins
    pub async fn initialize_all(&self) -> Result<()> {
        let mut plugins = self.plugins.write().await;

        for (id, plugin) in plugins.iter_mut() {
            if let Err(e) = plugin.initialize().await {
                warn!("Failed to initialize plugin {}: {}", id, e);
            }
        }

        Ok(())
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&self) -> Result<()> {
        let mut plugins = self.plugins.write().await;

        for (id, plugin) in plugins.iter_mut() {
            if let Err(e) = plugin.shutdown().await {
                warn!("Failed to shutdown plugin {}: {}", id, e);
            }
        }

        Ok(())
    }

    /// List all registered plugins
    pub async fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.metadata().clone()).collect()
    }
}

/// Discovered plugin information
#[derive(Debug, Clone)]
pub struct PluginDiscovery {
    /// Path to the plugin
    pub path: PathBuf,

    /// Type of plugin
    pub plugin_type: PluginType,

    /// Parsed manifest (if available)
    pub manifest: Option<PluginManifest>,
}

/// Plugin manifest structure (ltmatrix-plugin.toml)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    /// Plugin information
    pub plugin: PluginManifestInfo,
}

/// Plugin information from manifest
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManifestInfo {
    /// Plugin ID
    pub id: String,

    /// Plugin name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin description
    #[serde(default)]
    pub description: String,

    /// Plugin type
    #[serde(rename = "type")]
    pub plugin_type: PluginType,

    /// Plugin author
    #[serde(default)]
    pub author: String,

    /// Minimum ltmatrix version
    #[serde(default)]
    pub min_version: String,

    /// Entry point (for dynamic libraries)
    #[serde(default)]
    pub entry_point: Option<String>,

    /// Custom stage configuration (for pipeline stage plugins)
    #[serde(default)]
    pub stage: Option<CustomStageConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert!(!manager.plugin_paths.is_empty());
    }

    #[tokio::test]
    async fn test_register_custom_stage() {
        let manager = PluginManager::new();

        let config = CustomStageConfig::new(
            "test-stage",
            "Test Stage",
            super::super::StagePosition::After(super::super::StandardStage::Execute),
        );

        manager.register_custom_stage(config.clone()).await.unwrap();

        let stages = manager.get_custom_stages().await;
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].id, "test-stage");
    }

    #[tokio::test]
    async fn test_get_custom_stage() {
        let manager = PluginManager::new();

        let config = CustomStageConfig::new(
            "my-stage",
            "My Stage",
            super::super::StagePosition::Before(super::super::StandardStage::Test),
        );

        manager.register_custom_stage(config).await.unwrap();

        let retrieved = manager.get_custom_stage("my-stage").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "My Stage");
    }
}