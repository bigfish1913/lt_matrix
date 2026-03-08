// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Feature flag system for gradual rollout and experimental features
//!
//! This module provides a comprehensive feature flag system that allows:
//! - Enabling/disabling experimental features via configuration
//! - Gradual rollout based on percentage or user criteria
//! - A/B testing support for different implementations
//! - Feature flags for agent backends, pipeline stages, and schedulers
//!
//! # Configuration
//!
//! Feature flags are configured in the TOML configuration file:
//!
//! ```toml
//! [agent_backend]
//! enable_claude_opus_backend = true
//! enable_opencode_backend = false
//!
//! [pipeline]
//! enable_parallel_execution = true
//! enable_smart_cache = true
//! enable_incremental_builds = false
//! enable_distributed_tasks = false
//!
//! [scheduler]
//! enable_priority_scheduler = false
//! enable_adaptive_scheduler = false
//!
//! [rollout.enable_parallel_execution]
//! percentage = 50
//! users = ["beta_user1", "beta_user2"]
//!
//! [rollout.enable_smart_cache]
//! percentage = 100
//! ```
//!
//! # Usage
//!
//! ```
//! use ltmatrix::feature::{FeatureFlag, FeatureFlags, FeatureConfig};
//!
//! // Create feature flags from configuration
//! let config = FeatureConfig::default();
//! let flags = FeatureFlags::new(config);
//!
//! // Check if a feature is enabled
//! if flags.is_enabled(FeatureFlag::EnableParallelExecution) {
//!     // Use parallel execution
//! }
//!
//! // Check gradual rollout
//! # use std::collections::HashMap;
//! # use ltmatrix::feature::RolloutConfig;
//! # let mut config = FeatureConfig::default();
//! # config.pipeline.enable_smart_cache = true;
//! # let mut rollout = HashMap::new();
//! # rollout.insert("enable_smart_cache".to_string(), RolloutConfig::new(100));
//! # config.rollout = rollout;
//! # let flags = FeatureFlags::new(config);
//! if flags.is_enabled_for_user(FeatureFlag::EnableSmartCache, "user123") {
//!     // Enable smart cache for this user
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// All feature flags supported by ltmatrix
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFlag {
    // Agent Backend Features
    /// Enable Claude Opus backend (experimental)
    EnableClaudeOpusBackend,

    /// Enable OpenCode backend
    EnableOpenCodeBackend,

    /// Enable KimiCode backend
    EnableKimiCodeBackend,

    /// Enable Codex backend
    EnableCodexBackend,

    /// Enable custom agent backend support
    EnableCustomBackend,

    // Pipeline Stage Features
    /// Enable parallel task execution
    EnableParallelExecution,

    /// Enable smart caching for intermediate results
    EnableSmartCache,

    /// Enable incremental builds (only rebuild changed components)
    EnableIncrementalBuilds,

    /// Enable distributed task execution across machines
    EnableDistributedTasks,

    /// Enable experimental task dependency resolution
    EnableTaskDependencyGraph,

    /// Enable task batching for efficiency
    EnableTaskBatching,

    /// Enable pipeline optimization passes
    EnablePipelineOptimization,

    // Scheduler Features
    /// Enable priority-based scheduler
    EnablePriorityScheduler,

    /// Enable adaptive scheduler (adjusts based on performance)
    EnableAdaptiveScheduler,

    /// Enable ML-based scheduler (learns optimal scheduling)
    EnableMlScheduler,

    /// Enable fair-share scheduler (ensures equal resource allocation)
    EnableFairShareScheduler,

    /// Enable deadline-aware scheduler
    EnableDeadlineScheduler,

    // Monitoring & Observability Features
    /// Enable detailed metrics collection
    EnableDetailedMetrics,

    /// Enable performance profiling
    EnableProfiling,

    /// Enable real-time monitoring dashboard
    EnableMonitoringDashboard,

    /// Enable automated alerting
    EnableAlerting,

    // Development & Debugging Features
    /// Enable verbose debug output
    EnableVerboseDebug,

    /// Enable tracing of all operations
    EnableTracing,

    /// Enable experimental CLI commands
    EnableExperimentalCommands,

    /// Enable testing utilities in production
    EnableTestingUtilities,
}

