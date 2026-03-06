//! Configuration settings for ltmatrix
//!
//! This module handles loading configuration from TOML files and merging
//! configuration from multiple sources (global config, project config, CLI args).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::feature::FeatureConfig;
use crate::models::Agent;

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default agent to use
    #[serde(default)]
    pub default: Option<String>,

    /// Agent configurations
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,

    /// Mode-specific configurations
    #[serde(default)]
    pub modes: ModeConfigs,

    /// Output settings
    #[serde(default)]
    pub output: OutputConfig,

    /// Logging settings
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Feature flag configuration
    #[serde(default)]
    pub features: FeatureConfig,

    /// Warmup configuration
    #[serde(default)]
    pub warmup: WarmupConfig,

    /// Session pool configuration
    #[serde(default)]
    pub pool: PoolConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default: Some("claude".to_string()),
            agents: HashMap::new(),
            modes: ModeConfigs::default(),
            output: OutputConfig::default(),
            logging: LoggingConfig::default(),
            features: FeatureConfig::default(),
            warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        }
    }
}

/// Agent configuration from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Command to invoke the agent
    pub command: Option<String>,

    /// Model identifier
    pub model: Option<String>,

    /// Timeout in seconds
    pub timeout: Option<u64>,
}

/// Mode-specific configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfigs {
    /// Fast mode configuration
    #[serde(default)]
    pub fast: Option<ModeConfig>,

    /// Standard mode configuration
    #[serde(default)]
    pub standard: Option<ModeConfig>,

    /// Expert mode configuration
    #[serde(default)]
    pub expert: Option<ModeConfig>,
}

impl Default for ModeConfigs {
    fn default() -> Self {
        ModeConfigs {
            fast: None,
            standard: None,
            expert: None,
        }
    }
}

/// Configuration for a specific execution mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    /// Model to use
    pub model: Option<String>,

    /// Whether to run tests
    #[serde(default)]
    pub run_tests: bool,

    /// Whether to verify task completion
    #[serde(default)]
    pub verify: bool,

    /// Maximum number of retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Maximum depth for task decomposition
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,

    /// Timeout for planning stage (seconds)
    #[serde(default = "default_timeout_plan")]
    pub timeout_plan: u64,

    /// Timeout for execution stage (seconds)
    #[serde(default = "default_timeout_exec")]
    pub timeout_exec: u64,
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Default output format
    #[serde(default)]
    pub format: OutputFormat,

    /// Whether to use colors in output
    #[serde(default = "default_true")]
    pub colored: bool,

    /// Whether to show progress bars
    #[serde(default = "default_true")]
    pub progress: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Text
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig {
            format: OutputFormat::default(),
            colored: true,
            progress: true,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default)]
    pub level: LogLevel,

    /// Log file path (optional)
    pub file: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: LogLevel::default(),
            file: None,
        }
    }
}

/// Warmup configuration for agent sessions
///
/// Warmup queries can be sent to agents before actual tasks to initialize
/// sessions and reduce latency for the first real task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmupConfig {
    /// Whether warmup is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Maximum number of warmup queries to send
    #[serde(default = "default_max_queries")]
    pub max_queries: u32,

    /// Timeout for warmup queries in seconds
    #[serde(default = "default_warmup_timeout")]
    pub timeout_seconds: u64,

    /// Whether to retry warmup on failure
    #[serde(default)]
    pub retry_on_failure: bool,

    /// Custom prompt template for warmup queries
    pub prompt_template: Option<String>,
}

impl Default for WarmupConfig {
    fn default() -> Self {
        WarmupConfig {
            enabled: false,
            max_queries: 3,
            timeout_seconds: 30,
            retry_on_failure: false,
            prompt_template: None,
        }
    }
}

impl WarmupConfig {
    /// Validate the warmup configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_queries == 0 {
            return Err("max_queries must be greater than 0".to_string());
        }

        if self.timeout_seconds == 0 {
            return Err("timeout_seconds must be greater than 0".to_string());
        }

        if let Some(ref template) = self.prompt_template {
            if template.trim().is_empty() {
                return Err("prompt_template cannot be empty".to_string());
            }
        }

        Ok(())
    }
}

/// Session pool configuration
///
/// Controls how agent sessions are managed, reused, and cleaned up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of sessions to keep in the pool
    #[serde(default = "default_max_pool_sessions")]
    pub max_sessions: usize,

    /// Whether to automatically clean up stale sessions
    #[serde(default = "default_true")]
    pub auto_cleanup: bool,

    /// Interval in seconds between automatic cleanup runs
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval_seconds: u64,

    /// Session staleness threshold in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_stale_threshold")]
    pub stale_threshold_seconds: u64,

    /// Whether to enable session reuse across tasks
    #[serde(default = "default_true")]
    pub enable_reuse: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        PoolConfig {
            max_sessions: 100,
            auto_cleanup: true,
            cleanup_interval_seconds: 300, // 5 minutes
            stale_threshold_seconds: 3600, // 1 hour
            enable_reuse: true,
        }
    }
}

impl PoolConfig {
    /// Validate the pool configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_sessions == 0 {
            return Err("max_sessions must be greater than 0".to_string());
        }

        if self.cleanup_interval_seconds == 0 {
            return Err("cleanup_interval_seconds must be greater than 0".to_string());
        }

        if self.stale_threshold_seconds == 0 {
            return Err("stale_threshold_seconds must be greater than 0".to_string());
        }

        Ok(())
    }

    /// Get the stale threshold as a chrono Duration
    pub fn stale_threshold_duration(&self) -> chrono::Duration {
        chrono::Duration::seconds(self.stale_threshold_seconds as i64)
    }

    /// Get the cleanup interval as a tokio Duration
    pub fn cleanup_interval_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.cleanup_interval_seconds)
    }
}

fn default_max_queries() -> u32 {
    3
}

fn default_warmup_timeout() -> u64 {
    30
}

fn default_max_pool_sessions() -> usize {
    100
}

fn default_cleanup_interval() -> u64 {
    300 // 5 minutes
}

fn default_stale_threshold() -> u64 {
    3600 // 1 hour
}

fn default_true() -> bool {
    true
}

fn default_max_retries() -> u32 {
    3
}

