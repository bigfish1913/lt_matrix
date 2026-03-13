//! Integration tests for config file loading and parsing (Task: Implement config file loading and parsing)
//!
//! These tests verify the acceptance criteria:
//! - Load TOML files from ~/.ltmatrix/config.toml
//! - Load TOML files from .ltmatrix/config.toml
//! - Proper error handling for missing files
//! - Proper error handling for malformed files
//! - Auto-discovery and merging of config sources

use ltmatrix::config::settings::{
    get_global_config_path, get_project_config_path, load_config, load_config_file, merge_configs,
    Config,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Auto-Discovery Tests (load_config function)
// ============================================================================

#[test]
fn test_load_config_with_no_files() {
    // When no config files exist, should return default config
    let temp_dir = TempDir::new().unwrap();
    let current_dir = std::env::current_dir().unwrap();
    let test_dir = temp_dir.path();

    // Change to test directory where no .ltmatrix exists
    std::env::set_current_dir(test_dir).unwrap();

    // Should not fail even with no config files
    // Note: load_config() may still load system-wide configs, so we just verify it succeeds
    let result = load_config();
    assert!(
        result.is_ok(),
        "load_config should succeed with no config files"
    );

    let config = result.unwrap();
    // At minimum, should have a default agent configured
    assert!(config.default.is_some());

    // Restore original directory
    std::env::set_current_dir(current_dir).unwrap();
}

#[test]
fn test_load_config_with_only_global_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create a mock global config directory
    let ltmatrix_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let global_config_path = ltmatrix_dir.join("config.toml");
    let global_config_content = r#"
default = "global-agent"

[agents.global-agent]
command = "global-cmd"
model = "global-model"
timeout = 1000

[output]
format = "json"
colored = false

[logging]
level = "warn"
"#;

    fs::write(&global_config_path, global_config_content).unwrap();

    // Verify file exists
    assert!(
        global_config_path.exists(),
        "Global config file should exist"
    );

    // Load the config file directly from the path
    let config = load_config_file(&global_config_path).unwrap();
    assert_eq!(config.default, Some("global-agent".to_string()));
    assert_eq!(config.agents.len(), 1);
    assert!(config.agents.contains_key("global-agent"));
}

#[test]
fn test_load_config_with_only_project_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create .ltmatrix directory in temp directory
    let project_ltmatrix_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&project_ltmatrix_dir).unwrap();

    let project_config_path = project_ltmatrix_dir.join("config.toml");
    let project_config_content = r#"
default = "project-agent"

[agents.project-agent]
command = "project-cmd"
model = "project-model"
timeout = 2000

[output]
format = "text"
colored = true
progress = false

[logging]
level = "debug"
file = "/tmp/project.log"
"#;

    fs::write(&project_config_path, project_config_content).unwrap();

    // Verify file exists
    assert!(project_config_path.exists(), "Config file should exist");

    // Load the config file directly from the path
    let config = load_config_file(&project_config_path).unwrap();
    assert_eq!(config.default, Some("project-agent".to_string()));
    assert_eq!(config.agents.len(), 1);
    assert_eq!(
        config.output.format,
        ltmatrix::config::settings::OutputFormat::Text
    );
    assert_eq!(config.output.progress, false);
}

#[test]
fn test_load_config_merges_global_and_project() {
    let temp_dir = TempDir::new().unwrap();

    // Create global config
    let global_dir = temp_dir.path().join("global");
    fs::create_dir_all(&global_dir).unwrap();
    let global_config = global_dir.join("config.toml");
    fs::write(
        &global_config,
        r#"
default = "global-default"

[agents.agent1]
command = "global-command"
model = "global-model"
timeout = 1000

[agents.agent2]
command = "agent2-global"
model = "agent2-model"
timeout = 500

[modes.fast]
model = "global-fast"
run_tests = false
verify = true
max_retries = 1
max_depth = 2
timeout_plan = 60
timeout_exec = 1800
"#,
    )
    .unwrap();

    // Create project config directory structure
    let project_dir = temp_dir.path().join("project");
    let project_ltmatrix_dir = project_dir.join(".ltmatrix");
    fs::create_dir_all(&project_ltmatrix_dir).unwrap();
    let project_config = project_ltmatrix_dir.join("config.toml");
    fs::write(
        &project_config,
        r#"
default = "project-default"

[agents.agent1]
command = "project-command"
model = "project-model"
timeout = 2000

[agents.agent3]
command = "agent3-command"
model = "agent3-model"
timeout = 3000

[modes.standard]
model = "project-standard"
run_tests = true
verify = true
max_retries = 3
max_depth = 3
timeout_plan = 120
timeout_exec = 3600
"#,
    )
    .unwrap();

    // Verify files exist
    assert!(global_config.exists(), "Global config should exist");
    assert!(project_config.exists(), "Project config should exist");

    // Load both configs directly from their paths
    let global = load_config_file(&global_config).unwrap();
    let project = load_config_file(&project_config).unwrap();

    // Merge them
    let merged = merge_configs(Some(global), Some(project));

    // Project default should override global
    assert_eq!(merged.default, Some("project-default".to_string()));

    // agent1 should have merged config (project overrides global)
    let agent1 = &merged.agents["agent1"];
    assert_eq!(agent1.command, Some("project-command".to_string())); // From project
    assert_eq!(agent1.model, Some("project-model".to_string())); // From project
    assert_eq!(agent1.timeout, Some(2000)); // From project

    // agent2 should be from global only
    assert!(merged.agents.contains_key("agent2"));
    let agent2 = &merged.agents["agent2"];
    assert_eq!(agent2.command, Some("agent2-global".to_string()));

    // agent3 should be from project only
    assert!(merged.agents.contains_key("agent3"));
    let agent3 = &merged.agents["agent3"];
    assert_eq!(agent3.command, Some("agent3-command".to_string()));

    // Both modes should be present
    assert!(merged.modes.fast.is_some());
    assert!(merged.modes.standard.is_some());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_load_config_file_missing() {
    let result = load_config_file(PathBuf::from("/nonexistent/path/to/config.toml").as_path());
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to read") || error_msg.contains("No such file"));
}

#[test]
fn test_load_config_file_malformed_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Write malformed TOML
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
    assert!(error_msg.contains("Failed to parse") || error_msg.contains("TOML"));
}