impl FeatureFlag {
    /// Get the configuration key for this feature flag
    #[must_use]
    pub fn config_key(&self) -> &'static str {
        match self {
            // Agent Backend Features
            FeatureFlag::EnableClaudeOpusBackend => "enable_claude_opus_backend",
            FeatureFlag::EnableOpenCodeBackend => "enable_opencode_backend",
            FeatureFlag::EnableKimiCodeBackend => "enable_kimicode_backend",
            FeatureFlag::EnableCodexBackend => "enable_codex_backend",
            FeatureFlag::EnableCustomBackend => "enable_custom_backend",

            // Pipeline Stage Features
            FeatureFlag::EnableParallelExecution => "enable_parallel_execution",
            FeatureFlag::EnableSmartCache => "enable_smart_cache",
            FeatureFlag::EnableIncrementalBuilds => "enable_incremental_builds",
            FeatureFlag::EnableDistributedTasks => "enable_distributed_tasks",
            FeatureFlag::EnableTaskDependencyGraph => "enable_task_dependency_graph",
            FeatureFlag::EnableTaskBatching => "enable_task_batching",
            FeatureFlag::EnablePipelineOptimization => "enable_pipeline_optimization",

            // Scheduler Features
            FeatureFlag::EnablePriorityScheduler => "enable_priority_scheduler",
            FeatureFlag::EnableAdaptiveScheduler => "enable_adaptive_scheduler",
            FeatureFlag::EnableMlScheduler => "enable_ml_scheduler",
            FeatureFlag::EnableFairShareScheduler => "enable_fair_share_scheduler",
            FeatureFlag::EnableDeadlineScheduler => "enable_deadline_scheduler",

            // Monitoring & Observability Features
            FeatureFlag::EnableDetailedMetrics => "enable_detailed_metrics",
            FeatureFlag::EnableProfiling => "enable_profiling",
            FeatureFlag::EnableMonitoringDashboard => "enable_monitoring_dashboard",
            FeatureFlag::EnableAlerting => "enable_alerting",

            // Development & Debugging Features
            FeatureFlag::EnableVerboseDebug => "enable_verbose_debug",
            FeatureFlag::EnableTracing => "enable_tracing",
            FeatureFlag::EnableExperimentalCommands => "enable_experimental_commands",
            FeatureFlag::EnableTestingUtilities => "enable_testing_utilities",
        }
    }

    /// Get a human-readable description of this feature flag
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            // Agent Backend Features
            FeatureFlag::EnableClaudeOpusBackend => {
                "Enable Claude Opus backend for complex reasoning tasks"
            }
            FeatureFlag::EnableOpenCodeBackend => {
                "Enable OpenCode backend as an alternative to Claude"
            }
            FeatureFlag::EnableKimiCodeBackend => "Enable KimiCode backend for specialized tasks",
            FeatureFlag::EnableCodexBackend => "Enable Codex backend for code generation",
            FeatureFlag::EnableCustomBackend => "Enable support for custom agent backends",

            // Pipeline Stage Features
            FeatureFlag::EnableParallelExecution => {
                "Enable parallel execution of independent tasks"
            }
            FeatureFlag::EnableSmartCache => "Enable intelligent caching of intermediate results",
            FeatureFlag::EnableIncrementalBuilds => {
                "Enable incremental builds (only rebuild changed components)"
            }
            FeatureFlag::EnableDistributedTasks => {
                "Enable distributed task execution across machines"
            }
            FeatureFlag::EnableTaskDependencyGraph => {
                "Enable experimental task dependency resolution"
            }
            FeatureFlag::EnableTaskBatching => "Enable task batching for improved efficiency",
            FeatureFlag::EnablePipelineOptimization => {
                "Enable automated pipeline optimization passes"
            }

            // Scheduler Features
            FeatureFlag::EnablePriorityScheduler => "Enable priority-based task scheduling",
            FeatureFlag::EnableAdaptiveScheduler => {
                "Enable adaptive scheduler that adjusts based on performance"
            }
            FeatureFlag::EnableMlScheduler => {
                "Enable ML-based scheduler that learns optimal scheduling"
            }
            FeatureFlag::EnableFairShareScheduler => {
                "Enable fair-share scheduler for equal resource allocation"
            }
            FeatureFlag::EnableDeadlineScheduler => {
                "Enable deadline-aware scheduler for time-sensitive tasks"
            }

            // Monitoring & Observability Features
            FeatureFlag::EnableDetailedMetrics => {
                "Enable detailed metrics collection for performance analysis"
            }
            FeatureFlag::EnableProfiling => "Enable performance profiling capabilities",
            FeatureFlag::EnableMonitoringDashboard => "Enable real-time monitoring dashboard",
            FeatureFlag::EnableAlerting => "Enable automated alerting for critical events",

            // Development & Debugging Features
            FeatureFlag::EnableVerboseDebug => "Enable verbose debug output for troubleshooting",
            FeatureFlag::EnableTracing => "Enable detailed tracing of all operations",
            FeatureFlag::EnableExperimentalCommands => {
                "Enable experimental CLI commands (may be unstable)"
            }
            FeatureFlag::EnableTestingUtilities => {
                "Enable testing utilities in production environment"
            }
        }
    }

    /// Check if this feature flag is experimental (unstable, may change)
    #[must_use]
    pub fn is_experimental(&self) -> bool {
        matches!(
            self,
            FeatureFlag::EnableClaudeOpusBackend
                | FeatureFlag::EnableCustomBackend
                | FeatureFlag::EnableDistributedTasks
                | FeatureFlag::EnableTaskDependencyGraph
                | FeatureFlag::EnableMlScheduler
                | FeatureFlag::EnableMonitoringDashboard
                | FeatureFlag::EnableExperimentalCommands
                | FeatureFlag::EnableTestingUtilities
        )
    }

    /// Check if this feature flag is stable (safe for production use)
    #[must_use]
    pub fn is_stable(&self) -> bool {
        !self.is_experimental()
    }
}

