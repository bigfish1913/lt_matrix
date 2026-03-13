// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! MCP (Model Context Protocol) configuration
//!
//! This module handles loading and parsing MCP server configurations
//! for use with end-to-end testing tools like Playwright.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

/// MCP configuration root structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// MCP servers configuration
    #[serde(default)]
    pub mcp: McpServers,
}

/// MCP servers configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpServers {
    /// Map of server names to their configurations
    #[serde(default)]
    pub servers: HashMap<String, McpServer>,
}

/// Individual MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// Server type (e.g., "playwright", "browser")
    #[serde(rename = "type")]
    pub server_type: String,

    /// Command to run the server
    pub command: Option<String>,

    /// Arguments to pass to the command
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables for the server
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Server working directory
    pub cwd: Option<PathBuf>,

    /// Server timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Whether the server is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_timeout() -> u64 {
    60
}

impl McpConfig {
    /// Load MCP configuration from a file
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the MCP configuration file
    ///
    /// # Returns
    ///
    /// Returns the parsed MCP configuration or an error.
    pub fn from_file<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let path = config_path.as_ref();

        debug!("Loading MCP configuration from: {}", path.display());

        // Read the configuration file
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read MCP config file: {}", path.display()))?;

        // Parse the configuration
        let config: McpConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse MCP config from: {}", path.display()))?;

        // Validate the configuration
        config.validate()?;

        debug!(
            "Successfully loaded MCP configuration with {} servers",
            config.mcp.servers.len()
        );

        Ok(config)
    }

    /// Load MCP configuration from a string
    ///
    /// # Arguments
    ///
    /// * `content` - TOML configuration content
    ///
    /// # Returns
    ///
    /// Returns the parsed MCP configuration or an error.
    pub fn from_str(content: &str) -> Result<Self> {
        let config: McpConfig = toml::from_str(content).context("Failed to parse MCP config")?;

        config.validate()?;

        Ok(config)
    }

    /// Validate the MCP configuration
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if valid, or an error describing validation issues.
    pub fn validate(&self) -> Result<()> {
        for (name, server) in &self.mcp.servers {
            // Validate server type is not empty
            if server.server_type.trim().is_empty() {
                return Err(anyhow::anyhow!("MCP server '{}' has empty type", name));
            }

            // Validate command if provided
            if let Some(ref command) = server.command {
                if command.trim().is_empty() {
                    return Err(anyhow::anyhow!("MCP server '{}' has empty command", name));
                }
            }

            // Validate timeout is reasonable
            if server.timeout == 0 {
                return Err(anyhow::anyhow!("MCP server '{}' has zero timeout", name));
            }

            // Validate working directory if provided
            if let Some(ref cwd) = server.cwd {
                if !cwd.as_path().exists() {
                    return Err(anyhow::anyhow!(
                        "MCP server '{}' working directory does not exist: {}",
                        name,
                        cwd.display()
                    ));
                }
            }

            debug!("MCP server '{}' validated successfully", name);
        }

        Ok(())
    }

    /// Get enabled servers
    ///
    /// # Returns
    ///
    /// Returns a map of enabled server names to their configurations.
    pub fn enabled_servers(&self) -> HashMap<String, McpServer> {
        self.mcp
            .servers
            .iter()
            .filter(|(_, server)| server.enabled)
            .map(|(name, server)| (name.clone(), server.clone()))
            .collect()
    }

    /// Get server by name
    ///
    /// # Arguments
    ///
    /// * `name` - Server name
    ///
    /// # Returns
    ///
    /// Returns `Some(server)` if found, `None` otherwise.
    pub fn get_server(&self, name: &str) -> Option<&McpServer> {
        self.mcp.servers.get(name)
    }

    /// Check if a server exists and is enabled
    ///
    /// # Arguments
    ///
    /// * `name` - Server name
    ///
    /// # Returns
    ///
    /// Returns `true` if the server exists and is enabled.
    pub fn is_server_enabled(&self, name: &str) -> bool {
        self.mcp
            .servers
            .get(name)
            .map(|server| server.enabled)
            .unwrap_or(false)
    }

    /// Merge with another MCP configuration
    ///
    /// Values from `other` take precedence over values from `self`.
    ///
    /// # Arguments
    ///
    /// * `other` - Configuration to merge in
    ///
    /// # Returns
    ///
    /// Returns a new merged configuration.
    pub fn merge_with(&self, other: &McpConfig) -> McpConfig {
        let mut merged = self.clone();

        // Merge servers (other takes precedence)
        for (name, server) in &other.mcp.servers {
            merged.mcp.servers.insert(name.clone(), server.clone());
        }

        merged
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        McpConfig {
            mcp: McpServers::default(),
        }
    }
}

