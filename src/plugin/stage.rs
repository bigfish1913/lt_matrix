// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Pipeline stage plugin trait
//!
//! This module defines the trait for custom pipeline stages.

use anyhow::Result;
use async_trait::async_trait;
use indicatif::ProgressBar;

use super::{CustomStageConfig, Plugin, StageResult};
use crate::models::Task;

/// Trait for custom pipeline stages
#[async_trait]
pub trait PipelineStagePlugin: Plugin {
    /// Get the stage configuration
    fn config(&self) -> &CustomStageConfig;

    /// Get mutable stage configuration
    fn config_mut(&mut self) -> &mut CustomStageConfig;

    /// Execute the stage
    ///
    /// # Arguments
    ///
    /// * `tasks` - Tasks to process
    /// * `config` - Stage-specific configuration
    /// * `progress` - Optional progress bar for UI updates
    ///
    /// # Returns
    ///
    /// A StageResult containing the processed tasks and execution status.
    async fn execute(
        &self,
        tasks: Vec<Task>,
        config: &std::collections::HashMap<String, serde_json::Value>,
        progress: Option<&ProgressBar>,
    ) -> Result<StageResult>;

    /// Validate stage configuration
    fn validate_config(&self, config: &std::collections::HashMap<String, serde_json::Value>) -> Result<()> {
        // Default implementation does no validation
        let _ = config;
        Ok(())
    }

    /// Check prerequisites for stage execution
    async fn check_prerequisites(&self) -> Result<bool> {
        // Default implementation always passes
        Ok(true)
    }

    /// Get the stage timeout in seconds
    fn timeout(&self) -> u64 {
        self.config().timeout_seconds
    }

    /// Get whether the stage should skip on failure
    fn skip_on_failure(&self) -> bool {
        self.config().skip_on_failure
    }
}

/// Built-in example stage that logs task information
pub struct LoggingStage {
    metadata: super::PluginMetadata,
    config: CustomStageConfig,
}

impl LoggingStage {
    /// Create a new logging stage
    pub fn new() -> Self {
        let metadata = super::PluginMetadata::new(
            "builtin-logging",
            "Logging Stage",
            "1.0.0",
            super::PluginType::PipelineStage,
        )
        .with_description("Logs task information for debugging");

        let config = CustomStageConfig::new(
            "log-tasks",
            "Log Tasks",
            super::StagePosition::After(super::StandardStage::Execute),
        )
        .with_description("Log task details after execution");

        Self { metadata, config }
    }
}

impl Default for LoggingStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LoggingStage {
    fn metadata(&self) -> &super::PluginMetadata {
        &self.metadata
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }
}

#[async_trait]
impl PipelineStagePlugin for LoggingStage {
    fn config(&self) -> &CustomStageConfig {
        &self.config
    }

    fn config_mut(&mut self) -> &mut CustomStageConfig {
        &mut self.config
    }

    async fn execute(
        &self,
        tasks: Vec<Task>,
        config: &std::collections::HashMap<String, serde_json::Value>,
        progress: Option<&ProgressBar>,
    ) -> Result<StageResult> {
        let log_level = config
            .get("log_level")
            .and_then(|v| v.as_str())
            .unwrap_or("info");

        if let Some(pb) = progress {
            pb.set_message(format!("Logging {} tasks...", tasks.len()));
        }

        for task in &tasks {
            match log_level {
                "debug" => tracing::debug!(
                    "Task {}: {} - {:?}",
                    task.id,
                    task.title,
                    task.status
                ),
                "warn" => tracing::warn!(
                    "Task {}: {} - {:?}",
                    task.id,
                    task.title,
                    task.status
                ),
                _ => tracing::info!(
                    "Task {}: {} - {:?}",
                    task.id,
                    task.title,
                    task.status
                ),
            }
        }

        if let Some(pb) = progress {
            pb.set_message(format!("Logged {} tasks", tasks.len()));
        }

        Ok(StageResult::success(tasks))
    }
}

/// Built-in example stage that adds a delay (for testing)
pub struct DelayStage {
    metadata: super::PluginMetadata,
    config: CustomStageConfig,
}

impl DelayStage {
    /// Create a new delay stage
    pub fn new() -> Self {
        let metadata = super::PluginMetadata::new(
            "builtin-delay",
            "Delay Stage",
            "1.0.0",
            super::PluginType::PipelineStage,
        )
        .with_description("Adds a configurable delay between stages");

        let config = CustomStageConfig::new(
            "delay",
            "Delay",
            super::StagePosition::After(super::StandardStage::Generate),
        )
        .with_description("Pause pipeline execution")
        .with_config("seconds".to_string(), serde_json::json!(1));

        Self { metadata, config }
    }
}

impl Default for DelayStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for DelayStage {
    fn metadata(&self) -> &super::PluginMetadata {
        &self.metadata
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }
}

#[async_trait]
impl PipelineStagePlugin for DelayStage {
    fn config(&self) -> &CustomStageConfig {
        &self.config
    }

    fn config_mut(&mut self) -> &mut CustomStageConfig {
        &mut self.config
    }

    async fn execute(
        &self,
        tasks: Vec<Task>,
        config: &std::collections::HashMap<String, serde_json::Value>,
        progress: Option<&ProgressBar>,
    ) -> Result<StageResult> {
        let seconds = config
            .get("seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        if let Some(pb) = progress {
            pb.set_message(format!("Waiting {} seconds...", seconds));
        }

        tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;

        if let Some(pb) = progress {
            pb.set_message("Delay completed");
        }

        Ok(StageResult::success(tasks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_stage_creation() {
        let stage = LoggingStage::new();
        assert_eq!(stage.metadata().id, "builtin-logging");
        assert_eq!(stage.config().id, "log-tasks");
    }

    #[test]
    fn test_delay_stage_creation() {
        let stage = DelayStage::new();
        assert_eq!(stage.metadata().id, "builtin-delay");
        assert_eq!(stage.config().id, "delay");
    }

    #[tokio::test]
    async fn test_logging_stage_execute() {
        let stage = LoggingStage::new();
        let tasks = vec![
            Task::new("task-1", "Test 1", "Description 1"),
            Task::new("task-2", "Test 2", "Description 2"),
        ];

        let config = std::collections::HashMap::new();
        let result = stage.execute(tasks.clone(), &config, None).await.unwrap();

        assert!(result.success);
        assert_eq!(result.tasks.len(), 2);
    }
}