/// Gradual rollout configuration for a feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutConfig {
    /// Percentage of users who should have this feature enabled (0-100)
    #[serde(default = "default_rollout_percentage")]
    pub percentage: u32,

    /// Whitelist of specific users who should always have the feature
    #[serde(default)]
    pub users: HashSet<String>,

    /// Blacklist of specific users who should never have the feature
    #[serde(default)]
    pub excluded_users: HashSet<String>,
}

impl Default for RolloutConfig {
    fn default() -> Self {
        RolloutConfig {
            percentage: default_rollout_percentage(),
            users: HashSet::new(),
            excluded_users: HashSet::new(),
        }
    }
}

impl RolloutConfig {
    /// Create a new rollout configuration with the specified percentage
    #[must_use]
    pub fn new(percentage: u32) -> Self {
        RolloutConfig {
            percentage: percentage.min(100),
            ..Default::default()
        }
    }

    /// Add a user to the whitelist
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.users.insert(user.into());
        self
    }

    /// Add a user to the blacklist
    pub fn with_excluded_user(mut self, user: impl Into<String>) -> Self {
        self.excluded_users.insert(user.into());
        self
    }

    /// Check if a user should have this feature based on rollout configuration
    #[must_use]
    pub fn is_enabled_for(&self, user_id: &str) -> bool {
        // Check blacklist first (highest priority)
        if self.excluded_users.contains(user_id) {
            return false;
        }

        // Check whitelist (always enabled for these users)
        if self.users.contains(user_id) {
            return true;
        }

        // Use percentage-based rollout (hash-based for consistency)
        if self.percentage >= 100 {
            return true;
        }
        if self.percentage == 0 {
            return false;
        }

        // Use a simple hash-based approach for consistent rollout
        let hash = self.hash_user_id(user_id);
        (hash % 100) < self.percentage
    }

    /// Hash a user ID to a consistent value for percentage-based rollout
    fn hash_user_id(&self, user_id: &str) -> u32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        (hasher.finish() % u32::MAX as u64) as u32
    }
}

fn default_rollout_percentage() -> u32 {
    0 // Default to disabled
}

/// Feature flag configuration from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Agent backend feature flags
    #[serde(default, alias = "agent_backends")]
    pub agent_backend: AgentBackendFeatures,

    /// Pipeline stage feature flags
    #[serde(default, alias = "pipeline")]
    pub pipeline: PipelineFeatures,

    /// Scheduler feature flags
    #[serde(default)]
    pub scheduler: SchedulerFeatures,

    /// Monitoring and observability feature flags
    #[serde(default)]
    pub monitoring: MonitoringFeatures,

    /// Development and debugging feature flags
    #[serde(default)]
    pub development: DevelopmentFeatures,

    /// Gradual rollout configuration
    #[serde(default)]
    pub rollout: HashMap<String, RolloutConfig>,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        FeatureConfig {
            agent_backend: AgentBackendFeatures::default(),
            pipeline: PipelineFeatures::default(),
            scheduler: SchedulerFeatures::default(),
            monitoring: MonitoringFeatures::default(),
            development: DevelopmentFeatures::default(),
            rollout: HashMap::new(),
        }
    }
}