fn default_max_depth() -> u32 {
    3
}

fn default_timeout_plan() -> u64 {
    300 // 5 minutes
}

fn default_timeout_exec() -> u64 {
    3600 // 1 hour
}

/// Loads configuration from a TOML file
///
/// # Arguments
///
/// * `path` - Path to the TOML configuration file
///
/// # Returns
///
/// Returns a `Result` containing the parsed `Config` or an error.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::config::settings::load_config_file;
/// use std::path::Path;
///
/// let config = load_config_file(Path::new("~/.ltmatrix/config.toml"))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn load_config_file(path: &Path) -> Result<Config> {
    debug!("Loading configuration from: {}", path.display());

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse TOML from: {}", path.display()))?;

    debug!("Configuration loaded successfully");
    Ok(config)
}

/// Merges multiple configuration sources with precedence
///
/// Precedence order (highest to lowest):
/// 1. CLI arguments (not handled here)
/// 2. Project config (.ltmatrix/config.toml)
/// 3. Global config (~/.ltmatrix/config.toml)
/// 4. Defaults
///
/// # Arguments
///
/// * `global` - Optional global configuration
/// * `project` - Optional project configuration
///
/// # Returns
///
/// Returns a merged `Config` with all sources combined.
pub fn merge_configs(global: Option<Config>, project: Option<Config>) -> Config {
    let mut merged = Config::default();

    // Apply global config first
    if let Some(global_config) = global {
        merged = merge_config(merged, global_config);
    }

    // Then apply project config (overrides global)
    if let Some(project_config) = project {
        merged = merge_config(merged, project_config);
    }

    merged
}

/// Merges two configurations, with `override_config` taking precedence
fn merge_config(base: Config, override_config: Config) -> Config {
    Config {
        default: override_config.default.or(base.default),
        agents: merge_agent_configs(base.agents, override_config.agents),
        modes: ModeConfigs {
            fast: override_config.modes.fast.or(base.modes.fast),
            standard: override_config.modes.standard.or(base.modes.standard),
            expert: override_config.modes.expert.or(base.modes.expert),
        },
        output: override_config.output,
        logging: override_config.logging,
        features: override_config.features,
        warmup: override_config.warmup,
        pool: override_config.pool,
    }
}

fn merge_agent_configs(
    mut base: HashMap<String, AgentConfig>,
    override_map: HashMap<String, AgentConfig>,
) -> HashMap<String, AgentConfig> {
    for (key, override_agent) in override_map {
        if let Some(base_agent) = base.remove(&key) {
            // Merge individual fields
            let merged = AgentConfig {
                command: override_agent.command.or(base_agent.command),
                model: override_agent.model.or(base_agent.model),
                timeout: override_agent.timeout.or(base_agent.timeout),
            };
            base.insert(key, merged);
        } else {
            base.insert(key, override_agent);
        }
    }
    base
}

/// Gets the global configuration path
///
/// Returns `~/.ltmatrix/config.toml`
pub fn get_global_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to determine home directory")?;

    Ok(home.join(".ltmatrix").join("config.toml"))
}

/// Gets the project configuration path
///
/// Returns `<current_dir>/.ltmatrix/config.toml`
pub fn get_project_config_path() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    Some(current.join(".ltmatrix").join("config.toml"))
}

/// CLI override options for configuration
///
/// These values come from command-line arguments and take highest precedence
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    /// Custom config file path
    pub config_file: Option<PathBuf>,

    /// Agent backend to use
    pub agent: Option<String>,

    /// Execution mode
    pub mode: Option<String>,

    /// Output format
    pub output_format: Option<OutputFormat>,

    /// Log level
    pub log_level: Option<LogLevel>,

    /// Log file path
    pub log_file: Option<PathBuf>,

    /// Maximum retries
    pub max_retries: Option<u32>,

    /// Timeout in seconds
    pub timeout: Option<u64>,

    /// Disable colored output
    pub no_color: Option<bool>,

    /// Generate plan without execution
    pub dry_run: bool,

    /// Resume interrupted work
    pub resume: bool,

    /// Ask for clarification before planning
    pub ask: bool,

    /// Regenerate the plan
    pub regenerate_plan: bool,

    /// Strategy for blocked tasks
    pub on_blocked: Option<String>,

    /// MCP configuration file
    pub mcp_config: Option<PathBuf>,

    /// Show/hide progress bars
    pub progress: Option<bool>,

    /// Override test execution
    pub run_tests: Option<bool>,

    /// Override verification
    pub verify: Option<bool>,
}

impl From<crate::cli::Args> for CliOverrides {
    fn from(args: crate::cli::Args) -> Self {
        // Determine mode string from flags
        let mode = if args.fast {
            Some("fast".to_string())
        } else if args.expert {
            Some("expert".to_string())
        } else if args.mode.is_some() {
            args.mode.map(|m| m.to_string())
        } else {
            None // None means use config file default
        };

        // Convert CLI log level to config log level
        let log_level = args.log_level.map(|cli_level| match cli_level {
            crate::cli::args::LogLevel::Trace => LogLevel::Trace,
            crate::cli::args::LogLevel::Debug => LogLevel::Debug,
            crate::cli::args::LogLevel::Info => LogLevel::Info,
            crate::cli::args::LogLevel::Warn => LogLevel::Warn,
            crate::cli::args::LogLevel::Error => LogLevel::Error,
        });

        // Convert CLI output format to config output format
        let output_format = args.output.map(|cli_format| match cli_format {
            crate::cli::args::OutputFormat::Text => OutputFormat::Text,
            crate::cli::args::OutputFormat::Json => OutputFormat::Json,
            crate::cli::args::OutputFormat::JsonCompact => OutputFormat::Json,
        });

        CliOverrides {
            config_file: args.config,
            agent: args.agent,
            mode,
            output_format,
            log_level,
            log_file: args.log_file,
            max_retries: args.max_retries,
            timeout: args.timeout,
            no_color: if args.no_color { Some(true) } else { None },
            dry_run: args.dry_run,
            resume: args.resume,
            ask: args.ask,
            regenerate_plan: args.regenerate_plan,
            on_blocked: args.on_blocked.map(|s| s.to_string()),
            mcp_config: args.mcp_config,
            progress: None,  // Not currently exposed in CLI
            run_tests: None, // Not currently exposed in CLI
            verify: None,    // Not currently exposed in CLI
        }
    }
}