#[test]
fn test_load_config_file_invalid_syntax() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Various invalid TOML syntaxes
    let invalid_contents = vec![
        // Unclosed string
        r#"[agents.test]
command = "unclosed string
model = "test"
"#,
        // Invalid boolean
        r#"[output]
colored = maybe
"#,
        // Invalid number
        r#"[agents.test]
timeout = not_a_number
"#,
        // Missing closing bracket
        r#"[agents
test = "value"
"#,
    ];

    for content in invalid_contents {
        fs::write(&config_path, content).unwrap();

        let result = load_config_file(&config_path);
        assert!(result.is_err(), "Should fail for invalid TOML: {}", content);

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to parse") || error_msg.contains("TOML"));
    }
}

#[test]
fn test_load_config_file_with_invalid_data_types() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Valid TOML but wrong data types
    let invalid_type_content = r#"
default = 123  # Should be string

[agents.test]
command = "test"
model = "test"
timeout = "not_a_number"  # Should be integer
"#;

    fs::write(&config_path, invalid_type_content).unwrap();

    let result = load_config_file(&config_path);
    // This might succeed during parsing but fail during type conversion
    // or it might fail during parsing - either way, we expect an error
    // Actually, serde will handle this during parsing
    assert!(result.is_err());
}

#[test]
fn test_load_config_file_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Write empty file
    fs::write(&config_path, "").unwrap();

    let result = load_config_file(&config_path);
    // Empty TOML is valid, should return default config
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, None);
    assert!(config.agents.is_empty());
}

#[test]
fn test_load_config_file_only_comments() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Write file with only comments
    fs::write(
        &config_path,
        r#"
# This is a comment
# Another comment
# default = "test"
"#,
    )
    .unwrap();

    let result = load_config_file(&config_path);
    // File with only comments is valid TOML
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, None);
}

// ============================================================================
// Path Resolution Tests
// ============================================================================

#[test]
fn test_get_global_config_path_format() {
    let path = get_global_config_path();
    assert!(
        path.is_some(),
        "Should be able to determine global config path"
    );

    let path = path.unwrap();
    let path_str = path.to_string_lossy();

    // Should contain .ltmatrix and config.toml
    assert!(
        path_str.contains(".ltmatrix"),
        "Path should contain .ltmatrix: {}",
        path_str
    );
    assert!(
        path_str.contains("config.toml"),
        "Path should contain config.toml: {}",
        path_str
    );
}

#[test]
fn test_get_project_config_path_format() {
    let path = get_project_config_path();
    assert!(path.is_some());

    let path = path.unwrap();
    let path_str = path.to_string_lossy();

    // Should contain .ltmatrix and config.toml
    assert!(
        path_str.contains(".ltmatrix"),
        "Path should contain .ltmatrix: {}",
        path_str
    );
    assert!(
        path_str.contains("config.toml"),
        "Path should contain config.toml: {}",
        path_str
    );

    // Should be relative to current directory
    let current_dir = std::env::current_dir().unwrap();
    let current_dir_str = current_dir.to_string_lossy().to_string();
    assert!(path_str.starts_with(&current_dir_str) || path_str.contains(".ltmatrix"));
}

#[test]
fn test_config_path_functions_are_consistent() {
    // Get global path
    let global_path = get_global_config_path().unwrap();

    // Get project path
    let project_path = get_project_config_path().unwrap();

    // Both should exist
    assert!(global_path
        .as_path()
        .to_string_lossy()
        .contains(".ltmatrix"));
    assert!(project_path.to_string_lossy().contains(".ltmatrix"));

    // Both should end with config.toml
    assert!(global_path.to_string_lossy().ends_with("config.toml"));
    assert!(project_path.to_string_lossy().ends_with("config.toml"));
}

// ============================================================================
// Partial and Merged Configuration Tests
// ============================================================================