/// Agent backend feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBackendFeatures {
    /// Enable Claude Opus backend
    #[serde(default)]
    pub enable_claude_opus_backend: bool,

    /// Enable OpenCode backend
    #[serde(default)]
    pub enable_opencode_backend: bool,

    /// Enable KimiCode backend
    #[serde(default)]
    pub enable_kimicode_backend: bool,

    /// Enable Codex backend
    #[serde(default)]
    pub enable_codex_backend: bool,

    /// Enable custom backend support
    #[serde(default)]
    pub enable_custom_backend: bool,
}

impl Default for AgentBackendFeatures {
    fn default() -> Self {
        AgentBackendFeatures {
            enable_claude_opus_backend: false, // Experimental
            enable_opencode_backend: false,    // Opt-in
            enable_kimicode_backend: false,    // Opt-in
            enable_codex_backend: false,       // Opt-in
            enable_custom_backend: false,      // Experimental
        }
    }
}

/// Pipeline stage feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineFeatures {
    /// Enable parallel execution
    #[serde(default = "default_true")]
    pub enable_parallel_execution: bool,

    /// Enable smart cache
    #[serde(default = "default_true")]
    pub enable_smart_cache: bool,

    /// Enable incremental builds
    #[serde(default)]
    pub enable_incremental_builds: bool,

    /// Enable distributed tasks
    #[serde(default)]
    pub enable_distributed_tasks: bool,

    /// Enable task dependency graph
    #[serde(default)]
    pub enable_task_dependency_graph: bool,

    /// Enable task batching
    #[serde(default)]
    pub enable_task_batching: bool,

    /// Enable pipeline optimization
    #[serde(default)]
    pub enable_pipeline_optimization: bool,
}

impl Default for PipelineFeatures {
    fn default() -> Self {
        PipelineFeatures {
            enable_parallel_execution: true,     // Stable
            enable_smart_cache: true,            // Stable
            enable_incremental_builds: false,    // Beta
            enable_distributed_tasks: false,     // Experimental
            enable_task_dependency_graph: false, // Experimental
            enable_task_batching: false,         // Beta
            enable_pipeline_optimization: false, // Beta
        }
    }
}

/// Scheduler feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerFeatures {
    /// Enable priority scheduler
    #[serde(default)]
    pub enable_priority_scheduler: bool,

    /// Enable adaptive scheduler
    #[serde(default)]
    pub enable_adaptive_scheduler: bool,

    /// Enable ML scheduler
    #[serde(default)]
    pub enable_ml_scheduler: bool,

    /// Enable fair-share scheduler
    #[serde(default)]
    pub enable_fair_share_scheduler: bool,

    /// Enable deadline scheduler
    #[serde(default)]
    pub enable_deadline_scheduler: bool,
}

impl Default for SchedulerFeatures {
    fn default() -> Self {
        SchedulerFeatures {
            enable_priority_scheduler: false,   // Beta
            enable_adaptive_scheduler: false,   // Beta
            enable_ml_scheduler: false,         // Experimental
            enable_fair_share_scheduler: false, // Beta
            enable_deadline_scheduler: false,   // Beta
        }
    }
}

/// Monitoring and observability feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringFeatures {
    /// Enable detailed metrics
    #[serde(default)]
    pub enable_detailed_metrics: bool,

    /// Enable profiling
    #[serde(default)]
    pub enable_profiling: bool,

    /// Enable monitoring dashboard
    #[serde(default)]
    pub enable_monitoring_dashboard: bool,

    /// Enable alerting
    #[serde(default)]
    pub enable_alerting: bool,
}

impl Default for MonitoringFeatures {
    fn default() -> Self {
        MonitoringFeatures {
            enable_detailed_metrics: false,     // Opt-in
            enable_profiling: false,            // Opt-in
            enable_monitoring_dashboard: false, // Experimental
            enable_alerting: false,             // Beta
        }
    }
}

/// Development and debugging feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentFeatures {
    /// Enable verbose debug output
    #[serde(default)]
    pub enable_verbose_debug: bool,

    /// Enable tracing
    #[serde(default)]
    pub enable_tracing: bool,

    /// Enable experimental commands
    #[serde(default)]
    pub enable_experimental_commands: bool,

    /// Enable testing utilities
    #[serde(default)]
    pub enable_testing_utilities: bool,
}

impl Default for DevelopmentFeatures {
    fn default() -> Self {
        DevelopmentFeatures {
            enable_verbose_debug: false,         // Opt-in
            enable_tracing: false,               // Opt-in
            enable_experimental_commands: false, // Experimental
            enable_testing_utilities: false,     // Experimental
        }
    }
}