/// Loads configuration from all available sources
///
/// This function automatically discovers and merges configuration from:
/// - Global config: ~/.ltmatrix/config.toml
/// - Project config: .ltmatrix/config.toml (in current directory)
///
/// # Returns
///
/// Returns a merged `Config` or a default config if no files exist.
pub fn load_config() -> Result<Config> {
    load_config_with_overrides(None)
}

/// Loads configuration with CLI argument overrides
///
/// This is a convenience function that converts CLI arguments to overrides
/// and loads the final merged configuration.
///
/// # Arguments
///
/// * `args` - Parsed CLI arguments
///
/// # Returns
///
/// Returns a merged and validated `Config` with CLI overrides applied.
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::cli::Args;
/// use ltmatrix::config::settings::load_config_from_args;
/// use clap::Parser;
///
/// let args = Args::parse_from(["ltmatrix", "--agent", "claude", "goal"]);
/// let config = load_config_from_args(args)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn load_config_from_args(args: crate::cli::Args) -> Result<Config> {
    let overrides: CliOverrides = args.into();
    load_config_with_overrides(Some(overrides))
}

/// Loads configuration with CLI overrides
///
/// This function merges configuration from multiple sources with proper precedence:
/// 1. CLI overrides (highest)
/// 2. Project config (.ltmatrix/config.toml)
/// 3. Global config (~/.ltmatrix/config.toml)
/// 4. Defaults (lowest)
///
/// # Arguments
///
/// * `overrides` - Optional CLI override values
///
/// # Returns
///
/// Returns a merged and validated `Config`.
/// Loads configuration with CLI overrides
///
/// This function merges configuration from multiple sources with proper precedence:
/// 1. CLI overrides (highest)
/// 2. Custom config file (if specified via --config)
/// 3. Project config (.ltmatrix/config.toml)
/// 4. Global config (~/.ltmatrix/config.toml)
/// 5. Defaults (lowest)
///
/// # Arguments
///
/// * `overrides` - Optional CLI override values
///
/// # Returns
///
/// Returns a merged and validated `Config`.
pub fn load_config_with_overrides(overrides: Option<CliOverrides>) -> Result<Config> {
    let global_path = get_global_config_path()?;
    let project_path = get_project_config_path();

    // Use custom config file if specified, otherwise use standard paths
    let (config_paths, custom_path) = if let Some(ref overrides) = overrides {
        if let Some(ref custom_config) = overrides.config_file {
            // Load only from custom config file
            (vec![custom_config.clone()], Some(custom_config.clone()))
        } else {
            // Load from standard paths
            let mut paths = Vec::new();
            paths.push(global_path.clone());
            if let Some(ref project) = project_path {
                paths.push(project.clone());
            }
            (paths, None)
        }
    } else {
        // No overrides, use standard paths
        let mut paths = Vec::new();
        paths.push(global_path.clone());
        if let Some(ref project) = project_path {
            paths.push(project.clone());
        }
        (paths, None)
    };

    // Load configs from all paths
    let mut configs: Vec<Config> = Vec::new();

    for path in &config_paths {
        if path.exists() {
            debug!("Loading configuration from: {}", path.display());
            match load_config_file(path) {
                Ok(config) => configs.push(config),
                Err(e) => {
                    // If custom config was specified, fail hard
                    if custom_path.is_some() {
                        return Err(e.context(format!(
                            "Failed to load custom config file: {}",
                            path.display()
                        )));
                    }
                    // Otherwise, just log and continue
                    debug!("Skipping invalid config at {}: {}", path.display(), e);
                }
            }
        } else {
            // If custom config was specified and doesn't exist, fail hard
            if custom_path.is_some() {
                return Err(anyhow::anyhow!(
                    "Custom config file not found: {}",
                    path.display()
                ));
            }
            // Otherwise, just log and continue
            debug!("No config found at: {}", path.display());
        }
    }

    // Merge all configs (last one wins)
    let mut merged = Config::default();
    for config in configs {
        merged = merge_configs(Some(merged), Some(config));
    }

    // Apply CLI overrides if provided
    if let Some(overrides) = overrides {
        merged = apply_cli_overrides(merged, overrides);
    }

    // Validate the final configuration
    validate_config(&merged)?;

    Ok(merged)
}

/// Applies CLI overrides to a configuration
///
/// CLI arguments have the highest precedence and override all other sources.
fn apply_cli_overrides(mut config: Config, overrides: CliOverrides) -> Config {
    // Apply custom config file if specified
    // Note: This is handled by load_config_with_overrides before calling this

    // Override default agent
    if let Some(ref agent) = overrides.agent {
        config.default = Some(agent.clone());
    }

    // Override output format
    if let Some(ref format) = overrides.output_format {
        config.output.format = *format;
    }

    // Override log level
    if let Some(ref level) = overrides.log_level {
        config.logging.level = *level;
    }

    // Override log file
    if let Some(ref file) = overrides.log_file {
        config.logging.file = Some(file.clone());
    }

    // Override colored output
    if let Some(no_color) = overrides.no_color {
        config.output.colored = !no_color;
    }

    // Override progress bars (inverse of no_color)
    if let Some(no_color) = overrides.no_color {
        config.output.progress = !no_color;
    }

    // Apply execution mode-specific overrides
    if let Some(ref mode_str) = overrides.mode {
        apply_mode_overrides(&mut config, mode_str, &overrides);
    }

    config
}