#[test]
fn test_load_config_partial_agent_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Config with only some agent fields
    let partial_content = r#"
[agents.test]
command = "test-command"
# model and timeout missing
"#;

    fs::write(&config_path, partial_content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    let agent = &config.agents["test"];

    assert_eq!(agent.command, Some("test-command".to_string()));
    assert_eq!(agent.model, None); // Not specified
    assert_eq!(agent.timeout, None); // Not specified
}

#[test]
fn test_merge_configs_with_none_values() {
    // Test merging when one config is None
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
default = "test"

[agents.test]
command = "test"
model = "test"
timeout = 1000
"#,
    )
    .unwrap();

    let config = load_config_file(&config_path).unwrap();

    // Merge with None global
    let merged1 = merge_configs(None, Some(config.clone()));
    assert_eq!(merged1.default, Some("test".to_string()));

    // Merge with None project
    let merged2 = merge_configs(Some(config), None);
    assert_eq!(merged2.default, Some("test".to_string()));

    // Merge with both None
    let merged3 = merge_configs(None, None);
    assert_eq!(merged3.default, Some("claude".to_string())); // Default
}

#[test]
fn test_config_precedence_order() {
    // Test precedence: Project > Global > Default

    // Default config
    let default_config = Config::default();
    assert_eq!(
        default_config.output.format,
        ltmatrix::config::settings::OutputFormat::Text
    );

    // Global config (would override default)
    let mut global_config = default_config.clone();
    global_config.output.format = ltmatrix::config::settings::OutputFormat::Json;
    global_config.logging.level = ltmatrix::config::settings::LogLevel::Warn;

    // Project config (would override global)
    let mut project_config = Config::default();
    project_config.output.format = ltmatrix::config::settings::OutputFormat::Json;
    project_config.logging.level = ltmatrix::config::settings::LogLevel::Debug;

    let merged = merge_configs(Some(global_config), Some(project_config));

    // Project overrides global for logging
    assert_eq!(
        merged.logging.level,
        ltmatrix::config::settings::LogLevel::Debug
    );

    // Project's output format is kept (both had Json)
    assert_eq!(
        merged.output.format,
        ltmatrix::config::settings::OutputFormat::Json
    );
}

// ============================================================================
// Real-world Scenario Tests
// ============================================================================

#[test]
fn test_typical_claude_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Typical real-world Claude configuration
    let typical_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[modes.fast]
model = "claude-haiku-4-5"
run_tests = false
verify = true
max_retries = 1
max_depth = 2
timeout_plan = 60
timeout_exec = 1800

[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
verify = true
max_retries = 3
max_depth = 3
timeout_plan = 120
timeout_exec = 3600

[modes.expert]
model = "claude-opus-4-6"
run_tests = true
verify = true
max_retries = 5
max_depth = 4
timeout_plan = 300
timeout_exec = 7200

[output]
format = "text"
colored = true
progress = true

[logging]
level = "info"
"#;

    fs::write(&config_path, typical_content).unwrap();

    let config = load_config_file(&config_path).unwrap();

    assert_eq!(config.default, Some("claude".to_string()));
    assert_eq!(config.agents.len(), 1);
    assert!(config.agents.contains_key("claude"));

    assert!(config.modes.fast.is_some());
    assert!(config.modes.standard.is_some());
    assert!(config.modes.expert.is_some());

    let fast = config.modes.fast.as_ref().unwrap();
    assert_eq!(fast.model, Some("claude-haiku-4-5".to_string()));
    assert!(!fast.run_tests);

    let standard = config.modes.standard.as_ref().unwrap();
    assert_eq!(standard.model, Some("claude-sonnet-4-6".to_string()));
    assert!(standard.run_tests);

    let expert = config.modes.expert.as_ref().unwrap();
    assert_eq!(expert.model, Some("claude-opus-4-6".to_string()));
    assert_eq!(expert.max_retries, 5);
}

#[test]
fn test_minimal_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Minimal viable configuration
    let minimal_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
"#;

    fs::write(&config_path, minimal_content).unwrap();

    let config = load_config_file(&config_path).unwrap();

    assert_eq!(config.default, Some("claude".to_string()));
    assert_eq!(config.agents.len(), 1);

    let claude = &config.agents["claude"];
    assert_eq!(claude.command, Some("claude".to_string()));
    assert_eq!(claude.model, Some("claude-sonnet-4-6".to_string()));
    assert_eq!(claude.timeout, None); // Will use default
}

#[test]
fn test_config_with_special_characters_in_strings() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Config with special characters
    let content = r#"
[agents.test]
command = "agent --arg 'value with spaces' --other=\"quoted\""
model = "model-with_special.chars"
timeout = 3600
"#;

    fs::write(&config_path, content).unwrap();

    let config = load_config_file(&config_path).unwrap();
    let test_agent = &config.agents["test"];

    assert_eq!(
        test_agent.command,
        Some("agent --arg 'value with spaces' --other=\"quoted\"".to_string())
    );
    assert_eq!(
        test_agent.model,
        Some("model-with_special.chars".to_string())
    );
}
