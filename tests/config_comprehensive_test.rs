//! Comprehensive tests for config system covering all edge cases
//!
//! This test suite provides complete coverage including:
//! - Config structure validation
//! - File loading error scenarios
//! - Merge logic with conflicting values
//! - Precedence rule verification
//! - Boundary conditions and edge cases

use clap::Parser;
use ltmatrix::cli::Args;
use ltmatrix::config::settings::{
    load_config_file, merge_configs, validate_config, AgentConfig, CliOverrides, Config, LogLevel,
    LoggingConfig, ModeConfig, OutputConfig, OutputFormat,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Conflicting Values Tests
// ============================================================================

#[test]
fn test_conflicting_agent_names_between_sources() {
    // Global and project both define same agent with different settings
    let mut global = Config::default();
    global.agents.insert(
        "claude".to_string(),
        AgentConfig {
            command: Some("claude-global".to_string()),
            model: Some("global-model".to_string()),
            timeout: Some(1800),
        },
    );

    let mut project = Config::default();
    project.agents.insert(
        "claude".to_string(),
        AgentConfig {
            command: Some("claude-project".to_string()),
            model: Some("project-model".to_string()),
            timeout: Some(3600),
        },
    );

    let merged = merge_configs(Some(global), Some(project));

    // Project should override individual fields
    let claude = &merged.agents["claude"];
    assert_eq!(claude.command, Some("claude-project".to_string()));
    assert_eq!(claude.model, Some("project-model".to_string()));
    assert_eq!(claude.timeout, Some(3600));
}

#[test]
fn test_conflicting_mode_settings() {
    // Global and project both define same mode with different settings
    let mut global = Config::default();
    global.modes.fast = Some(ModeConfig {
        model: Some("global-fast-model".to_string()),
        run_tests: true,
        verify: false,
        max_retries: 5,
        max_depth: 3,
        timeout_plan: 120,
        timeout_exec: 1800,
    });

    let mut project = Config::default();
    project.modes.fast = Some(ModeConfig {
        model: Some("project-fast-model".to_string()),
        run_tests: false,
        verify: true,
        max_retries: 2,
        max_depth: 2,
        timeout_plan: 60,
        timeout_exec: 900,
    });

    let merged = merge_configs(Some(global), Some(project));

    // Project mode should completely replace global mode
    let fast = merged.modes.fast.unwrap();
    assert_eq!(fast.model, Some("project-fast-model".to_string()));
    assert_eq!(fast.run_tests, false);
    assert_eq!(fast.verify, true);
    assert_eq!(fast.max_retries, 2);
    assert_eq!(fast.max_depth, 2);
    assert_eq!(fast.timeout_plan, 60);
    assert_eq!(fast.timeout_exec, 900);
}

#[test]
fn test_conflicting_output_settings() {
    let global = Config {
        output: OutputConfig {
            format: OutputFormat::Json,
            colored: false,
            progress: false,
        },
        ..Default::default()
    };

    let project = Config {
        output: OutputConfig {
            format: OutputFormat::Text,
            colored: true,
            progress: true,
        },
        ..Default::default()
    };

    let merged = merge_configs(Some(global), Some(project));

    // Project output should completely replace global
    assert_eq!(merged.output.format, OutputFormat::Text);
    assert_eq!(merged.output.colored, true);
    assert_eq!(merged.output.progress, true);
}

#[test]
fn test_conflicting_logging_settings() {
    let global = Config {
        logging: LoggingConfig {
            level: LogLevel::Trace,
            file: Some(PathBuf::from("/tmp/global.log")),
        },
        ..Default::default()
    };

    let project = Config {
        logging: LoggingConfig {
            level: LogLevel::Error,
            file: Some(PathBuf::from("/tmp/project.log")),
        },
        ..Default::default()
    };

    let merged = merge_configs(Some(global), Some(project));

    // Project logging should completely replace global
    assert_eq!(merged.logging.level, LogLevel::Error);
    assert_eq!(merged.logging.file, Some(PathBuf::from("/tmp/project.log")));
}

// ============================================================================
// Additional File Loading Error Scenarios
// ============================================================================

#[test]
fn test_config_file_permission_denied() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Create a file with no read permissions (Unix-like systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(&config_path, "default = \"test\"").unwrap();
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&config_path, perms).unwrap();

        let result = load_config_file(&config_path);
        assert!(result.is_err());

        // Restore permissions for cleanup
        perms.set_mode(0o644);
        fs::set_permissions(&config_path, perms).unwrap();
    }

    #[cfg(windows)]
    {
        // Windows doesn't support the same permission model
        // Skip this test on Windows
        assert!(true);
    }
}

#[test]
fn test_config_file_with_invalid_utf8() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Write invalid UTF-8 bytes
    fs::write(&config_path, b"\xff\xfe default = \"test\"").unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());
}

