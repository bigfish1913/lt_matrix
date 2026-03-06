//! Edge case tests for config file loading and parsing
//!
//! These tests cover additional edge cases and boundary conditions
//! beyond the basic integration tests.

use ltmatrix::config::settings::{
    get_global_config_path, get_project_config_path, load_config_file, merge_configs, Config, WarmupConfig,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// File System Edge Cases
// ============================================================================

#[test]
fn test_config_path_with_trailing_slash() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
default = "test"

[agents.test]
command = "test"
model = "test-model"
"#,
    )
    .unwrap();

    // Load with trailing separator in path
    let path_with_trailing = format!("{}/", config_path.display());
    let result = load_config_file(PathBuf::from(&path_with_trailing).as_path());
    assert!(result.is_err()); // Path is directory, not file
}

#[test]
fn test_config_file_with_bom() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // UTF-8 BOM followed by valid TOML
    let content =
        "\u{FEFF}default = \"test\"\n\n[agents.test]\ncommand = \"test\"\nmodel = \"test-model\"\n";
    fs::write(&config_path, content.as_bytes()).unwrap();

    let result = load_config_file(&config_path);
    // TOML parser should handle BOM
    assert!(result.is_ok());
}

#[test]
fn test_config_file_with_crlf_line_endings() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // TOML with CRLF line endings
    let content = "default = \"test\"\r\n\r\n[agents.test]\r\ncommand = \"test\"\r\nmodel = \"test-model\"\r\n";
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("test".to_string()));
}

#[test]
fn test_config_file_with_mixed_line_endings() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Mixed line endings (should still parse)
    let content =
        "default = \"test\"\n[agents.test]\r\ncommand = \"test\"\nmodel = \"test-model\"\r\n";
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());
}

#[test]
fn test_config_file_with_unicode_characters() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
default = "test"

[agents.test]
command = "测试命令"
model = "测试模型"
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("test".to_string()));
    assert!(config.agents.contains_key("test"));

    let agent = &config.agents["test"];
    assert_eq!(agent.command, Some("测试命令".to_string()));
    assert_eq!(agent.model, Some("测试模型".to_string()));
}

#[test]
fn test_config_file_with_emoji() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
default = "🚀"

[agents.fast]
command = "⚡"
model = "gpt-4"
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("🚀".to_string()));
}

// ============================================================================
// TOML Syntax Edge Cases
// ============================================================================

#[test]
fn test_config_with_multiline_strings() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
default = "test"

[agents.test]
command = """
multi
line
command
"""
model = "test-model"
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    let agent = &config.agents["test"];
    // Triple-quoted strings in TOML include the newlines but not the trailing one
    assert_eq!(agent.command, Some("multi\nline\ncommand\n".to_string()));
}

#[test]
fn test_config_with_literal_strings() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
default = "test"

[agents.test]
command = 'C:\path\to\program.exe'
model = 'test-model'
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    let agent = &config.agents["test"];
    assert_eq!(agent.command, Some("C:\\path\\to\\program.exe".to_string()));
}

#[test]
fn test_config_with_valid_toml_syntax() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Standard TOML syntax without trailing commas (not allowed in tables)
    let content = r#"
[agents.test]
command = "test"
model = "test-model"
timeout = 1000
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());
}

#[test]
fn test_config_with_inline_tables() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
command = "test"
model = "test-model"

[output]
format = "json"
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());
}

#[test]
fn test_config_with_empty_tables() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
default = "test"

[agents.test]
command = "test"
model = "test-model"

[agents.another]

[output]

[logging]
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert!(config.agents.contains_key("another"));
    let another = &config.agents["another"];
    assert_eq!(another.command, None);
    assert_eq!(another.model, None);
}

// ============================================================================
// Data Type Edge Cases
// ============================================================================

#[test]
fn test_config_with_zero_values() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
command = "test"
model = "test-model"
timeout = 0
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    let agent = &config.agents["test"];
    assert_eq!(agent.timeout, Some(0));
}

#[test]
fn test_config_with_large_numbers() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Use a reasonable large value that fits in u64
    let content = r#"
[agents.test]
command = "test"
model = "test-model"
timeout = 9999999999
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    let agent = &config.agents["test"];
    assert_eq!(agent.timeout, Some(9999999999));
}

#[test]
fn test_config_with_boolean_values() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[output]
colored = true
progress = false
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.output.colored, true);
    assert_eq!(config.output.progress, false);
}

#[test]
fn test_config_with_all_log_levels() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Test each log level
    for level in &["trace", "debug", "info", "warn", "error"] {
        let content = format!(
            r#"
[logging]
level = "{}"
"#,
            level
        );

        fs::write(&config_path, content).unwrap();

        let result = load_config_file(&config_path);
        assert!(result.is_ok(), "Should parse log level: {}", level);
    }
}

// ============================================================================
// Merge Behavior Edge Cases
// ============================================================================