fn default_true() -> bool {
    true
}

/// Feature flag manager
///
/// This struct manages all feature flags and provides methods to check
/// if features are enabled.
#[derive(Debug, Clone)]
pub struct FeatureFlags {
    config: FeatureConfig,
}

impl FeatureFlags {
    /// Create a new feature flags manager from configuration
    #[must_use]
    pub fn new(config: FeatureConfig) -> Self {
        FeatureFlags { config }
    }

    /// Create feature flags with all features disabled
    #[must_use]
    pub fn all_disabled() -> Self {
        let mut config = FeatureConfig::default();
        // Explicitly disable all features
        config.agent_backend = AgentBackendFeatures::default();
        config.pipeline = PipelineFeatures {
            enable_parallel_execution: false,
            enable_smart_cache: false,
            enable_incremental_builds: false,
            enable_distributed_tasks: false,
            enable_task_dependency_graph: false,
            enable_task_batching: false,
            enable_pipeline_optimization: false,
        };
        config.scheduler = SchedulerFeatures::default();
        config.monitoring = MonitoringFeatures::default();
        config.development = DevelopmentFeatures::default();
        FeatureFlags { config }
    }

    /// Create feature flags with all stable features enabled
    #[must_use]
    pub fn stable_enabled() -> Self {
        let mut config = FeatureConfig::default();
        config.pipeline.enable_parallel_execution = true;
        config.pipeline.enable_smart_cache = true;
        FeatureFlags { config }
    }

    /// Check if a specific feature flag is enabled
    #[must_use]
    pub fn is_enabled(&self, flag: FeatureFlag) -> bool {
        match flag {
            // Agent Backend Features
            FeatureFlag::EnableClaudeOpusBackend => {
                self.config.agent_backend.enable_claude_opus_backend
            }
            FeatureFlag::EnableOpenCodeBackend => self.config.agent_backend.enable_opencode_backend,
            FeatureFlag::EnableKimiCodeBackend => self.config.agent_backend.enable_kimicode_backend,
            FeatureFlag::EnableCodexBackend => self.config.agent_backend.enable_codex_backend,
            FeatureFlag::EnableCustomBackend => self.config.agent_backend.enable_custom_backend,

            // Pipeline Stage Features
            FeatureFlag::EnableParallelExecution => self.config.pipeline.enable_parallel_execution,
            FeatureFlag::EnableSmartCache => self.config.pipeline.enable_smart_cache,
            FeatureFlag::EnableIncrementalBuilds => self.config.pipeline.enable_incremental_builds,
            FeatureFlag::EnableDistributedTasks => self.config.pipeline.enable_distributed_tasks,
            FeatureFlag::EnableTaskDependencyGraph => {
                self.config.pipeline.enable_task_dependency_graph
            }
            FeatureFlag::EnableTaskBatching => self.config.pipeline.enable_task_batching,
            FeatureFlag::EnablePipelineOptimization => {
                self.config.pipeline.enable_pipeline_optimization
            }

            // Scheduler Features
            FeatureFlag::EnablePriorityScheduler => self.config.scheduler.enable_priority_scheduler,
            FeatureFlag::EnableAdaptiveScheduler => self.config.scheduler.enable_adaptive_scheduler,
            FeatureFlag::EnableMlScheduler => self.config.scheduler.enable_ml_scheduler,
            FeatureFlag::EnableFairShareScheduler => {
                self.config.scheduler.enable_fair_share_scheduler
            }
            FeatureFlag::EnableDeadlineScheduler => self.config.scheduler.enable_deadline_scheduler,

            // Monitoring & Observability Features
            FeatureFlag::EnableDetailedMetrics => self.config.monitoring.enable_detailed_metrics,
            FeatureFlag::EnableProfiling => self.config.monitoring.enable_profiling,
            FeatureFlag::EnableMonitoringDashboard => {
                self.config.monitoring.enable_monitoring_dashboard
            }
            FeatureFlag::EnableAlerting => self.config.monitoring.enable_alerting,

            // Development & Debugging Features
            FeatureFlag::EnableVerboseDebug => self.config.development.enable_verbose_debug,
            FeatureFlag::EnableTracing => self.config.development.enable_tracing,
            FeatureFlag::EnableExperimentalCommands => {
                self.config.development.enable_experimental_commands
            }
            FeatureFlag::EnableTestingUtilities => self.config.development.enable_testing_utilities,
        }
    }

