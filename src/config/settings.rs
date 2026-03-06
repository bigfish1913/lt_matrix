//! Configuration settings for ltmatrix
//!
//! This module handles loading configuration from TOML files and merging
//! configuration from multiple sources (global config, project config, CLI args).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::debug;

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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default: Some("claude".to_string()),
            agents: HashMap::new(),
            modes: ModeConfigs::default(),
            output: OutputConfig::default(),
            logging: LoggingConfig::default(),
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
    #[serde(default)]
    pub max_retries: u32,

    /// Maximum depth for task decomposition
    #[serde(default)]
    pub max_depth: u32,

    /// Timeout for planning stage (seconds)
    #[serde(default)]
    pub timeout_plan: u64,

    /// Timeout for execution stage (seconds)
    #[serde(default)]
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

fn default_true() -> bool {
    true
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
    let home = dirs::home_dir()
        .context("Failed to determine home directory")?;

    Ok(home.join(".ltmatrix").join("config.toml"))
}

/// Gets the project configuration path
///
/// Returns `<current_dir>/.ltmatrix/config.toml`
pub fn get_project_config_path() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    Some(current.join(".ltmatrix").join("config.toml"))
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
    let global_path = get_global_config_path()?;
    let project_path = get_project_config_path();

    let global_config = if global_path.exists() {
        Some(load_config_file(&global_path)?)
    } else {
        debug!("No global config found at: {}", global_path.display());
        None
    };

    let project_config = if let Some(ref path) = project_path {
        if path.exists() {
            Some(load_config_file(path)?)
        } else {
            debug!("No project config found at: {}", path.display());
            None
        }
    } else {
        None
    };

    Ok(merge_configs(global_config, project_config))
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
    let command = config.command
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' missing 'command' field", name))?
        .clone();

    let model = config.model
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
    let agent_name = config.default
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No default agent configured"))?;

    let agent_config = config.agents
        .get(agent_name)
        .ok_or_else(|| anyhow::anyhow!("Default agent '{}' not found in config", agent_name))?;

    agent_config_to_agent(agent_name, agent_config)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(config.logging.file, Some(PathBuf::from("/tmp/ltmatrix.log")));
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
                map.insert("agent1".to_string(), AgentConfig {
                    command: Some("cmd1".to_string()),
                    model: Some("model1".to_string()),
                    timeout: Some(100),
                });
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
        };

        let project = Config {
            default: Some("project-agent".to_string()),
            agents: {
                let mut map = HashMap::new();
                map.insert("agent1".to_string(), AgentConfig {
                    command: Some("cmd1-overridden".to_string()),  // Override command
                    model: None,  // Keep global model
                    timeout: Some(200),  // Override timeout
                });
                map.insert("agent2".to_string(), AgentConfig {
                    command: Some("cmd2".to_string()),
                    model: Some("model2".to_string()),
                    timeout: Some(150),
                });
                map
            },
            modes: ModeConfigs {
                fast: None,  // Keep global fast
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
                format: OutputFormat::Json,  // Override format
                colored: false,  // Override colored
                progress: true,  // Keep global progress
            },
            logging: LoggingConfig {
                level: LogLevel::Debug,  // Override level
                file: None,
            },
        };

        let merged = merge_configs(Some(global), Some(project));

        // Project default overrides global
        assert_eq!(merged.default, Some("project-agent".to_string()));

        // agent1 should be merged
        let agent1 = &merged.agents["agent1"];
        assert_eq!(agent1.command, Some("cmd1-overridden".to_string()));
        assert_eq!(agent1.model, Some("model1".to_string()));  // From global
        assert_eq!(agent1.timeout, Some(200));  // From project

        // agent2 should be from project only
        let agent2 = &merged.agents["agent2"];
        assert_eq!(agent2.command, Some("cmd2".to_string()));

        // Fast mode from global
        assert!(merged.modes.fast.is_some());
        assert_eq!(merged.modes.fast.unwrap().model, Some("global-fast".to_string()));

        // Standard mode from project
        assert!(merged.modes.standard.is_some());
        assert_eq!(merged.modes.standard.unwrap().model, Some("project-standard".to_string()));

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
        assert!(result.unwrap_err().to_string().contains("missing 'command'"));
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
        assert_eq!(agent.timeout, 3600);  // Default timeout
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
}