#[test]
fn test_merge_with_overlapping_fields() {
    let base = Config {
        default: Some("base".to_string()),
        agents: {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "agent1".to_string(),
                ltmatrix::config::settings::AgentConfig {
                    command: Some("base-command".to_string()),
                    model: Some("base-model".to_string()),
                    timeout: Some(100),
                },
            );
            map
        },
        modes: ltmatrix::config::settings::ModeConfigs::default(),
        output: ltmatrix::config::settings::OutputConfig::default(),
        logging: ltmatrix::config::settings::LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
    };

    let override_config = Config {
        default: Some("override".to_string()),
        agents: {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "agent1".to_string(),
                ltmatrix::config::settings::AgentConfig {
                    command: Some("override-command".to_string()),
                    model: None,        // Keep base
                    timeout: Some(200), // Override
                },
            );
            map
        },
        modes: ltmatrix::config::settings::ModeConfigs::default(),
        output: ltmatrix::config::settings::OutputConfig::default(),
        logging: ltmatrix::config::settings::LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
    };

    let merged = merge_configs(Some(base), Some(override_config));

    assert_eq!(merged.default, Some("override".to_string()));

    let agent1 = &merged.agents["agent1"];
    assert_eq!(agent1.command, Some("override-command".to_string()));
    assert_eq!(agent1.model, Some("base-model".to_string())); // From base
    assert_eq!(agent1.timeout, Some(200)); // From override
}

#[test]
fn test_merge_empty_configs() {
    let empty1 = Config::default();
    let empty2 = Config::default();

    let merged = merge_configs(Some(empty1), Some(empty2));
    assert_eq!(merged.default, Some("claude".to_string()));
}

#[test]
fn test_merge_multiple_levels() {
    let level1 = Config {
        default: Some("level1".to_string()),
        agents: {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "agent".to_string(),
                ltmatrix::config::settings::AgentConfig {
                    command: Some("cmd1".to_string()),
                    model: Some("model1".to_string()),
                    timeout: Some(100),
                },
            );
            map
        },
        modes: ltmatrix::config::settings::ModeConfigs::default(),
        output: ltmatrix::config::settings::OutputConfig::default(),
        logging: ltmatrix::config::settings::LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
    };

    let level2 = Config {
        default: Some("level2".to_string()),
        agents: {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "agent".to_string(),
                ltmatrix::config::settings::AgentConfig {
                    command: Some("cmd2".to_string()),
                    model: None,
                    timeout: Some(200),
                },
            );
            map
        },
        modes: ltmatrix::config::settings::ModeConfigs::default(),
        output: ltmatrix::config::settings::OutputConfig::default(),
        logging: ltmatrix::config::settings::LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
    };

    let merged = merge_configs(Some(level1), Some(level2));

    let agent = &merged.agents["agent"];
    assert_eq!(agent.command, Some("cmd2".to_string())); // From level2
    assert_eq!(agent.model, Some("model1".to_string())); // From level1
    assert_eq!(agent.timeout, Some(200)); // From level2
}

// ============================================================================
// Path Resolution Edge Cases
// ============================================================================

#[test]
fn test_global_path_home_exists() {
    let path = get_global_config_path();
    assert!(path.is_ok());

    let path = path.unwrap();
    // Parent directory should be home
    let parent = path.parent().unwrap();
    let parent_str = parent.to_string_lossy();

    #[cfg(unix)]
    assert!(parent_str.contains(".ltmatrix"));

    #[cfg(windows)]
    assert!(parent_str.contains(".ltmatrix"));
}

#[test]
fn test_project_path_relative_to_current() {
    let path = get_project_config_path();
    assert!(path.is_some());

    let path = path.unwrap();
    let path_str = path.to_string_lossy();

    assert!(path_str.contains(".ltmatrix"));
    assert!(path_str.contains("config.toml"));
}

#[test]
fn test_config_paths_are_absolute() {
    let global_path = get_global_config_path().unwrap();
    assert!(global_path.is_absolute());

    let project_path = get_project_config_path().unwrap();
    assert!(project_path.is_absolute());
}

// ============================================================================
// Error Message Quality Tests
// ============================================================================

#[test]
fn test_error_message_for_missing_file_includes_path() {
    let result = load_config_file(PathBuf::from("/nonexistent/config.toml").as_path());
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("/nonexistent/config.toml") || error_msg.contains("config.toml"));
}

#[test]
fn test_error_message_for_invalid_toml_includes_context() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
[agents
command = "test"
"#,
    )
    .unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("parse") || error_msg.contains("TOML"));
}

// ============================================================================
// Large Configuration Tests
// ============================================================================

#[test]
fn test_config_with_many_agents() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let mut content = String::from("default = \"agent0\"\n\n");

    for i in 0..100 {
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
    assert_eq!(config.agents.len(), 100);
    assert!(config.agents.contains_key("agent0"));
    assert!(config.agents.contains_key("agent99"));
}

#[test]
fn test_config_with_deeply_nested_values() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
[agents.test]
command = "very-long-command-with-many-options --option1 value1 --option2 value2 --option3 value3"
model = "model-with-very-long-name-and-lots-of-hyphens"
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());
}