#[test]
fn test_config_file_with_null_bytes() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // TOML with embedded null bytes
    let content = "default = \x00 \"test\"\n\n[agents.test]\ncommand = \"test\"\n";
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    // Should fail or handle gracefully
    assert!(result.is_err());
}

#[test]
fn test_config_file_with_only_comments() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // File with only comments (no actual content)
    let content = "# This is a comment\n# Another comment\n# default = \"test\"\n";
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    // Empty config has no default set (defaults are applied at merge time)
    assert_eq!(config.default, None);
}

#[test]
fn test_config_file_with_empty_sections() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // TOML with empty sections
    let content = r#"
default = "test"

[agents.test]

[output]

[logging]
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());
}

#[test]
fn test_config_file_very_long_line() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Very long model name (10,000 characters)
    let long_model = "x".repeat(10000);
    let content = format!(
        r#"
default = "test"

[agents.test]
command = "test"
model = "{}"
"#,
        long_model
    );
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    // Should handle long lines
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.agents["test"].model.as_ref().unwrap().len(), 10000);
}

#[test]
fn test_config_file_with_special_path_characters() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Path with special characters (spaces, unicode, etc.)
    let content = r#"
default = "test"

[agents.test]
command = "test"
model = "test-model"

[logging]
file = "C:\\Users\\Test User\\Logs\\app.log"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(
        config.logging.file,
        Some(PathBuf::from(r"C:\Users\Test User\Logs\app.log"))
    );
}

#[test]
fn test_config_file_with_duplicate_keys() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // TOML with duplicate keys (should fail - TOML spec rejects duplicates)
    let content = r#"
default = "first"
default = "second"

[agents.test]
command = "first-command"
command = "second-command"
model = "test-model"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    // TOML parser should reject duplicate keys
    assert!(result.is_err(), "TOML parser should reject duplicate keys");
}

// ============================================================================
// Merge Logic Edge Cases
// ============================================================================

#[test]
fn test_merge_all_none_sources() {
    let merged = merge_configs(None, None);
    assert_eq!(merged.default, Some("claude".to_string())); // Default
    assert!(merged.agents.is_empty());
    assert!(merged.modes.fast.is_none());
}

#[test]
fn test_merge_with_partial_agent_configs() {
    // Global has agent with only command, project has same agent with only model
    let mut global = Config::default();
    global.agents.insert(
        "test".to_string(),
        AgentConfig {
            command: Some("global-command".to_string()),
            model: None,
            timeout: None,
        },
    );

    let mut project = Config::default();
    project.agents.insert(
        "test".to_string(),
        AgentConfig {
            command: None,
            model: Some("project-model".to_string()),
            timeout: Some(1800),
        },
    );

    let merged = merge_configs(Some(global), Some(project));

    let test_agent = &merged.agents["test"];
    // Fields should be combined
    assert_eq!(test_agent.command, Some("global-command".to_string()));
    assert_eq!(test_agent.model, Some("project-model".to_string()));
    assert_eq!(test_agent.timeout, Some(1800));
}

#[test]
fn test_merge_preserves_unique_agents() {
    // Global has agent1, project has agent2
    let mut global = Config::default();
    global.agents.insert(
        "agent1".to_string(),
        AgentConfig {
            command: Some("cmd1".to_string()),
            model: Some("model1".to_string()),
            timeout: Some(1000),
        },
    );

    let mut project = Config::default();
    project.agents.insert(
        "agent2".to_string(),
        AgentConfig {
            command: Some("cmd2".to_string()),
            model: Some("model2".to_string()),
            timeout: Some(2000),
        },
    );

    let merged = merge_configs(Some(global), Some(project));

    // Both agents should be present
    assert!(merged.agents.contains_key("agent1"));
    assert!(merged.agents.contains_key("agent2"));
    assert_eq!(merged.agents.len(), 2);
}

// ============================================================================
// Validation Boundary Tests
// ============================================================================

#[test]
fn test_validation_timeout_boundary_values() {
    let mut config = Config::default();
    config.agents.insert(
        "test".to_string(),
        AgentConfig {
            command: Some("test".to_string()),
            model: Some("test-model".to_string()),
            timeout: Some(86400), // Exactly 24 hours
        },
    );
    config.default = Some("test".to_string());

    let result = validate_config(&config);
    assert!(result.is_ok(), "24 hour timeout should be valid");
}

#[test]
fn test_validation_timeout_just_over_limit() {
    let mut config = Config::default();
    config.agents.insert(
        "test".to_string(),
        AgentConfig {
            command: Some("test".to_string()),
            model: Some("test-model".to_string()),
            timeout: Some(86401), // Just over 24 hours
        },
    );
    config.default = Some("test".to_string());

    let result = validate_config(&config);
    assert!(result.is_err(), "Timeout over 24 hours should be rejected");
}