/// Apply mode-specific overrides to configuration
fn apply_mode_overrides(config: &mut Config, mode_name: &str, overrides: &CliOverrides) {
    match mode_name {
        "fast" => {
            // Override fast mode settings
            if let Some(ref mut fast) = config.modes.fast {
                if overrides.max_retries.is_some() {
                    fast.max_retries = overrides.max_retries.unwrap();
                }
                if overrides.timeout.is_some() {
                    fast.timeout_exec = overrides.timeout.unwrap();
                }
                if overrides.run_tests.is_some() {
                    fast.run_tests = overrides.run_tests.unwrap();
                }
                if overrides.verify.is_some() {
                    fast.verify = overrides.verify.unwrap();
                }
            }
        }
        "standard" => {
            // Override standard mode settings
            if let Some(ref mut standard) = config.modes.standard {
                if overrides.max_retries.is_some() {
                    standard.max_retries = overrides.max_retries.unwrap();
                }
                if overrides.timeout.is_some() {
                    standard.timeout_exec = overrides.timeout.unwrap();
                }
                if overrides.run_tests.is_some() {
                    standard.run_tests = overrides.run_tests.unwrap();
                }
                if overrides.verify.is_some() {
                    standard.verify = overrides.verify.unwrap();
                }
            }
        }
        "expert" => {
            // Override expert mode settings
            if let Some(ref mut expert) = config.modes.expert {
                if overrides.max_retries.is_some() {
                    expert.max_retries = overrides.max_retries.unwrap();
                }
                if overrides.timeout.is_some() {
                    expert.timeout_exec = overrides.timeout.unwrap();
                }
                if overrides.run_tests.is_some() {
                    expert.run_tests = overrides.run_tests.unwrap();
                }
                if overrides.verify.is_some() {
                    expert.verify = overrides.verify.unwrap();
                }
            }
        }
        _ => {}
    }
}

/// Validates a configuration
///
/// Ensures that:
/// - Default agent exists in agents map
/// - Timeouts are positive and reasonable
/// - Retry limits are reasonable
/// - Paths are valid (if specified)
pub fn validate_config(config: &Config) -> Result<()> {
    // Validate default agent exists
    // Special case: if default is "claude" (the built-in default) and no agents are defined,
    // skip validation - this represents Config::default() with no config loaded
    if let Some(ref default_agent) = config.default {
        let is_builtin_default_with_no_agents =
            default_agent == "claude" && config.agents.is_empty();
        if !is_builtin_default_with_no_agents && !config.agents.contains_key(default_agent) {
            anyhow::bail!(
                "Default agent '{}' is not defined in configuration. Available agents: {}",
                default_agent,
                config.agents.keys().cloned().collect::<Vec<_>>().join(", ")
            );
        }
    }

    // Validate agent configurations
    for (name, agent_config) in &config.agents {
        // Validate timeout if specified
        if let Some(timeout) = agent_config.timeout {
            if timeout == 0 {
                anyhow::bail!("Agent '{}' has timeout of 0, must be positive", name);
            }
            if timeout > 86400 {
                // 24 hours
                anyhow::bail!(
                    "Agent '{}' has timeout of {}s (> 24 hours), likely an error",
                    name,
                    timeout
                );
            }
        }

        // Validate command exists if specified
        if let Some(ref command) = agent_config.command {
            if command.is_empty() {
                anyhow::bail!("Agent '{}' has empty command", name);
            }
        }
    }

    // Validate mode configurations
    validate_mode_config("fast", &config.modes.fast)?;
    validate_mode_config("standard", &config.modes.standard)?;
    validate_mode_config("expert", &config.modes.expert)?;

    // Validate log file path if specified
    if let Some(ref log_path) = config.logging.file {
        // Check if the parent directory exists or can be created
        if let Some(parent) = log_path.parent() {
            if !parent.exists() {
                debug!(
                    "Log file parent directory does not exist: {}, will be created if needed",
                    parent.display()
                );
            }
        }
    }

    debug!("Configuration validation passed");
    Ok(())
}

/// Validates a mode configuration
fn validate_mode_config(mode_name: &str, mode_config: &Option<ModeConfig>) -> Result<()> {
    if let Some(ref config) = mode_config {
        // Validate max_depth
        if config.max_depth > 5 {
            anyhow::bail!(
                "Mode '{}' has max_depth of {}, exceeding recommended maximum of 5",
                mode_name,
                config.max_depth
            );
        }

        // Validate max_retries
        if config.max_retries > 10 {
            anyhow::bail!(
                "Mode '{}' has max_retries of {}, exceeding recommended maximum of 10",
                mode_name,
                config.max_retries
            );
        }

        // Validate timeouts
        if config.timeout_plan == 0 {
            anyhow::bail!(
                "Mode '{}' has timeout_plan of 0, must be positive",
                mode_name
            );
        }
        if config.timeout_exec == 0 {
            anyhow::bail!(
                "Mode '{}' has timeout_exec of 0, must be positive",
                mode_name
            );
        }

        // Validate timeout_exec is reasonable (not too short)
        if config.timeout_exec < 60 && mode_name != "fast" {
            anyhow::bail!(
                "Mode '{}' has timeout_exec of {}s, less than recommended minimum of 60s",
                mode_name,
                config.timeout_exec
            );
        }
    }
    Ok(())
}

/// Converts an AgentConfig to an Agent model
///
/// # Arguments
///
/// * `name` - Agent name
/// * `config` - Agent configuration
///
/// # Returns
///
/// Returns an `Agent` instance or an error if required fields are missing.
pub fn agent_config_to_agent(name: &str, config: &AgentConfig) -> Result<Agent> {
    let command = config
        .command
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' missing 'command' field", name))?
        .clone();

    let model = config
        .model
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' missing 'model' field", name))?
        .clone();

    let timeout = config.timeout.unwrap_or(3600);

    Ok(Agent::new(name, command, model, timeout))
}

