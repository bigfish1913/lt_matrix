// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Execution mode configuration
//!
//! Defines configuration for different execution modes (fast, standard, expert)

use serde::{Deserialize, Serialize};

/// Execution mode type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Fast,
    Standard,
    Expert,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        ExecutionMode::Standard
    }
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionMode::Fast => write!(f, "fast"),
            ExecutionMode::Standard => write!(f, "standard"),
            ExecutionMode::Expert => write!(f, "expert"),
        }
    }
}

/// Configuration for execution modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    /// Model to use for this mode
    #[serde(default)]
    pub model: String,

    /// Fast model for simple tasks (standard mode only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_fast: Option<String>,

    /// Smart model for complex tasks (standard mode only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_smart: Option<String>,

    /// Maximum task depth for splitting
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,

    /// Maximum retries for failed operations
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Run tests during execution
    #[serde(default = "default_run_tests")]
    pub run_tests: bool,

    /// Verify task completion
    #[serde(default = "default_verify")]
    pub verify: bool,

    /// Planning phase timeout in seconds
    #[serde(default = "default_timeout_plan")]
    pub timeout_plan: u64,

    /// Execution phase timeout in seconds
    #[serde(default = "default_timeout_exec")]
    pub timeout_exec: u64,
}

impl Default for ModeConfig {
    fn default() -> Self {
        ModeConfig {
            model: String::new(),
            model_fast: None,
            model_smart: None,
            max_depth: default_max_depth(),
            max_retries: default_max_retries(),
            run_tests: default_run_tests(),
            verify: default_verify(),
            timeout_plan: default_timeout_plan(),
            timeout_exec: default_timeout_exec(),
        }
    }
}

impl ModeConfig {
    /// Create a new mode configuration with the specified model
    pub fn new(model: impl Into<String>) -> Self {
        ModeConfig {
            model: model.into(),
            ..Default::default()
        }
    }

    /// Set the fast model (for standard mode)
    pub fn with_fast_model(mut self, model: impl Into<String>) -> Self {
        self.model_fast = Some(model.into());
        self
    }

    /// Set the smart model (for standard mode)
    pub fn with_smart_model(mut self, model: impl Into<String>) -> Self {
        self.model_smart = Some(model.into());
        self
    }

    /// Set max task depth
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set max retries
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Enable or disable tests
    pub fn with_tests(mut self, run_tests: bool) -> Self {
        self.run_tests = run_tests;
        self
    }

    /// Enable or disable verification
    pub fn with_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }

    /// Set planning timeout
    pub fn with_timeout_plan(mut self, timeout: u64) -> Self {
        self.timeout_plan = timeout;
        self
    }

    /// Set execution timeout
    pub fn with_timeout_exec(mut self, timeout: u64) -> Self {
        self.timeout_exec = timeout;
        self
    }
}

/// Fast mode configuration
impl ModeConfig {
    /// Create fast mode configuration
    ///
    /// Fast mode: Quickest completion using Haiku/Sonnet, skips tests
    pub fn fast_mode() -> Self {
        ModeConfig {
            model: "claude-haiku-4-5".to_string(),
            max_depth: 2,
            max_retries: 1,
            run_tests: false,
            verify: true,
            timeout_plan: 60,
            timeout_exec: 1800,
            model_fast: None,
            model_smart: None,
        }
    }

    /// Create optimized fast mode
    pub fn fast_optimized() -> Self {
        ModeConfig {
            model: "claude-haiku-4-5".to_string(),
            max_depth: 1,
            max_retries: 1,
            run_tests: false,
            verify: false, // Skip verification for maximum speed
            timeout_plan: 30,
            timeout_exec: 600,
            model_fast: None,
            model_smart: None,
        }
    }
}

/// Standard mode configuration
impl ModeConfig {
    /// Create standard mode configuration
    ///
    /// Standard mode: Full 6-stage pipeline with proper testing and verification
    pub fn standard_mode() -> Self {
        ModeConfig {
            model: "claude-sonnet-4-6".to_string(),
            model_fast: Some("claude-sonnet-4-6".to_string()),
            model_smart: Some("claude-opus-4-6".to_string()),
            max_depth: 3,
            max_retries: 3,
            run_tests: true,
            verify: true,
            timeout_plan: 120,
            timeout_exec: 3600,
        }
    }

    /// Create balanced standard mode
    pub fn standard_balanced() -> Self {
        ModeConfig {
            model: "claude-sonnet-4-6".to_string(),
            model_fast: Some("claude-sonnet-4-6".to_string()),
            model_smart: Some("claude-sonnet-4-6".to_string()), // Use Sonnet for all
            max_depth: 2,
            max_retries: 2,
            run_tests: true,
            verify: true,
            timeout_plan: 90,
            timeout_exec: 1800,
        }
    }
}

