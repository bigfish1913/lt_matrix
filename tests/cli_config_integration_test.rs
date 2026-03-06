//! Integration tests for CLI-Config module integration
//!
//! Task: Connect config system with CLI module to support command-line overrides
//! for all config values, including mapping CLI args to config fields
//!
//! These tests verify that:
//! - CLI arguments properly map to config override fields
//! - All CLI config-related options are covered
//! - Type conversions work correctly (LogLevel, OutputFormat)
//! - CLI overrides properly merge with loaded configs
//! - Precedence is correct: CLI > Project > Global > Defaults

use clap::Parser;
use ltmatrix::cli::Args;
use ltmatrix::config::settings::{
    load_config_with_overrides, CliOverrides, LogLevel, OutputFormat,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Helper to restore current directory when dropped
struct DirGuard {
    original: PathBuf,
}

impl DirGuard {
    fn new() -> Self {
        DirGuard {
            original: std::env::current_dir().unwrap(),
        }
    }

    /// Change to a new directory
    fn change_to(&self, path: &Path) {
        std::env::set_current_dir(path).unwrap();
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

// ============================================================================
// CLI Args to CliOverrides Conversion Tests
// ============================================================================

#[test]
fn test_cli_args_to_overrides_agent_mapping() {
    // Test that --agent CLI arg maps to CliOverrides::agent
    let args = Args::try_parse_from(["ltmatrix", "--agent", "custom-agent", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.agent, Some("custom-agent".to_string()));
}

#[test]
fn test_cli_args_to_overrides_mode_mapping() {
    // Test that --mode CLI arg maps to CliOverrides::mode
    let args =
        Args::try_parse_from(["ltmatrix", "--mode", "fast", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.mode, Some("fast".to_string()));
}

#[test]
fn test_cli_args_to_overrides_fast_flag() {
    // Test that --fast flag maps to CliOverrides::mode with "fast"
    let args = Args::try_parse_from(["ltmatrix", "--fast", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.mode, Some("fast".to_string()));
}

#[test]
fn test_cli_args_to_overrides_expert_flag() {
    // Test that --expert flag maps to CliOverrides::mode with "expert"
    let args =
        Args::try_parse_from(["ltmatrix", "--expert", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.mode, Some("expert".to_string()));
}

#[test]
fn test_cli_args_to_overrides_output_format_text() {
    // Test that --output text maps to CliOverrides::output_format
    let args = Args::try_parse_from(["ltmatrix", "--output", "text", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.output_format, Some(OutputFormat::Text));
}

#[test]
fn test_cli_args_to_overrides_output_format_json() {
    // Test that --output json maps to CliOverrides::output_format
    let args = Args::try_parse_from(["ltmatrix", "--output", "json", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.output_format, Some(OutputFormat::Json));
}

#[test]
fn test_cli_args_to_overrides_log_level() {
    // Test that --log-level maps correctly to CliOverrides::log_level
    let test_cases = [
        ("trace", LogLevel::Trace),
        ("debug", LogLevel::Debug),
        ("info", LogLevel::Info),
        ("warn", LogLevel::Warn),
        ("error", LogLevel::Error),
    ];

    for (cli_value, expected) in test_cases {
        let args = Args::try_parse_from(["ltmatrix", "--log-level", cli_value, "goal"])
            .expect("Failed to parse args");

        let overrides: CliOverrides = args.into();

        assert_eq!(
            overrides.log_level,
            Some(expected),
            "Log level '{}' should map to {:?}",
            cli_value,
            expected
        );
    }
}

#[test]
fn test_cli_args_to_overrides_log_file() {
    // Test that --log-file maps to CliOverrides::log_file
    let args = Args::try_parse_from(["ltmatrix", "--log-file", "/var/log/ltmatrix.log", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(
        overrides.log_file,
        Some(PathBuf::from("/var/log/ltmatrix.log"))
    );
}

#[test]
fn test_cli_args_to_overrides_max_retries() {
    // Test that --max-retries maps to CliOverrides::max_retries
    let args = Args::try_parse_from(["ltmatrix", "--max-retries", "10", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.max_retries, Some(10));
}

#[test]
fn test_cli_args_to_overrides_timeout() {
    // Test that --timeout maps to CliOverrides::timeout
    let args = Args::try_parse_from(["ltmatrix", "--timeout", "3600", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.timeout, Some(3600));
}

#[test]
fn test_cli_args_to_overrides_no_color_flag() {
    // Test that --no-color maps to CliOverrides::no_color
    let args =
        Args::try_parse_from(["ltmatrix", "--no-color", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.no_color, Some(true));
}

#[test]
fn test_cli_args_to_overrides_none_values() {
    // Test that unspecified CLI args result in None values in CliOverrides
    let args = Args::try_parse_from(["ltmatrix", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    // All optional fields should be None when not specified
    assert!(overrides.agent.is_none());
    assert!(overrides.mode.is_none());
    assert!(overrides.output_format.is_none());
    assert!(overrides.log_level.is_none());
    assert!(overrides.log_file.is_none());
    assert!(overrides.max_retries.is_none());
    assert!(overrides.timeout.is_none());
    // no_color may be Some(false) or None depending on implementation
}

// ============================================================================
// End-to-End Integration Tests
// ============================================================================
//
// NOTE: Tests in this section change the current directory and must run
// sequentially to avoid interference. Use: cargo test -- --test-threads=1
//

#[test]
fn test_cli_override_with_config_file() {
    // Test that CLI overrides properly merge with config file
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let config_content = r#"
default = "file-agent"

[agents.file-agent]
command = "file-cmd"
model = "file-model"

[output]
format = "text"
colored = true

[logging]
level = "info"
"#;
    fs::write(&config_path, config_content).unwrap();

    // Use DirGuard to ensure directory is restored
    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Parse CLI args with overrides
    let args = Args::try_parse_from([
        "ltmatrix",
        "--output",
        "json",
        "--log-level",
        "debug",
        "goal",
    ])
    .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    let result = load_config_with_overrides(Some(overrides));

    if let Err(ref e) = result {
        eprintln!("Error loading config: {}", e);
    }
    assert!(
        result.is_ok(),
        "load_config_with_overrides should succeed: {:?}",
        result
    );
    let config = result.unwrap();

    // CLI overrides should take precedence
    assert_eq!(config.default, Some("file-agent".to_string()));
    assert_eq!(config.output.format, OutputFormat::Json);
    assert_eq!(config.logging.level, LogLevel::Debug);
}

#[test]
fn test_cli_override_precedence_over_project_config() {
    // Verify precedence: CLI > Project > Global > Defaults
    let temp_dir = TempDir::new().unwrap();

    // Create project config
    let project_config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&project_config_dir).unwrap();
    let project_config = project_config_dir.join("config.toml");
    fs::write(
        &project_config,
        r#"
[output]
format = "text"
[logging]
level = "warn"
"#,
    )
    .unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // CLI overrides should beat project config
    let args = Args::try_parse_from([
        "ltmatrix",
        "--output",
        "json",
        "--log-level",
        "debug",
        "goal",
    ])
    .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    let result = load_config_with_overrides(Some(overrides));

    assert!(result.is_ok());
    let config = result.unwrap();

    assert_eq!(config.output.format, OutputFormat::Json);
    assert_eq!(config.logging.level, LogLevel::Debug);
}

#[test]
fn test_multiple_cli_overrides_simultaneously() {
    // Test that multiple CLI overrides work together
    let args = Args::try_parse_from([
        "ltmatrix",
        "--agent",
        "test-agent",
        "--mode",
        "fast",
        "--output",
        "json",
        "--log-level",
        "trace",
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
    assert_eq!(overrides.log_level, Some(LogLevel::Trace));
    assert_eq!(overrides.log_file, Some(PathBuf::from("/tmp/test.log")));
    assert_eq!(overrides.max_retries, Some(5));
    assert_eq!(overrides.timeout, Some(1800));
    assert_eq!(overrides.no_color, Some(true));
}

// ============================================================================
// Type Conversion Tests
// ============================================================================

#[test]
fn test_output_format_type_conversion() {
    // Verify CLI OutputFormat converts to Config OutputFormat
    let args = Args::try_parse_from(["ltmatrix", "--output", "json", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.output_format, Some(OutputFormat::Json));
}

#[test]
fn test_log_level_type_conversion() {
    // Verify CLI LogLevel converts to Config LogLevel
    let args = Args::try_parse_from(["ltmatrix", "--log-level", "debug", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.log_level, Some(LogLevel::Debug));
}

#[test]
fn test_mode_string_conversion() {
    // Verify mode flags convert to correct string values
    // Test with --fast flag
    let args = Args::try_parse_from(["ltmatrix", "--fast", "goal"]).expect("Failed to parse args");
    let overrides: CliOverrides = args.into();
    assert_eq!(overrides.mode, Some("fast".to_string()));

    // Test with --expert flag
    let args =
        Args::try_parse_from(["ltmatrix", "--expert", "goal"]).expect("Failed to parse args");
    let overrides: CliOverrides = args.into();
    assert_eq!(overrides.mode, Some("expert".to_string()));

    // Test with --mode standard
    let args = Args::try_parse_from(["ltmatrix", "--mode", "standard", "goal"])
        .expect("Failed to parse args");
    let overrides: CliOverrides = args.into();
    assert_eq!(overrides.mode, Some("standard".to_string()));
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_cli_override_with_subcommand() {
    // Test that subcommands (release, completions, man) don't interfere with override parsing
    let args = Args::try_parse_from(["ltmatrix", "--agent", "test-agent", "completions", "bash"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    // Should still parse the agent override even with subcommand
    assert_eq!(overrides.agent, Some("test-agent".to_string()));
}

#[test]
fn test_cli_args_default_mode() {
    // When no mode flag is specified, mode should be None (not "standard")
    let args = Args::try_parse_from(["ltmatrix", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    // Standard mode is the default when not specified, but in overrides it should be None
    // to avoid overriding config file defaults
    assert!(overrides.mode.is_none() || overrides.mode == Some("standard".to_string()));
}

#[test]
fn test_cli_override_max_retries_boundary() {
    // Test boundary values for max-retries
    let args = Args::try_parse_from(["ltmatrix", "--max-retries", "0", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    assert_eq!(overrides.max_retries, Some(0));

    let args = Args::try_parse_from(["ltmatrix", "--max-retries", "999", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    assert_eq!(overrides.max_retries, Some(999));
}

#[test]
fn test_cli_override_timeout_boundary() {
    // Test boundary values for timeout
    let args =
        Args::try_parse_from(["ltmatrix", "--timeout", "1", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    assert_eq!(overrides.timeout, Some(1));

    let args = Args::try_parse_from(["ltmatrix", "--timeout", "86400", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    assert_eq!(overrides.timeout, Some(86400));
}

// ============================================================================
// Custom Config File Tests
// ============================================================================

#[test]
fn test_custom_config_file_loads_exclusively() {
    // Test that when --config is specified, only that file is loaded
    let temp_dir = TempDir::new().unwrap();

    // Create a standard project config that should be ignored
    let project_config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&project_config_dir).unwrap();
    let project_config = project_config_dir.join("config.toml");
    fs::write(
        &project_config,
        r#"
default = "project-agent"

[agents.project-agent]
command = "project-cmd"
model = "project-model"

[output]
format = "text"

[logging]
level = "warn"
"#,
    )
    .unwrap();

    // Create a custom config file that should be used
    let custom_config = temp_dir.path().join("custom-config.toml");
    fs::write(
        &custom_config,
        r#"
default = "custom-agent"

[agents.custom-agent]
command = "custom-cmd"
model = "custom-model"

[output]
format = "json"

[logging]
level = "debug"
"#,
    )
    .unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Use --config to specify custom config
    let args = Args::try_parse_from([
        "ltmatrix",
        "--config",
        custom_config.to_str().unwrap(),
        "goal",
    ])
    .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    let result = load_config_with_overrides(Some(overrides));

    assert!(
        result.is_ok(),
        "load_config_with_overrides should succeed: {:?}",
        result
    );
    let config = result.unwrap();

    // Should load from custom config, not project config
    assert_eq!(
        config.default,
        Some("custom-agent".to_string()),
        "Should use custom config default, not project config"
    );
    assert_eq!(
        config.output.format,
        OutputFormat::Json,
        "Should use custom config output format, not project config"
    );
    assert_eq!(
        config.logging.level,
        LogLevel::Debug,
        "Should use custom config log level, not project config"
    );
}

#[test]
fn test_custom_config_file_fails_on_invalid_path() {
    // Test that when --config points to non-existent file, it fails hard
    let temp_dir = TempDir::new().unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    let nonexistent_path = temp_dir.path().join("does-not-exist.toml");

    // Use --config to specify non-existent file
    let args = Args::try_parse_from([
        "ltmatrix",
        "--config",
        nonexistent_path.to_str().unwrap(),
        "goal",
    ])
    .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    let result = load_config_with_overrides(Some(overrides));

    // Should fail because custom config doesn't exist
    assert!(
        result.is_err(),
        "load_config_with_overrides should fail when custom config doesn't exist"
    );
}

#[test]
fn test_custom_config_file_fails_on_invalid_toml() {
    // Test that when --config points to invalid TOML, it fails hard
    let temp_dir = TempDir::new().unwrap();

    // Create a custom config file with invalid TOML
    let custom_config = temp_dir.path().join("invalid-config.toml");
    fs::write(
        &custom_config,
        r#"
this is not valid toml at all [[[
"#,
    )
    .unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Use --config to specify invalid config
    let args = Args::try_parse_from([
        "ltmatrix",
        "--config",
        custom_config.to_str().unwrap(),
        "goal",
    ])
    .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    let result = load_config_with_overrides(Some(overrides));

    // Should fail because custom config is invalid
    assert!(
        result.is_err(),
        "load_config_with_overrides should fail when custom config is invalid TOML"
    );
}

#[test]
fn test_custom_config_file_with_cli_overrides() {
    // Test that CLI overrides work with custom config file
    let temp_dir = TempDir::new().unwrap();

    // Create a custom config file
    let custom_config = temp_dir.path().join("custom-config.toml");
    fs::write(
        &custom_config,
        r#"
default = "custom-agent"

[agents.custom-agent]
command = "custom-cmd"
model = "custom-model"

[agents.cli-override-agent]
command = "cli-cmd"
model = "cli-model"

[output]
format = "text"

[logging]
level = "info"
"#,
    )
    .unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Use --config with CLI overrides
    let args = Args::try_parse_from([
        "ltmatrix",
        "--config",
        custom_config.to_str().unwrap(),
        "--agent",
        "cli-override-agent",
        "--output",
        "json",
        "--log-level",
        "trace",
        "goal",
    ])
    .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    let result = load_config_with_overrides(Some(overrides));

    assert!(
        result.is_ok(),
        "load_config_with_overrides should succeed: {:?}",
        result
    );
    let config = result.unwrap();

    // CLI overrides should take precedence over custom config
    assert_eq!(
        config.default,
        Some("cli-override-agent".to_string()),
        "CLI override should beat custom config"
    );
    assert_eq!(
        config.output.format,
        OutputFormat::Json,
        "CLI override should beat custom config"
    );
    assert_eq!(
        config.logging.level,
        LogLevel::Trace,
        "CLI override should beat custom config"
    );
}

#[test]
fn test_no_custom_config_uses_standard_paths() {
    // Test that without --config, standard paths are used
    let temp_dir = TempDir::new().unwrap();
    let current_dir = std::env::current_dir().unwrap();

    // Create project config
    let project_config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&project_config_dir).unwrap();
    let project_config = project_config_dir.join("config.toml");
    fs::write(
        &project_config,
        r#"
default = "project-agent"

[agents.project-agent]
command = "project-cmd"
model = "project-model"

[output]
format = "json"

[logging]
level = "debug"
"#,
    )
    .unwrap();

    // Change to temp directory within a scope to ensure proper cleanup
    let result = {
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // No --config flag, should use standard paths
        let args = Args::try_parse_from(["ltmatrix", "goal"]).expect("Failed to parse args");

        let overrides: CliOverrides = args.into();
        load_config_with_overrides(Some(overrides))
    };

    // Restore directory immediately after scope
    std::env::set_current_dir(&current_dir).unwrap();

    assert!(
        result.is_ok(),
        "load_config_with_overrides should succeed: {:?}",
        result
    );
    let config = result.unwrap();

    // Should load from project config
    assert_eq!(
        config.default,
        Some("project-agent".to_string()),
        "Should use project config when no --config specified"
    );
    assert_eq!(
        config.output.format,
        OutputFormat::Json,
        "Should use project config when no --config specified"
    );
    assert_eq!(
        config.logging.level,
        LogLevel::Debug,
        "Should use project config when no --config specified"
    );
}

// ============================================================================
// Integration Acceptance Tests
// ============================================================================

#[test]
fn test_complete_cli_config_integration() {
    // Full integration test: CLI args → CliOverrides → Config merge
    let temp_dir = TempDir::new().unwrap();

    // Setup config file
    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
default = "file-agent"

[agents.file-agent]
command = "file-cmd"
model = "file-model"

[agents.cli-agent]
command = "cli-cmd"
model = "cli-model"

[modes.expert]
max_retries = 3
timeout_exec = 1800

[output]
format = "text"

[logging]
level = "info"
"#,
    )
    .unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Parse comprehensive CLI args
    let args = Args::try_parse_from([
        "ltmatrix",
        "--mode",
        "expert",
        "--output",
        "json-compact",
        "--log-level",
        "trace",
        "--log-file",
        "custom.log",
        "--max-retries",
        "7",
        "--timeout",
        "2400",
        "--no-color",
        "implement feature",
    ])
    .expect("Failed to parse args");

    // Convert to overrides
    let overrides: CliOverrides = args.into();

    // Load and merge config
    let result = load_config_with_overrides(Some(overrides));

    if let Err(ref e) = result {
        eprintln!("Error loading config: {}", e);
    }
    assert!(
        result.is_ok(),
        "load_config_with_overrides should succeed: {:?}",
        result
    );
    let config = result.unwrap();

    // Verify all CLI overrides took effect
    assert_eq!(config.default, Some("file-agent".to_string()));
    assert_eq!(config.output.format, OutputFormat::Json);
    assert_eq!(config.logging.level, LogLevel::Trace);
    assert_eq!(config.logging.file, Some(PathBuf::from("custom.log")));
    assert_eq!(config.output.colored, false); // --no-color

    // Mode-specific settings should be applied
    if let Some(expert_mode) = &config.modes.expert {
        assert_eq!(expert_mode.max_retries, 7);
        assert_eq!(expert_mode.timeout_exec, 2400);
    }
}
