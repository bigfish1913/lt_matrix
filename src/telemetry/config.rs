//! Telemetry configuration
//!
//! This module handles configuration for telemetry collection and transmission.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use dirs::home_dir;
use tokio::fs;
use tracing::{debug, info};

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled (opt-in only)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Analytics endpoint URL
    #[serde(default = "default_endpoint")]
    pub endpoint: String,

    /// Number of events to batch before sending
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Maximum number of events to buffer in memory
    #[serde(default = "default_max_buffer_size")]
    pub max_buffer_size: usize,

    /// HTTP timeout in seconds for sending telemetry
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,

    /// Number of retry attempts for failed sends
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        TelemetryConfig {
            enabled: false,
            endpoint: default_endpoint(),
            batch_size: default_batch_size(),
            max_buffer_size: default_max_buffer_size(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
        }
    }
}

fn default_enabled() -> bool {
    false
}

fn default_endpoint() -> String {
    "https://telemetry.ltmatrix.dev/events".to_string()
}

fn default_batch_size() -> usize {
    10
}

fn default_max_buffer_size() -> usize {
    100
}

fn default_timeout_secs() -> u64 {
    5
}

fn default_max_retries() -> usize {
    3
}

impl TelemetryConfig {
    /// Create a new telemetry config with custom settings
    pub fn builder() -> TelemetryConfigBuilder {
        TelemetryConfigBuilder::new()
    }

    /// Create a config with telemetry enabled
    pub fn enabled() -> Self {
        TelemetryConfig {
            enabled: true,
            ..Default::default()
        }
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Builder for creating custom telemetry configurations
pub struct TelemetryConfigBuilder {
    config: TelemetryConfig,
}

impl TelemetryConfigBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        TelemetryConfigBuilder {
            config: TelemetryConfig::default(),
        }
    }

    /// Enable telemetry
    pub fn enabled(mut self) -> Self {
        self.config.enabled = true;
        self
    }

    /// Disable telemetry
    pub fn disabled(mut self) -> Self {
        self.config.enabled = false;
        self
    }

    /// Set the analytics endpoint
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.config.endpoint = endpoint.into();
        self
    }

    /// Set the batch size
    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }

    /// Set the maximum buffer size
    pub fn max_buffer_size(mut self, size: usize) -> Self {
        self.config.max_buffer_size = size;
        self
    }

    /// Set the HTTP timeout
    pub fn timeout_secs(mut self, timeout: u64) -> Self {
        self.config.timeout_secs = timeout;
        self
    }

    /// Set the maximum number of retries
    pub fn max_retries(mut self, retries: usize) -> Self {
        self.config.max_retries = retries;
        self
    }

    /// Build the configuration
    pub fn build(self) -> TelemetryConfig {
        self.config
    }
}

impl Default for TelemetryConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Load or create a session ID for telemetry
///
/// The session ID is an anonymous UUID that persists across runs
/// to track usage patterns without revealing user identity.
pub async fn load_or_create_session_id() -> Result<uuid::Uuid> {
    let session_file = get_session_id_file_path()?;

    // Try to load existing session ID
    if session_file.exists() {
        match fs::read_to_string(&session_file).await {
            Ok(content) => {
                match uuid::Uuid::from_str(&content.trim()) {
                    Ok(uuid) => {
                        debug!("Loaded existing telemetry session ID");
                        return Ok(uuid);
                    }
                    Err(e) => {
                        debug!("Invalid session ID in file, will create new one: {}", e);
                    }
                }
            }
            Err(e) => {
                debug!("Failed to read session ID file: {}", e);
            }
        }
    }

    // Create new session ID
    let new_id = uuid::Uuid::new_v4();

    // Ensure directory exists
    if let Some(parent) = session_file.parent() {
        fs::create_dir_all(parent).await
            .context("Failed to create telemetry directory")?;
    }

    // Write new session ID
    fs::write(&session_file, new_id.to_string())
        .await
        .context("Failed to write session ID file")?;

    info!("Created new anonymous telemetry session ID");
    Ok(new_id)
}

/// Get the path to the session ID file
fn get_session_id_file_path() -> Result<PathBuf> {
    let home = home_dir()
        .context("Could not determine home directory")?;

    Ok(home.join(".ltmatrix").join("telemetry_session_id"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TelemetryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.endpoint, "https://telemetry.ltmatrix.dev/events");
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.max_buffer_size, 100);
        assert_eq!(config.timeout_secs, 5);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_enabled_config() {
        let config = TelemetryConfig::enabled();
        assert!(config.enabled);
    }

    #[test]
    fn test_builder_default() {
        let config = TelemetryConfig::builder().build();
        assert!(!config.enabled);
    }

    #[test]
    fn test_builder_enabled() {
        let config = TelemetryConfig::builder()
            .enabled()
            .build();
        assert!(config.enabled);
    }

    #[test]
    fn test_builder_custom() {
        let config = TelemetryConfig::builder()
            .enabled()
            .endpoint("https://custom.endpoint.com/events")
            .batch_size(20)
            .timeout_secs(10)
            .build();

        assert!(config.enabled);
        assert_eq!(config.endpoint, "https://custom.endpoint.com/events");
        assert_eq!(config.batch_size, 20);
        assert_eq!(config.timeout_secs, 10);
    }

    #[test]
    fn test_config_serialization() {
        let config = TelemetryConfig {
            enabled: true,
            endpoint: "https://example.com".to_string(),
            batch_size: 15,
            ..Default::default()
        };

        let toml = toml::to_string(&config);
        assert!(toml.is_ok());

        let parsed: Result<TelemetryConfig, _> = toml::from_str(&toml.unwrap());
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_session_id_file_path() {
        let path = get_session_id_file_path();
        assert!(path.is_ok());

        let path = path.unwrap();
        assert!(path.ends_with(".ltmatrix/telemetry_session_id"));
    }
}
