//! Comprehensive validation scenarios and error handling tests
//!
//! This test suite focuses specifically on:
//! - Detailed validation error messages
//! - Boundary condition testing
//! - Error recovery scenarios
//! - Input sanitization
//! - Type conversion validation
//!
//! These tests ensure robust error handling and provide clear feedback to users.

use ltmatrix::config::settings::{
    load_config_file, merge_configs, AgentConfig, Config, LogLevel, LoggingConfig, ModeConfig,
    OutputConfig, OutputFormat, WarmupConfig,
};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// TOML Syntax Error Scenarios
// ============================================================================

#[test]
fn test_invalid_toml_unclosed_bracket() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test
command = "test"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Failed to parse") || error_msg.contains("TOML"),
        "Error should indicate parsing failure"
    );
}

#[test]
fn test_invalid_toml_unclosed_string() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
command = "unclosed string
model = "test"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse") || error_msg.contains("TOML"));
}

#[test]
fn test_invalid_toml_invalid_escape_sequence() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
command = "test\xzz"
model = "test"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse") || error_msg.contains("TOML"));
}

#[test]
fn test_invalid_toml_invalid_number_format() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
command = "test"
model = "test"
timeout = 123.456.789
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Failed to parse")
            || error_msg.contains("TOML")
            || error_msg.contains("number")
    );
}

#[test]
fn test_invalid_toml_invalid_boolean() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[output]
colored = maybe
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Failed to parse")
            || error_msg.contains("TOML")
            || error_msg.contains("boolean")
    );
}

#[test]
fn test_invalid_toml_array_in_table_position() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Arrays cannot be used as table keys
    let content = r#"
[output]
format = ["text", "json"]
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    // This might parse but fail type conversion
    assert!(result.is_err());
}

// ============================================================================
// Type Mismatch Error Scenarios
// ============================================================================

#[test]
fn test_type_mismatch_default_is_number() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
default = 123
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse") || error_msg.contains("string"));
}

#[test]
fn test_type_mismatch_timeout_is_string() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
command = "test"
model = "test"
timeout = "not_a_number"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse") || error_msg.contains("integer"));
}

#[test]
fn test_type_mismatch_run_tests_is_string() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[modes.fast]
run_tests = "yes"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse") || error_msg.contains("boolean"));
}

#[test]
fn test_type_mismatch_max_retries_is_float() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[modes.fast]
max_retries = 3.5
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    // TOML may accept float but Rust type expects integer
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse") || error_msg.contains("integer"));
}

// ============================================================================
// Boundary Value Tests
// ============================================================================

#[test]
fn test_timeout_at_zero_boundary() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
command = "test"
model = "test"
timeout = 0
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.agents["test"].timeout, Some(0));
}

#[test]
fn test_timeout_at_maximum_boundary() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
command = "test"
model = "test"
timeout = 86400
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.agents["test"].timeout, Some(86400));
}

#[test]
fn test_max_retries_at_zero() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[modes.fast]
max_retries = 0
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.modes.fast.unwrap().max_retries, 0);
}

#[test]
fn test_max_depth_at_zero() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[modes.fast]
max_depth = 0
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.modes.fast.unwrap().max_depth, 0);
}

#[test]
fn test_timeout_plan_at_minimum() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[modes.fast]
timeout_plan = 1
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.modes.fast.unwrap().timeout_plan, 1);
}

#[test]
fn test_timeout_exec_at_minimum() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[modes.fast]
timeout_exec = 1
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.modes.fast.unwrap().timeout_exec, 1);
}

// ============================================================================
// Empty and Whitespace Tests
// ============================================================================

#[test]
fn test_config_with_only_whitespace() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = "   \n\t\n   ";
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, None);
    assert!(config.agents.is_empty());
}

#[test]
fn test_config_with_leading_trailing_whitespace() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"

default = "test"

[agents.test]
command = "  test  "
model = "model"

"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    // Strings should preserve internal whitespace but not leading/trailing
    assert_eq!(config.agents["test"].command, Some("  test  ".to_string()));
}

// ============================================================================
// Special Characters and Encoding Tests
// ============================================================================

#[test]
fn test_config_with_tab_characters() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // TOML allows tabs in string values
    let content = r#"
[agents.test]
command = "test\twith\ttabs"
model = "test"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(
        config.agents["test"].command,
        Some("test\twith\ttabs".to_string())
    );
}

#[test]
fn test_config_with_newlines_in_strings() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Triple-quoted strings allow newlines
    let content = r#"
[agents.test]
command = """
line1
line2
line3
"""
model = "test"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    // Triple-quoted strings include newlines
    assert!(config.agents["test"]
        .command
        .as_ref()
        .unwrap()
        .contains('\n'));
}

#[test]
fn test_config_with_unicode_escapes() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Unicode escape sequences - TOML supports \uXXXX (4 hex digits) format
    // Using \u00E9 (é) as a valid example
    let content = r#"
[agents.test]
command = "test\u00e9"
model = "test"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    // Should decode unicode escape to é
    assert!(config.agents["test"]
        .command
        .as_ref()
        .unwrap()
        .contains('é'));
}

// ============================================================================
// Large Input Tests
// ============================================================================

#[test]
fn test_config_with_many_agents() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let mut content = String::from("default = \"agent0\"\n\n");

    // Create 50 agents
    for i in 0..50 {
        content.push_str(&format!(
            r#"
[agents.agent{}]
command = "command{}"
model = "model{}"
timeout = {}
"#,
            i,
            i,
            i,
            i * 100
        ));
    }

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.agents.len(), 50);
    assert!(config.agents.contains_key("agent0"));
    assert!(config.agents.contains_key("agent49"));
}