/// Expert mode configuration
impl ModeConfig {
    /// Create expert mode configuration
    ///
    /// Expert mode: Highest quality with Opus, full testing, code review
    pub fn expert_mode() -> Self {
        ModeConfig {
            model: "claude-opus-4-6".to_string(),
            max_depth: 4,
            max_retries: 5,
            run_tests: true,
            verify: true,
            timeout_plan: 300,
            timeout_exec: 7200, // 2 hours for complex tasks
            model_fast: None,
            model_smart: None,
        }
    }

    /// Create thorough expert mode with code review
    pub fn expert_with_review() -> Self {
        ModeConfig {
            model: "claude-opus-4-6".to_string(),
            max_depth: 5,
            max_retries: 5,
            run_tests: true,
            verify: true,
            timeout_plan: 600,
            timeout_exec: 10800, // 3 hours for very complex tasks
            model_fast: None,
            model_smart: None,
        }
    }
}

// Helper functions for defaults
fn default_max_depth() -> u32 {
    3
}

fn default_max_retries() -> u32 {
    3
}

fn default_run_tests() -> bool {
    true
}

fn default_verify() -> bool {
    true
}

fn default_timeout_plan() -> u64 {
    120
}

fn default_timeout_exec() -> u64 {
    3600
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_default() {
        assert_eq!(ExecutionMode::default(), ExecutionMode::Standard);
    }

    #[test]
    fn test_execution_mode_display() {
        assert_eq!(ExecutionMode::Fast.to_string(), "fast");
        assert_eq!(ExecutionMode::Standard.to_string(), "standard");
        assert_eq!(ExecutionMode::Expert.to_string(), "expert");
    }

    #[test]
    fn test_mode_config_default() {
        let config = ModeConfig::default();
        assert!(config.model.is_empty());
        assert!(config.model_fast.is_none());
        assert!(config.model_smart.is_none());
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.max_retries, 3);
        assert!(config.run_tests);
        assert!(config.verify);
        assert_eq!(config.timeout_plan, 120);
        assert_eq!(config.timeout_exec, 3600);
    }

    #[test]
    fn test_mode_config_builder() {
        let config = ModeConfig::new("claude-sonnet-4-6")
            .with_fast_model("claude-haiku-4-5")
            .with_smart_model("claude-opus-4-6")
            .with_max_depth(5)
            .with_max_retries(5)
            .with_tests(false)
            .with_verify(true)
            .with_timeout_plan(300)
            .with_timeout_exec(7200);

        assert_eq!(config.model, "claude-sonnet-4-6");
        assert_eq!(config.model_fast, Some("claude-haiku-4-5".to_string()));
        assert_eq!(config.model_smart, Some("claude-opus-4-6".to_string()));
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.max_retries, 5);
        assert!(!config.run_tests);
        assert!(config.verify);
        assert_eq!(config.timeout_plan, 300);
        assert_eq!(config.timeout_exec, 7200);
    }

    #[test]
    fn test_fast_mode() {
        let config = ModeConfig::fast_mode();
        assert_eq!(config.model, "claude-haiku-4-5");
        assert_eq!(config.max_depth, 2);
        assert_eq!(config.max_retries, 1);
        assert!(!config.run_tests);
        assert!(config.verify);
        assert_eq!(config.timeout_plan, 60);
        assert_eq!(config.timeout_exec, 1800);
    }

    #[test]
    fn test_standard_mode() {
        let config = ModeConfig::standard_mode();
        assert_eq!(config.model, "claude-sonnet-4-6");
        assert_eq!(config.model_fast, Some("claude-sonnet-4-6".to_string()));
        assert_eq!(config.model_smart, Some("claude-opus-4-6".to_string()));
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.max_retries, 3);
        assert!(config.run_tests);
        assert!(config.verify);
        assert_eq!(config.timeout_plan, 120);
        assert_eq!(config.timeout_exec, 3600);
    }

    #[test]
    fn test_expert_mode() {
        let config = ModeConfig::expert_mode();
        assert_eq!(config.model, "claude-opus-4-6");
        assert_eq!(config.max_depth, 4);
        assert_eq!(config.max_retries, 5);
        assert!(config.run_tests);
        assert!(config.verify);
        assert_eq!(config.timeout_plan, 300);
        assert_eq!(config.timeout_exec, 7200);
    }

    #[test]
    fn test_mode_config_serialization() {
        let config = ModeConfig::standard_mode();
        let json = serde_json::to_string(&config).unwrap();

        let deserialized: ModeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, config.model);
        assert_eq!(deserialized.max_depth, config.max_depth);
        assert_eq!(deserialized.run_tests, config.run_tests);
    }

    #[test]
    fn test_fast_mode_serialization() {
        let config = ModeConfig::fast_mode();
        let json = serde_json::to_string(&config).unwrap();

        let deserialized: ModeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, "claude-haiku-4-5");
        assert!(!deserialized.run_tests);
    }
}