#[test]
fn test_validation_max_depth_boundary() {
    let mut config = Config::default();
    config.agents.insert(
        "test".to_string(),
        AgentConfig {
            command: Some("test".to_string()),
            model: Some("test-model".to_string()),
            timeout: Some(3600),
        },
    );
    config.default = Some("test".to_string());

    config.modes.fast = Some(ModeConfig {
        model: Some("test".to_string()),
        run_tests: false,
        verify: true,
        max_retries: 1,
        max_depth: 5, // Exactly at limit
        timeout_plan: 60,
        timeout_exec: 1800,
    });

    let result = validate_config(&config);
    assert!(result.is_ok(), "Max depth of 5 should be valid");
}

#[test]
fn test_validation_max_retries_boundary() {
    let mut config = Config::default();
    config.agents.insert(
        "test".to_string(),
        AgentConfig {
            command: Some("test".to_string()),
            model: Some("test-model".to_string()),
            timeout: Some(3600),
        },
    );
    config.default = Some("test".to_string());

    config.modes.standard = Some(ModeConfig {
        model: Some("test".to_string()),
        run_tests: true,
        verify: true,
        max_retries: 10, // Exactly at limit
        max_depth: 3,
        timeout_plan: 120,
        timeout_exec: 3600,
    });

    let result = validate_config(&config);
    assert!(result.is_ok(), "Max retries of 10 should be valid");
}

#[test]
fn test_validation_mode_specific_timeout_minimums() {
    // Each mode has different timeout minimums
    let test_cases = [
        ("fast", 30, true),      // Fast mode allows short timeouts
        ("fast", 1, true),       // Even 1 second is OK for fast
        ("standard", 60, true),  // Standard needs at least 60s
        ("standard", 59, false), // 59s is too short
        ("expert", 60, true),    // Expert needs at least 60s
        ("expert", 59, false),   // 59s is too short
    ];

    for (mode, timeout, should_pass) in test_cases {
        let mut config = Config::default();
        config.agents.insert(
            "test".to_string(),
            AgentConfig {
                command: Some("test".to_string()),
                model: Some("test-model".to_string()),
                timeout: Some(3600),
            },
        );
        config.default = Some("test".to_string());

        let mode_config = ModeConfig {
            model: Some("test".to_string()),
            run_tests: true,
            verify: true,
            max_retries: 3,
            max_depth: 3,
            timeout_plan: 120,
            timeout_exec: timeout,
        };

        if mode == "fast" {
            config.modes.fast = Some(mode_config.clone());
        } else if mode == "standard" {
            config.modes.standard = Some(mode_config.clone());
        } else if mode == "expert" {
            config.modes.expert = Some(mode_config.clone());
        }

        let result = validate_config(&config);
        assert_eq!(
            result.is_ok(),
            should_pass,
            "Mode {} with timeout {} should {}",
            mode,
            timeout,
            if should_pass { "pass" } else { "fail" }
        );
    }
}

// ============================================================================
// CLI Override Conflict Tests
// ============================================================================

#[test]
fn test_cli_override_resolves_all_conflicts() {
    // Create conflicting configs in all sources
    let mut global = Config::default();
    global.default = Some("global".to_string());
    global.output.format = OutputFormat::Json;
    global.logging.level = LogLevel::Debug;

    let mut project = Config::default();
    project.default = Some("project".to_string());
    project.output.format = OutputFormat::Text;
    project.logging.level = LogLevel::Warn;

    // Merge them
    let merged = merge_configs(Some(global), Some(project));

    // Apply CLI overrides that conflict with both
    let cli_overrides = CliOverrides {
        agent: Some("claude".to_string()), // Use valid agent
        output_format: Some(OutputFormat::Json),
        log_level: Some(LogLevel::Error),
        ..Default::default()
    };

    let final_config = apply_cli_overrides(merged, cli_overrides);

    // CLI should win all conflicts
    assert_eq!(final_config.default, Some("claude".to_string()));
    assert_eq!(final_config.output.format, OutputFormat::Json);
    assert_eq!(final_config.logging.level, LogLevel::Error);
}

// Helper function (should be in settings module but defined here for testing)
fn apply_cli_overrides(mut config: Config, overrides: CliOverrides) -> Config {
    if let Some(agent) = overrides.agent {
        config.default = Some(agent);
    }
    if let Some(format) = overrides.output_format {
        config.output.format = format;
    }
    if let Some(level) = overrides.log_level {
        config.logging.level = level;
    }
    if let Some(file) = overrides.log_file {
        config.logging.file = Some(file);
    }
    if let Some(no_color) = overrides.no_color {
        config.output.colored = !no_color;
    }
    config
}