/// MCP configuration loaded from file with path tracking
#[derive(Debug, Clone)]
pub struct LoadedMcpConfig {
    /// The parsed configuration
    pub config: McpConfig,

    /// Path to the configuration file
    pub path: PathBuf,
}

impl LoadedMcpConfig {
    /// Load MCP configuration from a file
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the MCP configuration file
    ///
    /// # Returns
    ///
    /// Returns the loaded configuration with path tracking.
    pub fn from_file<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let path = config_path.as_ref().to_path_buf();

        let config = McpConfig::from_file(&path)?;

        Ok(LoadedMcpConfig { config, path })
    }

    /// Reload the configuration from the original file
    ///
    /// # Returns
    ///
    /// Returns the reloaded configuration.
    pub fn reload(&self) -> Result<Self> {
        Self::from_file(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_minimal_mcp_config() {
        let config_content = r#"
[mcp.servers]
"#;

        let config = McpConfig::from_str(config_content).unwrap();
        assert_eq!(config.mcp.servers.len(), 0);
    }

    #[test]
    fn test_parse_playwright_server() {
        let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
command = "npx"
args = ["-y", "@playwright/mcp@latest"]
timeout = 60
"#;

        let config = McpConfig::from_str(config_content).unwrap();
        assert_eq!(config.mcp.servers.len(), 1);

        let server = config.get_server("playwright").unwrap();
        assert_eq!(server.server_type, "playwright");
        assert_eq!(server.command, Some("npx".to_string()));
        assert_eq!(server.args, vec!["-y", "@playwright/mcp@latest"]);
        assert_eq!(server.timeout, 60);
        assert!(server.enabled);
    }

    #[test]
    fn test_parse_multiple_servers() {
        let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
command = "npx"
args = ["-y", "@playwright/mcp@latest"]

[mcp.servers.browser]
type = "browser"
command = "mcp-server-browser"
timeout = 30

[mcp.servers.filesystem]
type = "filesystem"
command = "mcp-server-filesystem"
enabled = false
"#;

        let config = McpConfig::from_str(config_content).unwrap();
        assert_eq!(config.mcp.servers.len(), 3);

        assert!(config.is_server_enabled("playwright"));
        assert!(config.is_server_enabled("browser"));
        assert!(!config.is_server_enabled("filesystem"));
    }

    #[test]
    fn test_enabled_servers_filter() {
        let config_content = r#"
[mcp.servers.server1]
type = "type1"
enabled = true

[mcp.servers.server2]
type = "type2"
enabled = false

[mcp.servers.server3]
type = "type3"
"#;

        let config = McpConfig::from_str(config_content).unwrap();
        let enabled = config.enabled_servers();

        assert_eq!(enabled.len(), 2);
        assert!(enabled.contains_key("server1"));
        assert!(enabled.contains_key("server3"));
        assert!(!enabled.contains_key("server2"));
    }

    #[test]
    fn test_validate_empty_type() {
        let config_content = r#"
[mcp.servers.bad_server]
type = ""
"#;

        let result = McpConfig::from_str(config_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty type"));
    }

    #[test]
    fn test_validate_zero_timeout() {
        let config_content = r#"
[mcp.servers.bad_server]
type = "test"
timeout = 0
"#;

        let result = McpConfig::from_str(config_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("zero timeout"));
    }

    #[test]
    fn test_validate_empty_command() {
        let config_content = r#"
[mcp.servers.bad_server]
type = "test"
command = ""
"#;

        let result = McpConfig::from_str(config_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty command"));
    }

    #[test]
    fn test_mcp_config_merge() {
        let config1_content = r#"
[mcp.servers.server1]
type = "type1"
command = "cmd1"

[mcp.servers.server2]
type = "type2"
"#;

        let config2_content = r#"
[mcp.servers.server2]
type = "type2-updated"
command = "cmd2"

[mcp.servers.server3]
type = "type3"
"#;

        let config1 = McpConfig::from_str(config1_content).unwrap();
        let config2 = McpConfig::from_str(config2_content).unwrap();

        let merged = config1.merge_with(&config2);

        assert_eq!(merged.mcp.servers.len(), 3);
        assert_eq!(
            merged.get_server("server1").unwrap().command,
            Some("cmd1".to_string())
        );
        assert_eq!(
            merged.get_server("server2").unwrap().command,
            Some("cmd2".to_string())
        );
        assert!(merged.get_server("server3").is_some());
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("mcp-config.toml");

        let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
command = "npx"
"#;

        fs::write(&config_path, config_content).unwrap();

        let loaded = LoadedMcpConfig::from_file(&config_path).unwrap();
        assert_eq!(loaded.config.mcp.servers.len(), 1);
        assert_eq!(loaded.path, config_path);
    }

    #[test]
    fn test_reload_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("mcp-config.toml");

        let initial_content = r#"
[mcp.servers.server1]
type = "type1"
"#;

        fs::write(&config_path, initial_content).unwrap();

        let loaded = LoadedMcpConfig::from_file(&config_path).unwrap();
        assert_eq!(loaded.config.mcp.servers.len(), 1);

        // Update the file
        let updated_content = r#"
[mcp.servers.server1]
type = "type1"

[mcp.servers.server2]
type = "type2"
"#;

        fs::write(&config_path, updated_content).unwrap();

        let reloaded = loaded.reload().unwrap();
        assert_eq!(reloaded.config.mcp.servers.len(), 2);
    }

    #[test]
    fn test_parse_server_with_environment() {
        let config_content = r#"
[mcp.servers.server1]
type = "test"
command = "cmd"

[mcp.servers.server1.env]
API_KEY = "test-key"
DEBUG = "true"
"#;

        let config = McpConfig::from_str(config_content).unwrap();
        let server = config.get_server("server1").unwrap();

        assert_eq!(server.env.len(), 2);
        assert_eq!(server.env.get("API_KEY"), Some(&"test-key".to_string()));
        assert_eq!(server.env.get("DEBUG"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_server_with_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let cwd = temp_dir.path();

        // Convert path to string with forward slashes for TOML compatibility
        let cwd_str = cwd.to_string_lossy().replace('\\', "/");

        let config_content = format!(
            r#"
[mcp.servers.server1]
type = "test"
command = "cmd"
cwd = "{}"
"#,
            cwd_str
        );

        let config = McpConfig::from_str(&config_content).unwrap();
        let server = config.get_server("server1").unwrap();

        assert_eq!(server.cwd, Some(cwd.to_path_buf()));
    }

    #[test]
    fn test_parse_server_with_custom_args() {
        let config_content = r#"
[mcp.servers.server1]
type = "test"
command = "my-command"
args = ["--port", "8080", "--verbose", "--host", "localhost"]
"#;

        let config = McpConfig::from_str(config_content).unwrap();
        let server = config.get_server("server1").unwrap();

        assert_eq!(server.args.len(), 5);
        assert_eq!(server.args[0], "--port");
        assert_eq!(server.args[1], "8080");
    }
}