    /// Check if a feature is enabled for a specific user (with gradual rollout)
    #[must_use]
    pub fn is_enabled_for_user(&self, flag: FeatureFlag, user_id: &str) -> bool {
        // First check if the feature is globally enabled
        if !self.is_enabled(flag) {
            return false;
        }

        // Check for rollout configuration
        let config_key = flag.config_key();
        if let Some(rollout) = self.config.rollout.get(config_key) {
            return rollout.is_enabled_for(user_id);
        }

        // No rollout config, feature is enabled for everyone
        true
    }

    /// Get the rollout configuration for a feature flag
    #[must_use]
    pub fn rollout_config(&self, flag: FeatureFlag) -> Option<&RolloutConfig> {
        self.config.rollout.get(flag.config_key())
    }

    /// Get all enabled feature flags
    #[must_use]
    pub fn enabled_flags(&self) -> Vec<FeatureFlag> {
        use FeatureFlag::*;

        let mut flags = Vec::new();

        // Agent Backend Features
        if self.is_enabled(EnableClaudeOpusBackend) {
            flags.push(EnableClaudeOpusBackend);
        }
        if self.is_enabled(EnableOpenCodeBackend) {
            flags.push(EnableOpenCodeBackend);
        }
        if self.is_enabled(EnableKimiCodeBackend) {
            flags.push(EnableKimiCodeBackend);
        }
        if self.is_enabled(EnableCodexBackend) {
            flags.push(EnableCodexBackend);
        }
        if self.is_enabled(EnableCustomBackend) {
            flags.push(EnableCustomBackend);
        }

        // Pipeline Stage Features
        if self.is_enabled(EnableParallelExecution) {
            flags.push(EnableParallelExecution);
        }
        if self.is_enabled(EnableSmartCache) {
            flags.push(EnableSmartCache);
        }
        if self.is_enabled(EnableIncrementalBuilds) {
            flags.push(EnableIncrementalBuilds);
        }
        if self.is_enabled(EnableDistributedTasks) {
            flags.push(EnableDistributedTasks);
        }
        if self.is_enabled(EnableTaskDependencyGraph) {
            flags.push(EnableTaskDependencyGraph);
        }
        if self.is_enabled(EnableTaskBatching) {
            flags.push(EnableTaskBatching);
        }
        if self.is_enabled(EnablePipelineOptimization) {
            flags.push(EnablePipelineOptimization);
        }

        // Scheduler Features
        if self.is_enabled(EnablePriorityScheduler) {
            flags.push(EnablePriorityScheduler);
        }
        if self.is_enabled(EnableAdaptiveScheduler) {
            flags.push(EnableAdaptiveScheduler);
        }
        if self.is_enabled(EnableMlScheduler) {
            flags.push(EnableMlScheduler);
        }
        if self.is_enabled(EnableFairShareScheduler) {
            flags.push(EnableFairShareScheduler);
        }
        if self.is_enabled(EnableDeadlineScheduler) {
            flags.push(EnableDeadlineScheduler);
        }

        // Monitoring & Observability Features
        if self.is_enabled(EnableDetailedMetrics) {
            flags.push(EnableDetailedMetrics);
        }
        if self.is_enabled(EnableProfiling) {
            flags.push(EnableProfiling);
        }
        if self.is_enabled(EnableMonitoringDashboard) {
            flags.push(EnableMonitoringDashboard);
        }
        if self.is_enabled(EnableAlerting) {
            flags.push(EnableAlerting);
        }

        // Development & Debugging Features
        if self.is_enabled(EnableVerboseDebug) {
            flags.push(EnableVerboseDebug);
        }
        if self.is_enabled(EnableTracing) {
            flags.push(EnableTracing);
        }
        if self.is_enabled(EnableExperimentalCommands) {
            flags.push(EnableExperimentalCommands);
        }
        if self.is_enabled(EnableTestingUtilities) {
            flags.push(EnableTestingUtilities);
        }

        flags
    }

    /// Get all experimental flags that are enabled
    #[must_use]
    pub fn enabled_experimental_flags(&self) -> Vec<FeatureFlag> {
        self.enabled_flags()
            .into_iter()
            .filter(|flag| flag.is_experimental())
            .collect()
    }

    /// Load feature flags from a TOML file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read feature flag config: {}", path.display()))?;

        let config: FeatureConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse feature flag config: {}", path.display()))?;

