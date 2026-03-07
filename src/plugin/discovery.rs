//! Plugin discovery utilities
//!
//! This module provides utilities for discovering plugins in the filesystem.

use anyhow::Result;
use std::path::{Path, PathBuf};

use super::manager::PluginDiscovery;
use super::traits::PluginType;

/// Plugin discovery service
pub struct PluginDiscoveryService {
    /// Paths to search for plugins
    search_paths: Vec<PathBuf>,
}

impl PluginDiscoveryService {
    /// Create a new discovery service
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
        }
    }

    /// Add a search path
    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Discover all plugins
    pub async fn discover(&self) -> Result<Vec<PluginDiscovery>> {
        let mut discoveries = Vec::new();

        for path in &self.search_paths {
            if path.exists() {
                let found = self.discover_in_directory(path).await?;
                discoveries.extend(found);
            }
        }

        Ok(discoveries)
    }

    /// Discover plugins in a directory
    async fn discover_in_directory(&self, dir: &Path) -> Result<Vec<PluginDiscovery>> {
        let mut discoveries = Vec::new();

        if !dir.is_dir() {
            return Ok(discoveries);
        }

        let entries = std::fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Check for plugin manifest
            let manifest_path = path.join("ltmatrix-plugin.toml");
            if manifest_path.exists() {
                // Parse manifest and create discovery
                if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = toml::from_str::<super::manager::PluginManifest>(&content) {
                        discoveries.push(PluginDiscovery {
                            path,
                            plugin_type: manifest.plugin.plugin_type,
                            manifest: Some(manifest),
                        });
                    }
                }
            }
        }

        Ok(discoveries)
    }
}

impl Default for PluginDiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_service_creation() {
        let service = PluginDiscoveryService::new();
        assert!(service.search_paths.is_empty());
    }
}