/// Gets the default agent from configuration
///
/// # Arguments
///
/// * `config` - Configuration to read from
///
/// # Returns
///
/// Returns the default `Agent` or an error if not found.
pub fn get_default_agent(config: &Config) -> Result<Agent> {
    let agent_name = config
        .default
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No default agent configured"))?;

    let agent_config = config
        .agents
        .get(agent_name)
        .ok_or_else(|| anyhow::anyhow!("Default agent '{}' not found in config", agent_name))?;

    agent_config_to_agent(agent_name, agent_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.default, Some("claude".to_string()));
        assert!(config.agents.is_empty());
    }

    #[test]
    fn test_parse_valid_toml() {
        let toml_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[agents.opencode]
command = "opencode"
model = "gpt-4"
timeout = 1800

[modes.fast]
model = "claude-haiku-4-5"
run_tests = false
verify = true
max_retries = 1

[output]
format = "json"
colored = false

[logging]
level = "debug"
file = "/tmp/ltmatrix.log"
"#;

        let config: Config = toml::from_str(toml_content).unwrap();

        assert_eq!(config.default, Some("claude".to_string()));
        assert_eq!(config.agents.len(), 2);

        let claude = &config.agents["claude"];
        assert_eq!(claude.command, Some("claude".to_string()));
        assert_eq!(claude.model, Some("claude-sonnet-4-6".to_string()));
        assert_eq!(claude.timeout, Some(3600));

        assert!(config.modes.fast.is_some());
        let fast = config.modes.fast.as_ref().unwrap();
        assert_eq!(fast.model, Some("claude-haiku-4-5".to_string()));
        assert_eq!(fast.run_tests, false);
        assert_eq!(fast.verify, true);

        assert_eq!(config.output.format, OutputFormat::Json);
        assert_eq!(config.output.colored, false);

        assert_eq!(config.logging.level, LogLevel::Debug);
        assert_eq!(
            config.logging.file,
            Some(PathBuf::from("/tmp/ltmatrix.log"))
        );
    }

    #[test]
    fn test_parse_invalid_toml() {
        let invalid_toml = r#"
[default
agent = "claude"
"#;

        let result: Result<Config> = toml::from_str(invalid_toml).map_err(|e| anyhow::anyhow!(e));
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_configs() {
        let global = Config {
            default: Some("global-agent".to_string()),
            agents: {
                let mut map = HashMap::new();
                map.insert(
                    "agent1".to_string(),
                    AgentConfig {
                        command: Some("cmd1".to_string()),
                        model: Some("model1".to_string()),
                        timeout: Some(100),
                    },
                );
                map
            },
            modes: ModeConfigs {
                fast: Some(ModeConfig {
                    model: Some("global-fast".to_string()),
                    run_tests: false,
                    verify: true,
                    max_retries: 1,
                    max_depth: 2,
                    timeout_plan: 60,
                    timeout_exec: 1800,
                }),
                standard: None,
                expert: None,
            },
            output: OutputConfig {
                format: OutputFormat::Text,
                colored: true,
                progress: true,
            },
            logging: LoggingConfig {
                level: LogLevel::Info,
                file: None,
            },
            features: FeatureConfig::default(),
        warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        };

        let project = Config {
            default: Some("project-agent".to_string()),
            agents: {
                let mut map = HashMap::new();
                map.insert(
                    "agent1".to_string(),
                    AgentConfig {
                        command: Some("cmd1-overridden".to_string()), // Override command
                        model: None,                                  // Keep global model
                        timeout: Some(200),                           // Override timeout
                    },
                );
                map.insert(
                    "agent2".to_string(),
                    AgentConfig {
                        command: Some("cmd2".to_string()),
                        model: Some("model2".to_string()),
                        timeout: Some(150),
                    },
                );
                map
            },
            modes: ModeConfigs {
                fast: None, // Keep global fast
                standard: Some(ModeConfig {
                    model: Some("project-standard".to_string()),
                    run_tests: true,
                    verify: true,
                    max_retries: 3,
                    max_depth: 3,
                    timeout_plan: 120,
                    timeout_exec: 3600,
                }),
                expert: None,
            },
            output: OutputConfig {
                format: OutputFormat::Json, // Override format
                colored: false,             // Override colored
                progress: true,             // Keep global progress
            },
            logging: LoggingConfig {
                level: LogLevel::Debug, // Override level
                file: None,
            },
            features: FeatureConfig::default(),
        warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        };

        let merged = merge_configs(Some(global), Some(project));

        // Project default overrides global
        assert_eq!(merged.default, Some("project-agent".to_string()));

        // agent1 should be merged
        let agent1 = &merged.agents["agent1"];
        assert_eq!(agent1.command, Some("cmd1-overridden".to_string()));
        assert_eq!(agent1.model, Some("model1".to_string())); // From global
        assert_eq!(agent1.timeout, Some(200)); // From project

        // agent2 should be from project only
        let agent2 = &merged.agents["agent2"];
        assert_eq!(agent2.command, Some("cmd2".to_string()));

        // Fast mode from global
        assert!(merged.modes.fast.is_some());
        assert_eq!(
            merged.modes.fast.unwrap().model,
            Some("global-fast".to_string())
        );

        // Standard mode from project
        assert!(merged.modes.standard.is_some());
        assert_eq!(
            merged.modes.standard.unwrap().model,
            Some("project-standard".to_string())
        );

        // Output settings from project (with some from global)
        assert_eq!(merged.output.format, OutputFormat::Json);
        assert_eq!(merged.output.colored, false);

        // Logging level from project
        assert_eq!(merged.logging.level, LogLevel::Debug);
    }

    #[test]
    fn test_merge_with_none() {
        let config = Config {
            default: Some("test".to_string()),
            agents: HashMap::new(),
            modes: ModeConfigs::default(),
            output: OutputConfig::default(),
            logging: LoggingConfig::default(),
            features: FeatureConfig::default(),
        warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        };

        let merged = merge_configs(Some(config.clone()), None);
        assert_eq!(merged.default, Some("test".to_string()));

        let merged = merge_configs(None, Some(config));
        assert_eq!(merged.default, Some("test".to_string()));
    }

    #[test]
    fn test_agent_config_to_agent() {
        let config = AgentConfig {
            command: Some("test-cmd".to_string()),
            model: Some("test-model".to_string()),
            timeout: Some(1234),
        };

        let agent = agent_config_to_agent("test-agent", &config).unwrap();
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.command, "test-cmd");
        assert_eq!(agent.model, "test-model");
        assert_eq!(agent.timeout, 1234);
    }

    #[test]
    fn test_agent_config_missing_command() {
        let config = AgentConfig {
            command: None,
            model: Some("test-model".to_string()),
            timeout: Some(1234),
        };

        let result = agent_config_to_agent("test-agent", &config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing 'command'"));
    }

    #[test]
    fn test_agent_config_missing_model() {
        let config = AgentConfig {
            command: Some("test-cmd".to_string()),
            model: None,
            timeout: Some(1234),
        };

        let result = agent_config_to_agent("test-agent", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing 'model'"));
    }

    #[test]
    fn test_agent_config_default_timeout() {
        let config = AgentConfig {
            command: Some("test-cmd".to_string()),
            model: Some("test-model".to_string()),
            timeout: None,
        };

        let agent = agent_config_to_agent("test-agent", &config).unwrap();
        assert_eq!(agent.timeout, 3600); // Default timeout
    }

    #[test]
    fn test_feature_config_default() {
        let config = Config::default();
        // Features should have default values
        assert!(!config.features.agent_backend.enable_claude_opus_backend);
        assert!(config.features.pipeline.enable_parallel_execution);
        assert!(config.features.pipeline.enable_smart_cache);
    }

    #[test]
    fn test_get_default_agent() {
        let mut config = Config::default();

        let agent_config = AgentConfig {
            command: Some("claude".to_string()),
            model: Some("claude-sonnet-4-6".to_string()),
            timeout: Some(3600),
        };

        config.agents.insert("claude".to_string(), agent_config);
        config.default = Some("claude".to_string());

        let agent = get_default_agent(&config).unwrap();
        assert_eq!(agent.name, "claude");
        assert_eq!(agent.command, "claude");
        assert_eq!(agent.model, "claude-sonnet-4-6");
    }

    #[test]
    fn test_get_default_agent_not_found() {
        let config = Config::default();
        let result = get_default_agent(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_default_agent_missing() {
        let mut config = Config::default();
        config.default = None;

        let result = get_default_agent(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_file_not_found() {
        let result = load_config_file(Path::new("/nonexistent/path/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_file_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        std::fs::write(&config_path, b"invalid [toml").unwrap();

        let result = load_config_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_output_format_in_config() {
        // Test OutputFormat as part of a struct
        let config = OutputConfig {
            format: OutputFormat::Json,
            colored: false,
            progress: true,
        };
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("format = \"json\""));
    }

    #[test]
    fn test_log_level_in_config() {
        // Test LogLevel as part of a struct
        let config = LoggingConfig {
            level: LogLevel::Debug,
            file: None,
        };
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("level = \"debug\""));
    }

    // ============================================================================
    // CLI Override Tests
    // ============================================================================

    #[test]
    fn test_apply_cli_overrides_agent() {
        let config = Config {
            default: Some("original".to_string()),
            agents: HashMap::new(),
            modes: ModeConfigs::default(),
            output: OutputConfig::default(),
            logging: LoggingConfig::default(),
            features: FeatureConfig::default(),
        warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        };

        let overrides = CliOverrides {
            agent: Some("cli-agent".to_string()),
            config_file: None,
            mode: None,
            output_format: None,
            log_level: None,
            log_file: None,
            max_retries: None,
            timeout: None,
            no_color: None,
            dry_run: false,
            resume: false,
            ask: false,
            regenerate_plan: false,
            on_blocked: None,
            mcp_config: None,
            progress: None,
            run_tests: None,
            verify: None,
        };

        let merged = apply_cli_overrides(config, overrides);
        assert_eq!(merged.default, Some("cli-agent".to_string()));
    }

    #[test]
    fn test_apply_cli_overrides_output_format() {
        let config = Config {
            default: None,
            agents: HashMap::new(),
            modes: ModeConfigs::default(),
            output: OutputConfig {
                format: OutputFormat::Text,
                colored: true,
                progress: true,
            },
            logging: LoggingConfig::default(),
            features: FeatureConfig::default(),
        warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        };

        let overrides = CliOverrides {
            agent: None,
            config_file: None,
            mode: None,
            output_format: Some(OutputFormat::Json),
            log_level: None,
            log_file: None,
            max_retries: None,
            timeout: None,
            no_color: None,
            dry_run: false,
            resume: false,
            ask: false,
            regenerate_plan: false,
            on_blocked: None,
            mcp_config: None,
            progress: None,
            run_tests: None,
            verify: None,
        };

        let merged = apply_cli_overrides(config, overrides);
        assert_eq!(merged.output.format, OutputFormat::Json);
    }

    #[test]
    fn test_apply_cli_overrides_log_level() {
        let config = Config {
            default: None,
            agents: HashMap::new(),
            modes: ModeConfigs::default(),
            output: OutputConfig::default(),
            logging: LoggingConfig {
                level: LogLevel::Info,
                file: None,
            },
            features: FeatureConfig::default(),
        warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        };

        let overrides = CliOverrides {
            agent: None,
            config_file: None,
            mode: None,
            output_format: None,
            log_level: Some(LogLevel::Debug),
            log_file: None,
            max_retries: None,
            timeout: None,
            no_color: None,
            dry_run: false,
            resume: false,
            ask: false,
            regenerate_plan: false,
            on_blocked: None,
            mcp_config: None,
            progress: None,
            run_tests: None,
            verify: None,
        };

        let merged = apply_cli_overrides(config, overrides);
        assert_eq!(merged.logging.level, LogLevel::Debug);
    }

    #[test]
    fn test_apply_cli_overrides_no_color() {
        let config = Config {
            default: None,
            agents: HashMap::new(),
            modes: ModeConfigs::default(),
            output: OutputConfig {
                format: OutputFormat::Text,
                colored: true,
                progress: true,
            },
            logging: LoggingConfig::default(),
            features: FeatureConfig::default(),
        warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        };

        let overrides = CliOverrides {
            agent: None,
            config_file: None,
            mode: None,
            output_format: None,
            log_level: None,
            log_file: None,
            max_retries: None,
            timeout: None,
            no_color: Some(true),
            dry_run: false,
            resume: false,
            ask: false,
            regenerate_plan: false,
            on_blocked: None,
            mcp_config: None,
            progress: None,
            run_tests: None,
            verify: None,
        };

        let merged = apply_cli_overrides(config, overrides);
        assert_eq!(merged.output.colored, false);
        assert_eq!(merged.output.progress, false); // Progress bars also disabled with no_color
    }

    #[test]
    fn test_apply_cli_overrides_mode_settings() {
        let config = Config {
            default: Some("claude".to_string()),
            agents: {
                let mut map = HashMap::new();
                map.insert(
                    "claude".to_string(),
                    AgentConfig {
                        command: Some("claude".to_string()),
                        model: Some("claude-sonnet-4-6".to_string()),
                        timeout: None,
                    },
                );
                map
            },
            modes: ModeConfigs {
                fast: Some(ModeConfig {
                    model: Some("claude-haiku-4-5".to_string()),
                    run_tests: false,
                    verify: true,
                    max_retries: 1,
                    max_depth: 2,
                    timeout_plan: 60,
                    timeout_exec: 1800,
                }),
                standard: None,
                expert: None,
            },
            output: OutputConfig::default(),
            logging: LoggingConfig::default(),
            features: FeatureConfig::default(),
        warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        };

        let overrides = CliOverrides {
            agent: None,
            config_file: None,
            mode: Some("fast".to_string()),
            output_format: None,
            log_level: None,
            log_file: None,
            max_retries: Some(5),
            timeout: Some(2400),
            no_color: None,
            dry_run: false,
            resume: false,
            ask: false,
            regenerate_plan: false,
            on_blocked: None,
            mcp_config: None,
            progress: None,
            run_tests: Some(true), // Override from false to true
            verify: Some(false),   // Override from true to false
        };

        let merged = apply_cli_overrides(config, overrides);
        assert_eq!(merged.modes.fast.as_ref().unwrap().max_retries, 5);
        assert_eq!(merged.modes.fast.as_ref().unwrap().timeout_exec, 2400);
        assert_eq!(merged.modes.fast.as_ref().unwrap().run_tests, true); // Overridden
        assert_eq!(merged.modes.fast.as_ref().unwrap().verify, false); // Overridden
    }

    // ============================================================================
    // Config Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_config_success() {
        let mut config = Config::default();
        let agent_config = AgentConfig {
            command: Some("claude".to_string()),
            model: Some("claude-sonnet-4-6".to_string()),
            timeout: Some(3600),
        };
        config.agents.insert("claude".to_string(), agent_config);

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_default_agent() {
        let config = Config {
            default: Some("nonexistent".to_string()),
            agents: HashMap::new(),
            modes: ModeConfigs::default(),
            output: OutputConfig::default(),
            logging: LoggingConfig::default(),
            features: FeatureConfig::default(),
        warmup: WarmupConfig::default(),
            pool: PoolConfig::default(),
        };

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not defined in configuration"));
    }

    #[test]
    fn test_validate_config_zero_timeout() {
        let mut config = Config::default();
        let agent_config = AgentConfig {
            command: Some("claude".to_string()),
            model: Some("claude-sonnet-4-6".to_string()),
            timeout: Some(0), // Invalid
        };
        config.agents.insert("claude".to_string(), agent_config);
        config.default = Some("claude".to_string());

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout of 0"));
    }

    #[test]
    fn test_validate_config_excessive_timeout() {
        let mut config = Config::default();
        let agent_config = AgentConfig {
            command: Some("claude".to_string()),
            model: Some("claude-sonnet-4-6".to_string()),
            timeout: Some(100000), // > 24 hours
        };
        config.agents.insert("claude".to_string(), agent_config);
        config.default = Some("claude".to_string());

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("likely an error"));
    }

    #[test]
    fn test_validate_config_empty_command() {
        let mut config = Config::default();
        let agent_config = AgentConfig {
            command: Some("".to_string()), // Invalid
            model: Some("claude-sonnet-4-6".to_string()),
            timeout: Some(3600),
        };
        config.agents.insert("claude".to_string(), agent_config);
        config.default = Some("claude".to_string());

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty command"));
    }

    #[test]
    fn test_validate_mode_config_excessive_depth() {
        let config = ModeConfig {
            model: Some("test".to_string()),
            run_tests: true,
            verify: true,
            max_retries: 3,
            max_depth: 10, // > 5
            timeout_plan: 120,
            timeout_exec: 3600,
        };

        let result = validate_mode_config("test", &Some(config));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeding recommended maximum"));
    }

    #[test]
    fn test_validate_mode_config_excessive_retries() {
        let config = ModeConfig {
            model: Some("test".to_string()),
            run_tests: true,
            verify: true,
            max_retries: 20, // > 10
            max_depth: 3,
            timeout_plan: 120,
            timeout_exec: 3600,
        };

        let result = validate_mode_config("test", &Some(config));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeding recommended maximum"));
    }

    #[test]
    fn test_validate_mode_config_zero_timeout_plan() {
        let config = ModeConfig {
            model: Some("test".to_string()),
            run_tests: true,
            verify: true,
            max_retries: 3,
            max_depth: 3,
            timeout_plan: 0, // Invalid
            timeout_exec: 3600,
        };

        let result = validate_mode_config("test", &Some(config));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be positive"));
    }

    #[test]
    fn test_validate_mode_config_zero_timeout_exec() {
        let config = ModeConfig {
            model: Some("test".to_string()),
            run_tests: true,
            verify: true,
            max_retries: 3,
            max_depth: 3,
            timeout_plan: 120,
            timeout_exec: 0, // Invalid
        };

        let result = validate_mode_config("test", &Some(config));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be positive"));
    }

    #[test]
    fn test_validate_mode_config_too_short_timeout() {
        let config = ModeConfig {
            model: Some("test".to_string()),
            run_tests: true,
            verify: true,
            max_retries: 3,
            max_depth: 3,
            timeout_plan: 120,
            timeout_exec: 30, // < 60s for non-fast mode
        };

        let result = validate_mode_config("standard", &Some(config));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("less than recommended minimum"));
    }

    #[test]
    fn test_full_merge_precedence() {
        // Test: CLI > Project > Global > Defaults
        let mut global = Config::default();
        global.default = Some("global-agent".to_string());
        global.output.format = OutputFormat::Text;

        let project = Config {
            default: Some("project-agent".to_string()),
            output: OutputConfig {
                format: OutputFormat::Json,
                ..Default::default()
            },
            ..Default::default()
        };

        // Merge global and project
        let merged = merge_configs(Some(global), Some(project));

        // Project overrides global
        assert_eq!(merged.default, Some("project-agent".to_string()));
        assert_eq!(merged.output.format, OutputFormat::Json);

        // Now apply CLI override
        let cli_overrides = CliOverrides {
            config_file: None,
            agent: Some("cli-agent".to_string()),
            mode: None,
            output_format: Some(OutputFormat::Json),
            log_level: None,
            log_file: None,
            max_retries: None,
            timeout: None,
            no_color: None,
            dry_run: false,
            resume: false,
            ask: false,
            regenerate_plan: false,
            on_blocked: None,
            mcp_config: None,
            progress: None,
            run_tests: None,
            verify: None,
        };

        let final_config = apply_cli_overrides(merged, cli_overrides);

        // CLI overrides everything
        assert_eq!(final_config.default, Some("cli-agent".to_string()));
        assert_eq!(final_config.output.format, OutputFormat::Json);
    }

    #[test]
    fn test_validate_mode_fast_mode_short_timeout_allowed() {
        // Fast mode should allow shorter timeouts
        let config = ModeConfig {
            model: Some("claude-haiku-4-5".to_string()),
            run_tests: false,
            verify: true,
            max_retries: 1,
            max_depth: 2,
            timeout_plan: 30,
            timeout_exec: 30, // Short timeout OK for fast mode
        };

        let result = validate_mode_config("fast", &Some(config));
        assert!(result.is_ok()); // Should pass for fast mode
    }

    // ============================================================================
    // CLI-Config Integration Tests
    // ============================================================================

    #[test]
    fn test_load_config_from_args_basic() {
        // Test with the default "claude" agent which will pass validation
        let args = crate::cli::Args::try_parse_from(["ltmatrix", "--agent", "claude", "goal"])
            .expect("Failed to parse args");

        let result = load_config_from_args(args);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.default, Some("claude".to_string()));
    }

    #[test]
    fn test_load_config_from_args_with_mode() {
        let args = crate::cli::Args::try_parse_from(["ltmatrix", "--fast", "goal"])
            .expect("Failed to parse args");

        let result = load_config_from_args(args);
        assert!(result.is_ok());

        let config = result.unwrap();
        // Mode should be preserved in CliOverrides even if not directly visible in Config
        assert_eq!(config.default, Some("claude".to_string())); // Default agent
    }

    #[test]
    fn test_load_config_from_args_with_output_format() {
        let args = crate::cli::Args::try_parse_from(["ltmatrix", "--output", "json", "goal"])
            .expect("Failed to parse args");

        let result = load_config_from_args(args);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.output.format, OutputFormat::Json);
    }

    #[test]
    fn test_load_config_from_args_with_log_settings() {
        let args = crate::cli::Args::try_parse_from([
            "ltmatrix",
            "--log-level",
            "debug",
            "--log-file",
            "/tmp/test.log",
            "goal",
        ])
        .expect("Failed to parse args");

        let result = load_config_from_args(args);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.logging.level, LogLevel::Debug);
        assert_eq!(config.logging.file, Some(PathBuf::from("/tmp/test.log")));
    }

    #[test]
    fn test_load_config_from_args_no_color() {
        let args = crate::cli::Args::try_parse_from(["ltmatrix", "--no-color", "goal"])
            .expect("Failed to parse args");

        let result = load_config_from_args(args);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.output.colored, false);
    }

    #[test]
    fn test_load_config_from_args_with_all_overrides() {
        let args = crate::cli::Args::try_parse_from([
            "ltmatrix",
            "--agent",
            "claude",
            "--mode",
            "expert",
            "--output",
            "json",
            "--log-level",
            "trace",
            "--log-file",
            "/tmp/custom.log",
            "--max-retries",
            "7",
            "--timeout",
            "2400",
            "--no-color",
            "goal",
        ])
        .expect("Failed to parse args");

        let result = load_config_from_args(args);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.default, Some("claude".to_string()));
        assert_eq!(config.output.format, OutputFormat::Json);
        assert_eq!(config.logging.level, LogLevel::Trace);
        assert_eq!(config.logging.file, Some(PathBuf::from("/tmp/custom.log")));
        assert_eq!(config.output.colored, false);
    }

    #[test]
    fn test_args_to_overrides_conversion_comprehensive() {
        // Test all fields are properly converted (no validation required)
        let args = crate::cli::Args::try_parse_from([
            "ltmatrix",
            "--agent",
            "test-agent",
            "--mode",
            "fast",
            "--output",
            "json",
            "--log-level",
            "debug",
            "--log-file",
            "/tmp/test.log",
            "--max-retries",
            "5",
            "--timeout",
            "1800",
            "--no-color",
            "goal",
        ])
        .expect("Failed to parse args");

        let overrides: CliOverrides = args.into();

        assert_eq!(overrides.agent, Some("test-agent".to_string()));
        assert_eq!(overrides.mode, Some("fast".to_string()));
        assert_eq!(overrides.output_format, Some(OutputFormat::Json));
        assert_eq!(overrides.log_level, Some(LogLevel::Debug));
        assert_eq!(overrides.log_file, Some(PathBuf::from("/tmp/test.log")));
        assert_eq!(overrides.max_retries, Some(5));
        assert_eq!(overrides.timeout, Some(1800));
        assert_eq!(overrides.no_color, Some(true));
    }

    #[test]
    fn test_args_to_overrides_with_expert_flag() {
        let args = crate::cli::Args::try_parse_from(["ltmatrix", "--expert", "goal"])
            .expect("Failed to parse args");

        let overrides: CliOverrides = args.into();
        assert_eq!(overrides.mode, Some("expert".to_string()));
    }

    #[test]
    fn test_args_to_overrides_without_mode_flags() {
        let args =
            crate::cli::Args::try_parse_from(["ltmatrix", "goal"]).expect("Failed to parse args");

        let overrides: CliOverrides = args.into();
        assert_eq!(overrides.mode, None); // No mode specified
    }

    #[test]
    fn test_args_to_overrides_partial_fields() {
        // Test with only some fields set (no validation required for conversion)
        let args =
            crate::cli::Args::try_parse_from(["ltmatrix", "--agent", "partial-agent", "goal"])
                .expect("Failed to parse args");

        let overrides: CliOverrides = args.into();

        assert_eq!(overrides.agent, Some("partial-agent".to_string()));
        assert_eq!(overrides.mode, None);
        assert_eq!(overrides.output_format, None);
        assert_eq!(overrides.log_level, None);
        assert_eq!(overrides.log_file, None);
        assert_eq!(overrides.max_retries, None);
        assert_eq!(overrides.timeout, None);
        assert_eq!(overrides.no_color, None); // false is None
    }
}