        Ok(FeatureFlags { config })
    }

    /// Save feature flags to a TOML file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(&self.config)
            .context("Failed to serialize feature flag config")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write feature flag config: {}", path.display()))?;

        Ok(())
    }

    /// Get a reference to the underlying configuration
    #[must_use]
    pub fn config(&self) -> &FeatureConfig {
        &self.config
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        FeatureFlags::stable_enabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flag_config_key() {
        assert_eq!(
            FeatureFlag::EnableParallelExecution.config_key(),
            "enable_parallel_execution"
        );
        assert_eq!(
            FeatureFlag::EnableClaudeOpusBackend.config_key(),
            "enable_claude_opus_backend"
        );
    }

    #[test]
    fn test_feature_flag_description() {
        let desc = FeatureFlag::EnableParallelExecution.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("parallel"));
    }

    #[test]
    fn test_feature_flag_is_experimental() {
        assert!(FeatureFlag::EnableClaudeOpusBackend.is_experimental());
        assert!(FeatureFlag::EnableDistributedTasks.is_experimental());
        assert!(!FeatureFlag::EnableParallelExecution.is_experimental());
        assert!(!FeatureFlag::EnableSmartCache.is_experimental());
    }

    #[test]
    fn test_rollout_config_new() {
        let config = RolloutConfig::new(50);
        assert_eq!(config.percentage, 50);
        assert!(config.users.is_empty());
        assert!(config.excluded_users.is_empty());
    }

    #[test]
    fn test_rollout_config_with_users() {
        let config = RolloutConfig::new(50)
            .with_user("user1")
            .with_user("user2")
            .with_excluded_user("user3");

        assert!(config.users.contains("user1"));
        assert!(config.users.contains("user2"));
        assert!(config.excluded_users.contains("user3"));
    }

    #[test]
    fn test_rollout_config_whitelist() {
        let config = RolloutConfig::new(0).with_user("user1");

        // user1 is whitelisted, should be enabled even at 0%
        assert!(config.is_enabled_for("user1"));
        // user2 is not whitelisted, should be disabled at 0%
        assert!(!config.is_enabled_for("user2"));
    }

    #[test]
    fn test_rollout_config_blacklist() {
        let config = RolloutConfig::new(100).with_excluded_user("user1");

        // user1 is blacklisted, should be disabled even at 100%
        assert!(!config.is_enabled_for("user1"));
        // user2 is not blacklisted, should be enabled at 100%
        assert!(config.is_enabled_for("user2"));
    }

    #[test]
    fn test_rollout_config_percentage() {
        let config = RolloutConfig::new(0);

        // At 0%, should be disabled for everyone not in whitelist
        assert!(!config.is_enabled_for("user1"));
        assert!(!config.is_enabled_for("user2"));

        // At 100%, should be enabled for everyone not in blacklist
        let config = RolloutConfig::new(100);
        assert!(config.is_enabled_for("user1"));
        assert!(config.is_enabled_for("user2"));
    }

    #[test]
    fn test_rollout_config_consistent_hashing() {
        let config = RolloutConfig::new(50);

        // Same user should get the same result every time
        let result1 = config.is_enabled_for("test_user");
        let result2 = config.is_enabled_for("test_user");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_feature_flags_all_disabled() {
        let flags = FeatureFlags::all_disabled();

        assert!(!flags.is_enabled(FeatureFlag::EnableParallelExecution));
        assert!(!flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend));
        assert!(!flags.is_enabled(FeatureFlag::EnablePriorityScheduler));
    }

    #[test]
    fn test_feature_flags_stable_enabled() {
        let flags = FeatureFlags::stable_enabled();

        // Stable features should be enabled
        assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));
        assert!(flags.is_enabled(FeatureFlag::EnableSmartCache));

        // Experimental features should be disabled
        assert!(!flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend));
        assert!(!flags.is_enabled(FeatureFlag::EnableDistributedTasks));
    }

    #[test]
    fn test_feature_flags_enabled_flags() {
        let mut config = FeatureConfig::default();
        config.pipeline.enable_parallel_execution = true;
        config.pipeline.enable_smart_cache = false;
        config.agent_backend.enable_claude_opus_backend = true;

        let flags = FeatureFlags::new(config);
        let enabled = flags.enabled_flags();

        assert!(enabled.contains(&FeatureFlag::EnableParallelExecution));
        assert!(enabled.contains(&FeatureFlag::EnableClaudeOpusBackend));
        assert!(!enabled.contains(&FeatureFlag::EnableSmartCache));
    }

    #[test]
    fn test_feature_flags_enabled_experimental_flags() {
        let mut config = FeatureConfig::default();
        config.agent_backend.enable_claude_opus_backend = true;
        config.pipeline.enable_parallel_execution = true;

        let flags = FeatureFlags::new(config);
        let experimental = flags.enabled_experimental_flags();

        assert!(experimental.contains(&FeatureFlag::EnableClaudeOpusBackend));
        assert!(!experimental.contains(&FeatureFlag::EnableParallelExecution));
    }

    #[test]
    fn test_feature_flags_is_enabled_for_user() {
        let mut config = FeatureConfig::default();
        config.pipeline.enable_parallel_execution = true;
        config.scheduler.enable_priority_scheduler = true;

        // Add rollout config for priority scheduler
        let rollout = RolloutConfig::new(0).with_user("user1");
        config.rollout.insert(
            FeatureFlag::EnablePriorityScheduler
                .config_key()
                .to_string(),
            rollout,
        );

        let flags = FeatureFlags::new(config);

        // Parallel execution has no rollout, should be enabled for everyone
        assert!(flags.is_enabled_for_user(FeatureFlag::EnableParallelExecution, "user1"));
        assert!(flags.is_enabled_for_user(FeatureFlag::EnableParallelExecution, "user2"));

        // Priority scheduler has rollout at 0% with user1 whitelisted
        assert!(flags.is_enabled_for_user(FeatureFlag::EnablePriorityScheduler, "user1"));
        assert!(!flags.is_enabled_for_user(FeatureFlag::EnablePriorityScheduler, "user2"));
    }

    #[test]
    fn test_feature_config_default() {
        let config = FeatureConfig::default();

        // Check stable defaults
        assert!(config.pipeline.enable_parallel_execution);
        assert!(config.pipeline.enable_smart_cache);

        // Check experimental defaults (disabled)
        assert!(!config.agent_backend.enable_claude_opus_backend);
        assert!(!config.pipeline.enable_distributed_tasks);
        assert!(!config.scheduler.enable_ml_scheduler);
    }

    #[test]
    fn test_parse_feature_config_from_toml() {
        let toml_str = r#"
[agent_backend]
enable_claude_opus_backend = true
enable_opencode_backend = false

[pipeline]
enable_parallel_execution = true
enable_smart_cache = true
enable_incremental_builds = false

[scheduler]
enable_priority_scheduler = true

[monitoring]
enable_detailed_metrics = true

[development]
enable_verbose_debug = false
"#;

        let config: FeatureConfig = toml::from_str(toml_str).unwrap();

        assert!(config.agent_backend.enable_claude_opus_backend);
        assert!(!config.agent_backend.enable_opencode_backend);
        assert!(config.pipeline.enable_parallel_execution);
        assert!(config.pipeline.enable_smart_cache);
        assert!(!config.pipeline.enable_incremental_builds);
        assert!(config.scheduler.enable_priority_scheduler);
        assert!(config.monitoring.enable_detailed_metrics);
        assert!(!config.development.enable_verbose_debug);
    }

    #[test]
    fn test_parse_rollout_config_from_toml() {
        let toml_str = r#"
[rollout.enable_parallel_execution]
percentage = 50
users = ["user1", "user2"]
excluded_users = ["user3"]

[rollout.enable_smart_cache]
percentage = 100
"#;

        let config: FeatureConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.rollout.len(), 2);

        let parallel = &config.rollout["enable_parallel_execution"];
        assert_eq!(parallel.percentage, 50);
        assert!(parallel.users.contains("user1"));
        assert!(parallel.users.contains("user2"));
        assert!(parallel.excluded_users.contains("user3"));

        let smart_cache = &config.rollout["enable_smart_cache"];
        assert_eq!(smart_cache.percentage, 100);
    }

    #[test]
    fn test_feature_flags_roundtrip() {
        let mut config = FeatureConfig::default();
        config.agent_backend.enable_claude_opus_backend = true;
        config.pipeline.enable_parallel_execution = true;
        config.scheduler.enable_priority_scheduler = true;

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Deserialize back
        let deserialized: FeatureConfig = toml::from_str(&toml_str).unwrap();

        // Should be identical
        assert_eq!(
            deserialized.agent_backend.enable_claude_opus_backend,
            config.agent_backend.enable_claude_opus_backend
        );
        assert_eq!(
            deserialized.pipeline.enable_parallel_execution,
            config.pipeline.enable_parallel_execution
        );
        assert_eq!(
            deserialized.scheduler.enable_priority_scheduler,
            config.scheduler.enable_priority_scheduler
        );
    }
}