#[test]
fn test_config_with_very_long_string_value() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Create a very long command string
    let long_command = "a".repeat(10000);
    let content = format!(
        r#"
[agents.test]
command = "{}"
model = "test"
"#,
        long_command
    );

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.agents["test"].command.as_ref().unwrap().len(), 10000);
}

// ============================================================================
// Merge Logic Edge Cases
// ============================================================================

#[test]
fn test_merge_with_none_values_in_override() {
    // Test that None values in override don't override base values
    let mut base = Config::default();
    base.default = Some("base-agent".to_string());

    let override_config = Config {
        default: None, // Should not override base
        agents: std::collections::HashMap::new(),
        modes: ltmatrix::config::settings::ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
    };

    let merged = merge_configs(Some(base), Some(override_config));

    assert_eq!(merged.default, Some("base-agent".to_string()));
}

#[test]
fn test_merge_with_none_values_in_base() {
    // Test that base None values don't prevent override from being applied
    let base = Config {
        default: None,
        agents: std::collections::HashMap::new(),
        modes: ltmatrix::config::settings::ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
    };

    let mut override_config = Config::default();
    override_config.default = Some("override-agent".to_string());

    let merged = merge_configs(Some(base), Some(override_config));

    assert_eq!(merged.default, Some("override-agent".to_string()));
}

#[test]
fn test_merge_preserves_all_unique_modes() {
    let mut base = Config::default();
    base.modes.fast = Some(ModeConfig {
        model: Some("fast".to_string()),
        run_tests: false,
        verify: true,
        max_retries: 1,
        max_depth: 2,
        timeout_plan: 60,
        timeout_exec: 1800,
    });

    let mut override_config = Config::default();
    override_config.modes.standard = Some(ModeConfig {
        model: Some("standard".to_string()),
        run_tests: true,
        verify: true,
        max_retries: 3,
        max_depth: 3,
        timeout_plan: 120,
        timeout_exec: 3600,
    });

    let merged = merge_configs(Some(base), Some(override_config));

    // Both modes should be present
    assert!(merged.modes.fast.is_some());
    assert!(merged.modes.standard.is_some());
    assert_eq!(merged.modes.fast.unwrap().model, Some("fast".to_string()));
    assert_eq!(
        merged.modes.standard.unwrap().model,
        Some("standard".to_string())
    );
}

#[test]
fn test_merge_agent_with_partial_override() {
    let mut base = Config::default();
    base.agents.insert(
        "test".to_string(),
        AgentConfig {
            command: Some("base-command".to_string()),
            model: Some("base-model".to_string()),
            timeout: Some(1000),
        },
    );

    let mut override_config = Config::default();
    override_config.agents.insert(
        "test".to_string(),
        AgentConfig {
            command: None,                             // Keep base
            model: Some("override-model".to_string()), // Override
            timeout: None,                             // Keep base
        },
    );

    let merged = merge_configs(Some(base), Some(override_config));

    let agent = &merged.agents["test"];
    assert_eq!(agent.command, Some("base-command".to_string()));
    assert_eq!(agent.model, Some("override-model".to_string()));
    assert_eq!(agent.timeout, Some(1000));
}

// ============================================================================
// Output Format Tests
// ============================================================================

#[test]
fn test_all_output_formats() {
    let formats = vec![("text", OutputFormat::Text), ("json", OutputFormat::Json)];

    for (format_str, expected_format) in formats {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let content = format!(
            r#"
[output]
format = "{}"
"#,
            format_str
        );

        fs::write(&config_path, content).unwrap();

        let result = load_config_file(&config_path);
        assert!(result.is_ok(), "Should parse format: {}", format_str);

        let config = result.unwrap();
        assert_eq!(config.output.format, expected_format);
    }
}

#[test]
fn test_invalid_output_format() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[output]
format = "invalid"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse") || error_msg.contains("unknown variant"));
}

// ============================================================================
// Log Level Tests
// ============================================================================

#[test]
fn test_all_log_levels() {
    let levels = vec![
        ("trace", LogLevel::Trace),
        ("debug", LogLevel::Debug),
        ("info", LogLevel::Info),
        ("warn", LogLevel::Warn),
        ("error", LogLevel::Error),
    ];

    for (level_str, expected_level) in levels {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let content = format!(
            r#"
[logging]
level = "{}"
"#,
            level_str
        );

        fs::write(&config_path, content).unwrap();

        let result = load_config_file(&config_path);
        assert!(result.is_ok(), "Should parse log level: {}", level_str);

        let config = result.unwrap();
        assert_eq!(config.logging.level, expected_level);
    }
}

#[test]
fn test_invalid_log_level() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[logging]
level = "invalid"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse") || error_msg.contains("unknown variant"));
}

#[test]
fn test_log_level_case_sensitivity() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Log levels should be lowercase (per serde rename_all = "lowercase")
    let content = r#"
[logging]
level = "DEBUG"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());
}

// ============================================================================
// Error Message Quality Tests
// ============================================================================

#[test]
fn test_error_message_includes_file_path() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[invalid
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    // Error message should include the file path
    let path_str = config_path.to_string_lossy().to_string();
    assert!(
        error_msg.contains(&path_str) || error_msg.contains("config.toml"),
        "Error message should include file path"
    );
}

#[test]
fn test_error_message_is_descriptive() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
timeout = "not_a_number"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    // Error message should be descriptive
    assert!(error_msg.len() > 10, "Error message should be descriptive");
}
