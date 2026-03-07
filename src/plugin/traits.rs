//! Plugin traits and types
//!
//! This module defines the core traits and types for the plugin system.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::Task;

/// Plugin type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    /// Agent backend plugin
    AgentBackend,
    /// Pipeline stage plugin
    PipelineStage,
    /// Output formatter plugin
    Formatter,
    /// Validation rule plugin
    Validator,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin description
    pub description: String,

    /// Plugin author
    pub author: Option<String>,

    /// Plugin type
    pub plugin_type: PluginType,

    /// Minimum ltmatrix version required
    pub min_version: Option<String>,

    /// Plugin homepage URL
    pub homepage: Option<String>,

    /// Plugin repository URL
    pub repository: Option<String>,

    /// Plugin license
    pub license: Option<String>,

    /// Plugin tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl PluginMetadata {
    /// Create new plugin metadata
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        plugin_type: PluginType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: String::new(),
            author: None,
            plugin_type,
            min_version: None,
            homepage: None,
            repository: None,
            license: None,
            tags: Vec::new(),
        }
    }

    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }
}

/// Core plugin trait
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    /// Check if the plugin is enabled
    fn is_enabled(&self) -> bool {
        true
    }

    /// Enable or disable the plugin
    fn set_enabled(&mut self, enabled: bool);
}

/// Position where a custom stage should be inserted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagePosition {
    /// Insert before a standard stage
    Before(StandardStage),
    /// Insert after a standard stage
    After(StandardStage),
    /// Insert at the beginning of the pipeline
    First,
    /// Insert at the end of the pipeline
    Last,
    /// Replace a standard stage
    Replace(StandardStage),
}

/// Standard pipeline stages for positioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StandardStage {
    Generate,
    Assess,
    Execute,
    Test,
    Review,
    Verify,
    Commit,
    Memory,
}

impl StandardStage {
    /// Convert to PipelineStage
    pub fn to_pipeline_stage(&self) -> crate::models::PipelineStage {
        match self {
            StandardStage::Generate => crate::models::PipelineStage::Generate,
            StandardStage::Assess => crate::models::PipelineStage::Assess,
            StandardStage::Execute => crate::models::PipelineStage::Execute,
            StandardStage::Test => crate::models::PipelineStage::Test,
            StandardStage::Review => crate::models::PipelineStage::Review,
            StandardStage::Verify => crate::models::PipelineStage::Verify,
            StandardStage::Commit => crate::models::PipelineStage::Commit,
            StandardStage::Memory => crate::models::PipelineStage::Memory,
        }
    }
}

impl std::fmt::Display for StandardStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StandardStage::Generate => write!(f, "generate"),
            StandardStage::Assess => write!(f, "assess"),
            StandardStage::Execute => write!(f, "execute"),
            StandardStage::Test => write!(f, "test"),
            StandardStage::Review => write!(f, "review"),
            StandardStage::Verify => write!(f, "verify"),
            StandardStage::Commit => write!(f, "commit"),
            StandardStage::Memory => write!(f, "memory"),
        }
    }
}

impl std::str::FromStr for StandardStage {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "generate" => Ok(StandardStage::Generate),
            "assess" => Ok(StandardStage::Assess),
            "execute" => Ok(StandardStage::Execute),
            "test" => Ok(StandardStage::Test),
            "review" => Ok(StandardStage::Review),
            "verify" => Ok(StandardStage::Verify),
            "commit" => Ok(StandardStage::Commit),
            "memory" => Ok(StandardStage::Memory),
            _ => anyhow::bail!("Unknown standard stage: {}", s),
        }
    }
}

/// Result of a custom stage execution
#[derive(Debug, Clone)]
pub struct StageResult {
    /// Tasks after stage processing
    pub tasks: Vec<Task>,

    /// Whether the stage succeeded
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,

    /// Stage-specific metrics
    pub metrics: HashMap<String, serde_json::Value>,
}

impl StageResult {
    /// Create a successful result
    pub fn success(tasks: Vec<Task>) -> Self {
        Self {
            tasks,
            success: true,
            error: None,
            metrics: HashMap::new(),
        }
    }

    /// Create a failed result
    pub fn failure(tasks: Vec<Task>, error: impl Into<String>) -> Self {
        Self {
            tasks,
            success: false,
            error: Some(error.into()),
            metrics: HashMap::new(),
        }
    }

    /// Add a metric
    pub fn with_metric(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metrics.insert(key.into(), value);
        self
    }
}

/// Configuration for a custom stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomStageConfig {
    /// Stage identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Stage description
    #[serde(default)]
    pub description: String,

    /// Position in the pipeline
    pub position: StagePosition,

    /// Whether this stage is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Whether to skip this stage on failure
    #[serde(default)]
    pub skip_on_failure: bool,

    /// Maximum timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Stage-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,

    /// Modes this stage should run in (empty = all modes)
    #[serde(default)]
    pub modes: Vec<String>,

    /// Plugin that provides this stage (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
}

fn default_enabled() -> bool {
    true
}

fn default_timeout() -> u64 {
    3600
}

impl CustomStageConfig {
    /// Create a new custom stage configuration
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        position: StagePosition,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            position,
            enabled: true,
            skip_on_failure: false,
            timeout_seconds: 3600,
            config: HashMap::new(),
            modes: Vec::new(),
            plugin: None,
        }
    }

    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set enabled status
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a configuration value
    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }

    /// Set modes
    pub fn with_modes(mut self, modes: Vec<String>) -> Self {
        self.modes = modes;
        self
    }

    /// Check if this stage should run for the given mode
    pub fn should_run_for_mode(&self, mode: &crate::models::ExecutionMode) -> bool {
        if self.modes.is_empty() {
            return true;
        }
        let mode_str = mode.to_string();
        self.modes.contains(&mode_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata_creation() {
        let meta = PluginMetadata::new("test-plugin", "Test Plugin", "1.0.0", PluginType::PipelineStage)
            .with_description("A test plugin")
            .with_author("Test Author");

        assert_eq!(meta.id, "test-plugin");
        assert_eq!(meta.name, "Test Plugin");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.plugin_type, PluginType::PipelineStage);
    }

    #[test]
    fn test_standard_stage_parsing() {
        assert_eq!(
            StandardStage::from_str("generate").unwrap(),
            StandardStage::Generate
        );
        assert_eq!(
            StandardStage::from_str("EXECUTE").unwrap(),
            StandardStage::Execute
        );
        assert!(StandardStage::from_str("unknown").is_err());
    }

    #[test]
    fn test_custom_stage_config() {
        let config = CustomStageConfig::new(
            "my-stage",
            "My Custom Stage",
            StagePosition::After(StandardStage::Test),
        )
        .with_description("A custom stage")
        .with_config("key".to_string(), serde_json::json!("value"));

        assert_eq!(config.id, "my-stage");
        assert!(config.enabled);
        assert!(config.should_run_for_mode(&crate::models::ExecutionMode::Standard));
    }

    #[test]
    fn test_stage_result() {
        let result = StageResult::success(vec![])
            .with_metric("count".to_string(), serde_json::json!(42));

        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.metrics.get("count").unwrap(), &serde_json::json!(42));
    }
}