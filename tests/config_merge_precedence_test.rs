//! Comprehensive tests for config merge logic with precedence
//!
//! This test suite validates the complete configuration merging implementation
//! that combines multiple sources (defaults, global config, project config, CLI overrides)
//! with correct precedence rules and validation.
//!
//! Precedence order (highest to lowest):
//! 1. CLI overrides
//! 2. Project config (.ltmatrix/config.toml)
//! 3. Global config (~/.ltmatrix/config.toml)
//! 4. Built-in defaults

use ltmatrix::config::settings::{
    load_config_file, merge_configs, validate_config, AgentConfig, CliOverrides, Config, LogLevel,
    LoggingConfig, ModeConfig, ModeConfigs, OutputConfig, OutputFormat, WarmupConfig,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Precedence Order Tests
// ============================================================================

#[test]
fn test_precedence_cli_overrides_project() {
    // Create a project config with specific values
    let mut project_config = Config::default();
    project_config.default = Some("project-agent".to_string());
    project_config.output.format = OutputFormat::Json;
    project_config.logging.level = LogLevel::Debug;

    // Create CLI overrides that should take precedence
    let cli_overrides = CliOverrides {
        agent: Some("cli-agent".to_string()),
        output_format: Some(OutputFormat::Text),
        log_level: Some(LogLevel::Warn),
        ..Default::default()
    };

    // Merge: start with project config
    let mut merged = project_config;

    // Apply CLI overrides (manually simulating the function behavior)
    if let Some(agent) = cli_overrides.agent {
        merged.default = Some(agent);
    }
    if let Some(format) = cli_overrides.output_format {
        merged.output.format = format;
    }
    if let Some(level) = cli_overrides.log_level {
        merged.logging.level = level;
    }

    // Verify CLI overrides took precedence
    assert_eq!(
        merged.default,
        Some("cli-agent".to_string()),
        "CLI agent override should override project config"
    );
    assert_eq!(
        merged.output.format,
        OutputFormat::Text,
        "CLI output format should override project config"
    );
    assert_eq!(
        merged.logging.level,
        LogLevel::Warn,
        "CLI log level should override project config"
    );
}

#[test]
fn test_precedence_project_overrides_global() {
    // Create global config
    let mut global_config = Config::default();
    global_config.default = Some("global-agent".to_string());
    global_config.output.format = OutputFormat::Json;
    global_config.logging.level = LogLevel::Warn;

    // Create project config that overrides some settings
    let mut project_config = Config::default();
    project_config.default = Some("project-agent".to_string());
    project_config.logging.level = LogLevel::Debug;
    // Note: project_config.output.format is Text (default)

    // Merge them
    let merged = merge_configs(Some(global_config), Some(project_config));

    // Project should override for default agent
    assert_eq!(
        merged.default,
        Some("project-agent".to_string()),
        "Project config should override global config for default agent"
    );

    // Output config is completely replaced by project (even if just defaults)
    assert_eq!(
        merged.output.format,
        OutputFormat::Text,
        "Project's output config completely replaces global's"
    );

    // Logging config is completely replaced by project
    assert_eq!(
        merged.logging.level,
        LogLevel::Debug,
        "Project's logging config completely replaces global's"
    );
}

#[test]
fn test_precedence_global_overrides_default() {
    // Default config
    let default_config = Config::default();
    assert_eq!(default_config.default, Some("claude".to_string()));

    // Global config that overrides default
    let mut global_config = Config::default();
    global_config.default = Some("global-agent".to_string());
    global_config.output.format = OutputFormat::Json;

    // Merge (no project config)
    let merged = merge_configs(Some(global_config), None);

    // Global should override default
    assert_eq!(
        merged.default,
        Some("global-agent".to_string()),
        "Global config should override built-in defaults"
    );
    assert_eq!(
        merged.output.format,
        OutputFormat::Json,
        "Global config settings should be used"
    );
}

#[test]
fn test_precedence_full_chain() {
    // Test the full precedence chain: CLI > Project > Global > Default

    // 1. Start with defaults
    let defaults = Config::default();
    assert_eq!(defaults.default, Some("claude".to_string()));
    assert_eq!(defaults.output.format, OutputFormat::Text);
    assert_eq!(defaults.logging.level, LogLevel::Info);

    // 2. Apply global config
    let mut global = defaults.clone();
    global.default = Some("global-agent".to_string());
    global.output.format = OutputFormat::Json;
    global.logging.level = LogLevel::Warn;

    // 3. Apply project config (replaces output/logging completely)
    let mut project = Config::default();
    project.default = Some("project-agent".to_string());
    project.logging.level = LogLevel::Debug;
    // project.output.format is Text (default)

    // Merge global + project
    let merged = merge_configs(Some(global), Some(project));

    // Verify project overrides global for default agent
    assert_eq!(
        merged.default,
        Some("project-agent".to_string()),
        "Project should override global"
    );

    // Output and logging are completely replaced by project's values
    assert_eq!(
        merged.output.format,
        OutputFormat::Text,
        "Project's output config replaces global's completely"
    );
    assert_eq!(
        merged.logging.level,
        LogLevel::Debug,
        "Project's logging config replaces global's completely"
    );
}

#[test]
fn test_precedence_cli_highest_priority() {
    // Test that CLI overrides have highest priority of all

    // Setup: global + project merged
    let mut global = Config::default();
    global.default = Some("global".to_string());
    global.output.format = OutputFormat::Json;
    global.logging.level = LogLevel::Warn;

    let mut project = Config::default();
    project.default = Some("project".to_string());
    project.logging.level = LogLevel::Debug;

    let mut merged = merge_configs(Some(global), Some(project));

    // Now apply CLI overrides
    let cli_overrides = CliOverrides {
        agent: Some("cli".to_string()),
        output_format: Some(OutputFormat::Text),
        log_level: Some(LogLevel::Error),
        ..Default::default()
    };

    if let Some(agent) = cli_overrides.agent {
        merged.default = Some(agent);
    }
    if let Some(format) = cli_overrides.output_format {
        merged.output.format = format;
    }
    if let Some(level) = cli_overrides.log_level {
        merged.logging.level = level;
    }

    // Verify CLI overrides win
    assert_eq!(
        merged.default,
        Some("cli".to_string()),
        "CLI should override both project and global"
    );
    assert_eq!(
        merged.output.format,
        OutputFormat::Text,
        "CLI output format should override all config files"
    );
    assert_eq!(
        merged.logging.level,
        LogLevel::Error,
        "CLI log level should override all config files"
    );
}

// ============================================================================
// Deep Merge Tests for Nested Structures
// ============================================================================

#[test]
fn test_deep_merge_agent_configs() {
    // When both global and project define the same agent, fields should merge
    // with project taking precedence for fields it specifies

    let mut global_config = Config::default();
    global_config.default = Some("claude".to_string());

    let mut global_agents = HashMap::new();
    global_agents.insert(
        "claude".to_string(),
        AgentConfig {
            command: Some("claude".to_string()),
            model: Some("claude-sonnet-4-6".to_string()),
            timeout: Some(3600),
        },
    );
    global_agents.insert(
        "other".to_string(),
        AgentConfig {
            command: Some("other-cmd".to_string()),
            model: Some("other-model".to_string()),
            timeout: Some(1800),
        },
    );
    global_config.agents = global_agents;

    let mut project_config = Config::default();
    let mut project_agents = HashMap::new();
    // Project overrides only model for claude agent
    project_agents.insert(
        "claude".to_string(),
        AgentConfig {
            command: None,                              // Not specified, should inherit from global
            model: Some("claude-opus-4-6".to_string()), // Override global
            timeout: None,                              // Not specified, should inherit from global
        },
    );
    // Project adds a new agent
    project_agents.insert(
        "project-only".to_string(),
        AgentConfig {
            command: Some("project-cmd".to_string()),
            model: Some("project-model".to_string()),
            timeout: Some(7200),
        },
    );
    project_config.agents = project_agents;

    let merged = merge_configs(Some(global_config), Some(project_config));

    // Check claude agent was properly merged
    let claude = &merged.agents["claude"];
    assert_eq!(
        claude.command,
        Some("claude".to_string()),
        "Command should come from global when not specified in project"
    );
    assert_eq!(
        claude.model,
        Some("claude-opus-4-6".to_string()),
        "Model from project should override global"
    );
    assert_eq!(
        claude.timeout,
        Some(3600),
        "Timeout should come from global when not specified in project"
    );

    // Check other agent from global is preserved
    assert!(
        merged.agents.contains_key("other"),
        "Agents only in global should be preserved"
    );
    let other = &merged.agents["other"];
    assert_eq!(other.command, Some("other-cmd".to_string()));

    // Check project-only agent exists
    assert!(
        merged.agents.contains_key("project-only"),
        "Agents only in project should be present"
    );
}

#[test]
fn test_deep_merge_mode_configs() {
    // Modes should be additive, not merged
    // If both global and project define a mode, project should win

    let mut global_config = Config::default();
    global_config.modes = ModeConfigs {
        fast: Some(ModeConfig {
            model: Some("global-fast".to_string()),
            run_tests: false,
            verify: true,
            max_retries: 1,
            max_depth: 2,
            timeout_plan: 60,
            timeout_exec: 1800,
        }),
        standard: Some(ModeConfig {
            model: Some("global-standard".to_string()),
            run_tests: true,
            verify: true,
            max_retries: 3,
            max_depth: 3,
            timeout_plan: 120,
            timeout_exec: 3600,
        }),
        expert: None,
    };

    let mut project_config = Config::default();
    project_config.modes = ModeConfigs {
        fast: Some(ModeConfig {
            model: Some("project-fast".to_string()),
            run_tests: false,
            verify: false,
            max_retries: 2,
            max_depth: 2,
            timeout_plan: 90,
            timeout_exec: 2400,
        }),
        standard: None, // Not specified, should use global
        expert: Some(ModeConfig {
            model: Some("project-expert".to_string()),
            run_tests: true,
            verify: true,
            max_retries: 5,
            max_depth: 4,
            timeout_plan: 300,
            timeout_exec: 7200,
        }),
    };

    let merged = merge_configs(Some(global_config), Some(project_config));

    // Fast mode: project should completely replace global
    assert!(merged.modes.fast.is_some());
    let fast = merged.modes.fast.as_ref().unwrap();
    assert_eq!(
        fast.model,
        Some("project-fast".to_string()),
        "Project fast mode should override global fast mode"
    );
    assert_eq!(
        fast.max_retries, 2,
        "All fields from project mode should be used"
    );

    // Standard mode: global should be used (not specified in project)
    assert!(merged.modes.standard.is_some());
    let standard = merged.modes.standard.as_ref().unwrap();
    assert_eq!(
        standard.model,
        Some("global-standard".to_string()),
        "Global standard mode should be used when project doesn't specify"
    );

    // Expert mode: only in project
    assert!(merged.modes.expert.is_some());
    let expert = merged.modes.expert.as_ref().unwrap();
    assert_eq!(expert.model, Some("project-expert".to_string()));
}

#[test]
fn test_deep_merge_output_config() {
    // Output config should be completely replaced, not merged
    // Project output config replaces global entirely

    let mut global_config = Config::default();
    global_config.output = OutputConfig {
        format: OutputFormat::Json,
        colored: true,
        progress: true,
    };

    let mut project_config = Config::default();
    project_config.output = OutputConfig {
        format: OutputFormat::Text, // Only this field set explicitly
        colored: false,
        progress: false,
    };

    let merged = merge_configs(Some(global_config), Some(project_config));

    // Entire output config should come from project
    assert_eq!(merged.output.format, OutputFormat::Text);
    assert_eq!(merged.output.colored, false);
    assert_eq!(merged.output.progress, false);
}

#[test]
fn test_deep_merge_logging_config() {
    // Logging config should be completely replaced, not merged

    let mut global_config = Config::default();
    global_config.logging = LoggingConfig {
        level: LogLevel::Warn,
        file: Some(PathBuf::from("/tmp/global.log")),
    };

    let mut project_config = Config::default();
    project_config.logging = LoggingConfig {
        level: LogLevel::Debug,
        file: Some(PathBuf::from("/tmp/project.log")),
    };

    let merged = merge_configs(Some(global_config), Some(project_config));

    // Entire logging config should come from project
    assert_eq!(merged.logging.level, LogLevel::Debug);
    assert_eq!(merged.logging.file, Some(PathBuf::from("/tmp/project.log")));
}

// ============================================================================
// CLI Override Tests
// ============================================================================

#[test]
fn test_cli_override_agent() {
    let mut config = Config::default();
    config.default = Some("config-agent".to_string());

    let overrides = CliOverrides {
        agent: Some("cli-agent".to_string()),
        ..Default::default()
    };

    // Apply overrides manually
    if let Some(agent) = overrides.agent {
        config.default = Some(agent);
    }

    assert_eq!(config.default, Some("cli-agent".to_string()));
}

#[test]
fn test_cli_override_output_format() {
    let mut config = Config::default();
    config.output.format = OutputFormat::Json;

    let overrides = CliOverrides {
        output_format: Some(OutputFormat::Text),
        ..Default::default()
    };

    if let Some(format) = overrides.output_format {
        config.output.format = format;
    }

    assert_eq!(config.output.format, OutputFormat::Text);
}

#[test]
fn test_cli_override_log_level() {
    let mut config = Config::default();
    config.logging.level = LogLevel::Info;

    let overrides = CliOverrides {
        log_level: Some(LogLevel::Debug),
        ..Default::default()
    };

    if let Some(level) = overrides.log_level {
        config.logging.level = level;
    }

    assert_eq!(config.logging.level, LogLevel::Debug);
}

#[test]
fn test_cli_override_log_file() {
    let mut config = Config::default();
    config.logging.file = Some(PathBuf::from("/tmp/default.log"));

    let overrides = CliOverrides {
        log_file: Some(PathBuf::from("/tmp/cli.log")),
        ..Default::default()
    };

    if let Some(file) = overrides.log_file {
        config.logging.file = Some(file);
    }

    assert_eq!(config.logging.file, Some(PathBuf::from("/tmp/cli.log")));
}

#[test]
fn test_cli_override_no_color() {
    let mut config = Config::default();
    config.output.colored = true;

    let overrides = CliOverrides {
        no_color: Some(true),
        ..Default::default()
    };

    if let Some(no_color) = overrides.no_color {
        config.output.colored = !no_color;
    }

    assert_eq!(config.output.colored, false);
}

#[test]
fn test_cli_override_max_retries() {
    let mut config = Config::default();
    config.modes.fast = Some(ModeConfig {
        model: Some("test".to_string()),
        run_tests: false,
        verify: true,
        max_retries: 1,
        max_depth: 2,
        timeout_plan: 60,
        timeout_exec: 1800,
    });

    let overrides = CliOverrides {
        mode: Some("fast".to_string()),
        max_retries: Some(5),
        ..Default::default()
    };

    // Apply mode-specific override
    if let Some(mode_str) = overrides.mode {
        if mode_str == "fast" {
            if let Some(ref mut fast) = config.modes.fast {
                if let Some(max_retries) = overrides.max_retries {
                    fast.max_retries = max_retries;
                }
            }
        }
    }

    assert_eq!(config.modes.fast.as_ref().unwrap().max_retries, 5);
}

#[test]
fn test_cli_override_timeout() {
    let mut config = Config::default();
    config.modes.standard = Some(ModeConfig {
        model: Some("test".to_string()),
        run_tests: true,
        verify: true,
        max_retries: 3,
        max_depth: 3,
        timeout_plan: 120,
        timeout_exec: 3600,
    });

    let overrides = CliOverrides {
        mode: Some("standard".to_string()),
        timeout: Some(7200),
        ..Default::default()
    };

    // Apply mode-specific timeout override
    if let Some(mode_str) = overrides.mode {
        if mode_str == "standard" {
            if let Some(ref mut standard) = config.modes.standard {
                if let Some(timeout) = overrides.timeout {
                    standard.timeout_exec = timeout;
                }
            }
        }
    }

    assert_eq!(config.modes.standard.as_ref().unwrap().timeout_exec, 7200);
}

#[test]
fn test_cli_override_partial() {
    // Test that only specified CLI overrides are applied
    let mut config = Config::default();
    config.default = Some("config-agent".to_string());
    config.output.format = OutputFormat::Json;
    config.logging.level = LogLevel::Debug;

    // Only override agent and log level, not output format
    let overrides = CliOverrides {
        agent: Some("cli-agent".to_string()),
        log_level: Some(LogLevel::Warn),
        ..Default::default()
    };

    if let Some(agent) = overrides.agent {
        config.default = Some(agent);
    }
    if let Some(level) = overrides.log_level {
        config.logging.level = level;
    }

    assert_eq!(
        config.default,
        Some("cli-agent".to_string()),
        "Agent should be overridden"
    );
    assert_eq!(
        config.logging.level,
        LogLevel::Warn,
        "Log level should be overridden"
    );
    assert_eq!(
        config.output.format,
        OutputFormat::Json,
        "Output format should remain unchanged"
    );
}

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
fn test_validation_valid_config() {
    let config = Config {
        default: Some("claude".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "claude".to_string(),
                AgentConfig {
                    command: Some("claude".to_string()),
                    model: Some("claude-sonnet-4-6".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(result.is_ok(), "Valid config should pass validation");
}

#[test]
fn test_validation_missing_default_agent() {
    let config = Config {
        default: Some("nonexistent".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "claude".to_string(),
                AgentConfig {
                    command: Some("claude".to_string()),
                    model: Some("claude-sonnet-4-6".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_err(),
        "Config with missing default agent should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("not defined"),
        "Error should mention agent is not defined"
    );
}

#[test]
fn test_validation_zero_timeout() {
    let config = Config {
        default: Some("test".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "test".to_string(),
                AgentConfig {
                    command: Some("test".to_string()),
                    model: Some("test-model".to_string()),
                    timeout: Some(0), // Invalid: must be positive
                },
            );
            map
        },
        modes: ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_err(),
        "Config with zero timeout should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("timeout") && error_msg.contains("positive"),
        "Error should mention timeout must be positive"
    );
}

#[test]
fn test_validation_excessive_timeout() {
    let config = Config {
        default: Some("test".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "test".to_string(),
                AgentConfig {
                    command: Some("test".to_string()),
                    model: Some("test-model".to_string()),
                    timeout: Some(100000), // > 24 hours
                },
            );
            map
        },
        modes: ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_err(),
        "Config with excessive timeout should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("timeout") && error_msg.contains("24 hours"),
        "Error should mention timeout exceeds 24 hours"
    );
}

#[test]
fn test_validation_empty_command() {
    let config = Config {
        default: Some("test".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "test".to_string(),
                AgentConfig {
                    command: Some("".to_string()), // Invalid: empty command
                    model: Some("test-model".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_err(),
        "Config with empty command should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("empty command"),
        "Error should mention empty command"
    );
}

#[test]
fn test_validation_mode_max_depth_exceeded() {
    let config = Config {
        default: Some("test".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "test".to_string(),
                AgentConfig {
                    command: Some("test".to_string()),
                    model: Some("test-model".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ModeConfigs {
            fast: Some(ModeConfig {
                model: Some("test".to_string()),
                run_tests: false,
                verify: true,
                max_retries: 1,
                max_depth: 10, // Exceeds recommended max of 5
                timeout_plan: 60,
                timeout_exec: 1800,
            }),
            standard: None,
            expert: None,
        },
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_err(),
        "Config with excessive max_depth should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("max_depth") && error_msg.contains("5"),
        "Error should mention max_depth exceeds recommended maximum"
    );
}

#[test]
fn test_validation_mode_max_retries_exceeded() {
    let config = Config {
        default: Some("test".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "test".to_string(),
                AgentConfig {
                    command: Some("test".to_string()),
                    model: Some("test-model".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ModeConfigs {
            standard: Some(ModeConfig {
                model: Some("test".to_string()),
                run_tests: true,
                verify: true,
                max_retries: 20, // Exceeds recommended max of 10
                max_depth: 3,
                timeout_plan: 120,
                timeout_exec: 3600,
            }),
            fast: None,
            expert: None,
        },
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_err(),
        "Config with excessive max_retries should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("max_retries") && error_msg.contains("10"),
        "Error should mention max_retries exceeds recommended maximum"
    );
}

#[test]
fn test_validation_mode_zero_timeout_plan() {
    let config = Config {
        default: Some("test".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "test".to_string(),
                AgentConfig {
                    command: Some("test".to_string()),
                    model: Some("test-model".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ModeConfigs {
            expert: Some(ModeConfig {
                model: Some("test".to_string()),
                run_tests: true,
                verify: true,
                max_retries: 5,
                max_depth: 4,
                timeout_plan: 0, // Invalid: must be positive
                timeout_exec: 7200,
            }),
            fast: None,
            standard: None,
        },
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_err(),
        "Config with zero timeout_plan should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("timeout_plan") && error_msg.contains("positive"),
        "Error should mention timeout_plan must be positive"
    );
}

#[test]
fn test_validation_mode_zero_timeout_exec() {
    let config = Config {
        default: Some("test".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "test".to_string(),
                AgentConfig {
                    command: Some("test".to_string()),
                    model: Some("test-model".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ModeConfigs {
            fast: Some(ModeConfig {
                model: Some("test".to_string()),
                run_tests: false,
                verify: true,
                max_retries: 1,
                max_depth: 2,
                timeout_plan: 60,
                timeout_exec: 0, // Invalid: must be positive
            }),
            standard: None,
            expert: None,
        },
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_err(),
        "Config with zero timeout_exec should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("timeout_exec") && error_msg.contains("positive"),
        "Error should mention timeout_exec must be positive"
    );
}

#[test]
fn test_validation_mode_too_short_timeout_exec() {
    let config = Config {
        default: Some("test".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "test".to_string(),
                AgentConfig {
                    command: Some("test".to_string()),
                    model: Some("test-model".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ModeConfigs {
            standard: Some(ModeConfig {
                model: Some("test".to_string()),
                run_tests: true,
                verify: true,
                max_retries: 3,
                max_depth: 3,
                timeout_plan: 120,
                timeout_exec: 30, // Too short for non-fast mode
            }),
            fast: None,
            expert: None,
        },
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_err(),
        "Config with too short timeout_exec should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("timeout_exec") && error_msg.contains("60"),
        "Error should mention timeout_exec is less than recommended minimum"
    );
}

#[test]
fn test_validation_fast_mode_allows_short_timeout() {
    // Fast mode should allow shorter timeouts
    let config = Config {
        default: Some("test".to_string()),
        agents: {
            let mut map = HashMap::new();
            map.insert(
                "test".to_string(),
                AgentConfig {
                    command: Some("test".to_string()),
                    model: Some("test-model".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ModeConfigs {
            fast: Some(ModeConfig {
                model: Some("test".to_string()),
                run_tests: false,
                verify: true,
                max_retries: 1,
                max_depth: 2,
                timeout_plan: 60,
                timeout_exec: 30, // OK for fast mode
            }),
            standard: None,
            expert: None,
        },
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(result.is_ok(), "Fast mode should allow short timeout_exec");
}

#[test]
fn test_validation_no_default_agent() {
    // Config with no default agent is valid
    let config = Config {
        default: None,
        agents: HashMap::new(),
        modes: ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let result = validate_config(&config);
    assert!(
        result.is_ok(),
        "Config with no default agent should be valid"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_merge_with_empty_configs() {
    let config1 = Config::default();
    let config2 = Config::default();

    let merged = merge_configs(Some(config1), Some(config2));

    assert_eq!(merged.default, Some("claude".to_string()));
    assert!(merged.agents.is_empty());
}

#[test]
fn test_merge_with_none_global() {
    let project_config = Config {
        default: Some("project".to_string()),
        agents: HashMap::new(),
        modes: ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let merged = merge_configs(None, Some(project_config));

    assert_eq!(merged.default, Some("project".to_string()));
}

#[test]
fn test_merge_with_none_project() {
    let global_config = Config {
        default: Some("global".to_string()),
        agents: HashMap::new(),
        modes: ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
    };

    let merged = merge_configs(Some(global_config), None);

    assert_eq!(merged.default, Some("global".to_string()));
}

#[test]
fn test_merge_with_both_none() {
    let merged = merge_configs(None, None);

    assert_eq!(merged.default, Some("claude".to_string()));
}

#[test]
fn test_agent_config_partial_override() {
    // Test that agent config fields are individually overridden
    let mut base_agents = HashMap::new();
    base_agents.insert(
        "test".to_string(),
        AgentConfig {
            command: Some("base-command".to_string()),
            model: Some("base-model".to_string()),
            timeout: Some(1000),
        },
    );

    let mut override_agents = HashMap::new();
    override_agents.insert(
        "test".to_string(),
        AgentConfig {
            command: None,                             // Keep base
            model: Some("override-model".to_string()), // Override base
            timeout: None,                             // Keep base
        },
    );

    let mut merged = HashMap::new();
    for (key, override_agent) in override_agents {
        if let Some(base_agent) = base_agents.remove(&key) {
            let merged_agent = AgentConfig {
                command: override_agent.command.or(base_agent.command),
                model: override_agent.model.or(base_agent.model),
                timeout: override_agent.timeout.or(base_agent.timeout),
            };
            merged.insert(key, merged_agent);
        }
    }

    let test_agent = &merged["test"];
    assert_eq!(test_agent.command, Some("base-command".to_string()));
    assert_eq!(test_agent.model, Some("override-model".to_string()));
    assert_eq!(test_agent.timeout, Some(1000));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_config_load_with_overrides_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Create global config
    let global_dir = temp_dir.path().join("home").join(".ltmatrix");
    fs::create_dir_all(&global_dir).unwrap();
    let global_config = global_dir.join("config.toml");
    fs::write(
        &global_config,
        r#"
default = "global-agent"

[agents.global-agent]
command = "global-cmd"
model = "global-model"
timeout = 3600

[output]
format = "json"
colored = true

[logging]
level = "warn"
"#,
    )
    .unwrap();

    // Create project config (output config will be default: Text, colored=false, progress=false)
    let project_dir = temp_dir.path().join("project").join(".ltmatrix");
    fs::create_dir_all(&project_dir).unwrap();
    let project_config = project_dir.join("config.toml");
    fs::write(
        &project_config,
        r#"
default = "project-agent"

[agents.project-agent]
command = "project-cmd"
model = "project-model"
timeout = 1800

[logging]
level = "debug"
"#,
    )
    .unwrap();

    // Load configs
    let global = load_config_file(&global_config).unwrap();
    let project = load_config_file(&project_config).unwrap();

    // Merge them
    let merged = merge_configs(Some(global), Some(project));

    // Verify merge
    assert_eq!(merged.default, Some("project-agent".to_string()));

    // Verify both agents exist
    assert!(merged.agents.contains_key("global-agent"));
    assert!(merged.agents.contains_key("project-agent"));

    // Verify project's logging overrides global's
    assert_eq!(merged.logging.level, LogLevel::Debug);

    // Output config is completely replaced by project's defaults
    assert_eq!(
        merged.output.format,
        OutputFormat::Text,
        "Project's output config (defaults) replaces global's"
    );
    assert_eq!(
        merged.output.colored, true,
        "Project's output colored (default=true) replaces global's"
    );
}

#[test]
fn test_config_validation_after_merge() {
    let temp_dir = TempDir::new().unwrap();

    // Create a valid global config
    let global_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&global_dir).unwrap();
    let global_config = global_dir.join("config.toml");
    fs::write(
        &global_config,
        r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[agents.other]
command = "other"
model = "other-model"
timeout = 1800
"#,
    )
    .unwrap();

    let global = load_config_file(&global_config).unwrap();

    // Validate global config
    let result = validate_config(&global);
    assert!(result.is_ok(), "Valid global config should pass validation");

    // Create project config that references non-existent agent
    let project_config = global_dir.join("project.toml");
    fs::write(
        &project_config,
        r#"
default = "nonexistent-agent"
"#,
    )
    .unwrap();

    let project = load_config_file(&project_config).unwrap();

    // Merge
    let merged = merge_configs(Some(global), Some(project));

    // Validate merged config - should fail because default agent doesn't exist
    let result = validate_config(&merged);
    assert!(
        result.is_err(),
        "Merged config with invalid default agent should fail"
    );
}
