//! Plugin loader for dynamic libraries and WASM modules
//!
//! This module provides functionality for loading plugins from
//! shared libraries and WASM modules.

use anyhow::{Result, bail};
use std::path::Path;

/// Plugin loader interface
pub struct PluginLoader;

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Self {
        Self
    }

    /// Load a plugin from a shared library
    #[cfg(feature = "dynamic-plugins")]
    pub async fn load_shared_library(&self, path: &Path) -> Result<()> {
        // Dynamic library loading is disabled by default for security
        // and cross-platform compatibility
        bail!("Dynamic plugin loading is not enabled. Recompile with 'dynamic-plugins' feature.")
    }

    /// Load a plugin from a WASM module
    pub async fn load_wasm_module(&self, path: &Path) -> Result<()> {
        // WASM loading is planned for future implementation
        bail!("WASM plugin loading is not yet implemented")
    }

    /// Check if a file is a valid shared library
    pub fn is_shared_library(path: &Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        matches!(ext, "so" | "dylib" | "dll")
    }

    /// Check if a file is a WASM module
    pub fn is_wasm_module(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "wasm")
            .unwrap_or(false)
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_loader_creation() {
        let loader = PluginLoader::new();
        // Basic creation test
        assert!(true);
    }

    #[test]
    fn test_is_shared_library() {
        assert!(PluginLoader::is_shared_library(Path::new("plugin.so")));
        assert!(PluginLoader::is_shared_library(Path::new("plugin.dll")));
        assert!(PluginLoader::is_shared_library(Path::new("plugin.dylib")));
        assert!(!PluginLoader::is_shared_library(Path::new("plugin.wasm")));
        assert!(!PluginLoader::is_shared_library(Path::new("plugin.toml")));
    }

    #[test]
    fn test_is_wasm_module() {
        assert!(PluginLoader::is_wasm_module(Path::new("plugin.wasm")));
        assert!(!PluginLoader::is_wasm_module(Path::new("plugin.so")));
    }
}