#[test]
fn test_cli_args_with_all_override_flags() {
    let temp_dir = TempDir::new().unwrap();

    // Create a valid config file to ensure validation passes
    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600
"#,
    )
    .unwrap();

    // Change to temp directory so project config is found
    let current_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--agent",
        "claude", // Use valid agent
        "--mode",
        "expert",
        "--output",
        "json",
        "--log-level",
        "trace",
        "--log-file",
        "/tmp/override.log",
        "--max-retries",
        "9",
        "--timeout",
        "7200",
        "--no-color",
        "goal",
    ])
    .expect("Failed to parse args");

    let config = ltmatrix::config::settings::load_config();

    std::env::set_current_dir(current_dir).unwrap();

    assert!(
        config.is_ok(),
        "Config with all overrides should load successfully: {:?}",
        config.err()
    );
    let config = config.unwrap();

    // Verify all overrides were applied
    assert_eq!(config.default, Some("claude".to_string()));
    assert_eq!(config.output.format, OutputFormat::Json);
    assert_eq!(config.logging.level, LogLevel::Trace);
    assert_eq!(
        config.logging.file,
        Some(PathBuf::from("/tmp/override.log"))
    );
    assert_eq!(config.output.colored, false);
}

// ============================================================================
// Complex Real-World Scenarios
// ============================================================================

#[test]
fn test_real_world_config_hierarchy() {
    // Test realistic scenario: global defaults + project overrides
    let global_toml = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[modes.fast]
model = "claude-haiku-4-5"
max_retries = 1

[output]
colored = true

[logging]
level = "info"
"#;

    let project_toml = r#"
[modes.fast]
max_retries = 3

[output]
colored = false

[logging]
level = "debug"
file = "/tmp/project.log"
"#;

    let mut global: Config = toml::from_str(global_toml).unwrap();
    let project: Config = toml::from_str(project_toml).unwrap();

    // Merge: project overrides global
    let merged = merge_configs(Some(global), Some(project));

    // Verify precedence: project > global > defaults
    assert_eq!(merged.default, Some("claude".to_string())); // From global
    assert_eq!(merged.agents["claude"].timeout, Some(3600)); // From global
    assert_eq!(merged.modes.fast.as_ref().unwrap().max_retries, 3); // Project override
    assert_eq!(merged.output.colored, false); // Project override
    assert_eq!(merged.logging.level, LogLevel::Debug); // Project override
    assert_eq!(merged.logging.file, Some(PathBuf::from("/tmp/project.log"))); // Project
}

#[test]
fn test_config_development_vs_production() {
    // Test scenario: Dev uses fast mode locally, prod uses standard in CI
    let dev_config = r#"
default = "claude"

[modes.fast]
model = "claude-haiku-4-5"
run_tests = false
max_retries = 1

[output]
colored = true

[logging]
level = "debug"
"#;

    let prod_config = r#"
default = "claude"

[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
max_retries = 3

[output]
colored = false

[logging]
level = "warn"
file = "/var/log/ltmatrix.log"
"#;

    // Parse both
    let dev: Config = toml::from_str(dev_config).unwrap();
    let prod: Config = toml::from_str(prod_config).unwrap();

    // Verify differences
    assert_eq!(dev.modes.fast.as_ref().unwrap().run_tests, false);
    assert_eq!(dev.output.colored, true);
    assert_eq!(dev.logging.level, LogLevel::Debug);

    assert_eq!(prod.modes.standard.as_ref().unwrap().run_tests, true);
    assert_eq!(prod.output.colored, false);
    assert_eq!(prod.logging.level, LogLevel::Warn);
    assert!(prod.logging.file.is_some());
}

#[test]
fn test_config_team_collaboration_scenario() {
    // Scenario: Team has base config, each member customizes locally
    let base_config = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[modes.standard]
model = "claude-sonnet-4-6"
max_retries = 3
max_depth = 3
run_tests = true
verify = true
timeout_plan = 120
timeout_exec = 3600
"#;

    let member_customization = r#"
[modes.standard]
model = "claude-sonnet-4-6"
max_retries = 5
max_depth = 3
run_tests = true
verify = true
timeout_plan = 120
timeout_exec = 3600

[logging]
level = "debug"
"#;

    let base: Config = toml::from_str(base_config).unwrap();
    let member: Config = toml::from_str(member_customization).unwrap();

    // Merge: member customizes base
    let merged = merge_configs(Some(base), Some(member));

    assert_eq!(merged.modes.standard.as_ref().unwrap().max_retries, 5); // Overridden
    assert_eq!(merged.modes.standard.as_ref().unwrap().max_depth, 3); // From base
    assert_eq!(merged.modes.standard.as_ref().unwrap().run_tests, true); // From base
    assert_eq!(merged.logging.level, LogLevel::Debug); // Overridden
}
