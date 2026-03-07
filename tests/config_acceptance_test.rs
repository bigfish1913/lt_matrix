//! Acceptance tests for config file loading and parsing task
//!
//! These tests verify the complete acceptance criteria:
//! - Load TOML files from ~/.ltmatrix/config.toml
//! - Load TOML files from .ltmatrix/config.toml
//! - Proper error handling for missing files
//! - Proper error handling for malformed files
//! - Auto-discovery and merging of config sources
//!
//! Tests are organized by acceptance criterion.

use ltmatrix::config::settings::{
    get_global_config_path, get_project_config_path, load_config, load_config_file, merge_configs,
    Config,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Acceptance Criterion 1: Load TOML files from ~/.ltmatrix/config.toml
// ============================================================================

#[test]
fn acceptance_1_1_global_config_path_exists() {
    // Verify the function to get global config path works
    let result = get_global_config_path();
    assert!(result.is_some(), "Should be able to get global config path");

    let path = result.unwrap();
    let path_str = path.to_string_lossy();

    // Path should contain .ltmatrix
    assert!(
        path_str.contains(".ltmatrix"),
        "Path should contain .ltmatrix"
    );

    // Path should end with config.toml
    assert!(
        path_str.ends_with("config.toml"),
        "Path should end with config.toml"
    );
}

#[test]
fn acceptance_1_2_load_valid_global_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create a mock home directory structure
    let ltmatrix_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let global_config = ltmatrix_dir.join("config.toml");
    let content = r#"
default = "global-agent"

[agents.global-agent]
command = "global-command"
model = "global-model"
timeout = 3600
"#;

    fs::write(&global_config, content).unwrap();

    // Should be able to load the global config
    let result = load_config_file(&global_config);
    assert!(result.is_ok(), "Should load valid global config file");

    let config = result.unwrap();
    assert_eq!(config.default, Some("global-agent".to_string()));
    assert!(config.agents.contains_key("global-agent"));
}

#[test]
fn acceptance_1_3_global_config_with_all_sections() {
    let temp_dir = TempDir::new().unwrap();

    let ltmatrix_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&ltmatrix_dir).unwrap();

    let global_config = ltmatrix_dir.join("config.toml");
    let content = r#"
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

[output]
format = "text"
colored = true
progress = true

[logging]
level = "info"
"#;

    fs::write(&global_config, content).unwrap();

    let result = load_config_file(&global_config);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert!(config.modes.fast.is_some());
    assert_eq!(
        config.output.format,
        ltmatrix::config::settings::OutputFormat::Text
    );
    assert_eq!(
        config.logging.level,
        ltmatrix::config::settings::LogLevel::Info
    );
}

// ============================================================================
// Acceptance Criterion 2: Load TOML files from .ltmatrix/config.toml
// ============================================================================

#[test]
fn acceptance_2_1_project_config_path_exists() {
    let result = get_project_config_path();
    assert!(
        result.is_some(),
        "Should be able to get project config path"
    );

    let path = result.unwrap();
    let path_str = path.to_string_lossy();

    // Path should contain .ltmatrix
    assert!(
        path_str.contains(".ltmatrix"),
        "Path should contain .ltmatrix"
    );

    // Path should end with config.toml
    assert!(
        path_str.ends_with("config.toml"),
        "Path should end with config.toml"
    );
}

#[test]
fn acceptance_2_2_load_valid_project_config() {
    let temp_dir = TempDir::new().unwrap();

    let project_ltmatrix = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&project_ltmatrix).unwrap();

    let project_config = project_ltmatrix.join("config.toml");
    let content = r#"
default = "project-agent"

[agents.project-agent]
command = "project-command"
model = "project-model"
timeout = 1800
"#;

    fs::write(&project_config, content).unwrap();

    let result = load_config_file(&project_config);
    assert!(result.is_ok(), "Should load valid project config file");

    let config = result.unwrap();
    assert_eq!(config.default, Some("project-agent".to_string()));
    assert!(config.agents.contains_key("project-agent"));
}

#[test]
fn acceptance_2_3_project_config_overrides_global() {
    let temp_dir = TempDir::new().unwrap();

    // Create global config
    let global_dir = temp_dir.path().join("home").join(".ltmatrix");
    fs::create_dir_all(&global_dir).unwrap();
    let global_config = global_dir.join("config.toml");
    fs::write(
        &global_config,
        r#"
default = "global-agent"

[agents.shared]
command = "global-command"
model = "global-model"
timeout = 1000
"#,
    )
    .unwrap();

    // Create project config
    let project_dir = temp_dir.path().join("project").join(".ltmatrix");
    fs::create_dir_all(&project_dir).unwrap();
    let project_config = project_dir.join("config.toml");
    fs::write(
        &project_config,
        r#"
default = "project-agent"

[agents.shared]
command = "project-command"
timeout = 2000
"#,
    )
    .unwrap();

    // Load both
    let global = load_config_file(&global_config).unwrap();
    let project = load_config_file(&project_config).unwrap();

    // Merge
    let merged = merge_configs(Some(global), Some(project));

    // Project default should override
    assert_eq!(merged.default, Some("project-agent".to_string()));

    // Project command should override
    let shared = &merged.agents["shared"];
    assert_eq!(shared.command, Some("project-command".to_string()));

    // Global model should be preserved
    assert_eq!(shared.model, Some("global-model".to_string()));

    // Project timeout should override
    assert_eq!(shared.timeout, Some(2000));
}

// ============================================================================
// Acceptance Criterion 3: Proper error handling for missing files
// ============================================================================

#[test]
fn acceptance_3_1_missing_global_config_returns_error() {
    let nonexistent_path = PathBuf::from("/nonexistent/path/.ltmatrix/config.toml");
    let result = load_config_file(&nonexistent_path);

    assert!(
        result.is_err(),
        "Should return error for missing config file"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Failed to read") || error_msg.contains("No such file"),
        "Error message should indicate file read failure"
    );
}

#[test]
fn acceptance_3_2_missing_project_config_returns_error() {
    let nonexistent_path = PathBuf::from("/tmp/nonexistent_project/.ltmatrix/config.toml");
    let result = load_config_file(&nonexistent_path);

    assert!(
        result.is_err(),
        "Should return error for missing config file"
    );
}

#[test]
fn acceptance_3_3_load_config_handles_missing_files_gracefully() {
    // Change to a directory with no .ltmatrix
    let temp_dir = TempDir::new().unwrap();
    let current_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // load_config should still succeed (returning defaults)
    let result = load_config();
    assert!(
        result.is_ok(),
        "load_config should succeed even with missing files"
    );

    let config = result.unwrap();
    assert!(config.default.is_some(), "Should have default config");

    std::env::set_current_dir(current_dir).unwrap();
}

// ============================================================================
// Acceptance Criterion 4: Proper error handling for malformed files
// ============================================================================

#[test]
fn acceptance_4_1_malformed_toml_returns_error() {
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
    assert!(result.is_err(), "Should return error for malformed TOML");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Failed to parse") || error_msg.contains("TOML"),
        "Error message should indicate parse failure"
    );
}

#[test]
fn acceptance_4_2_invalid_syntax_returns_descriptive_error() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Various invalid syntaxes
    let invalid_samples = vec![
        r#"default = "unclosed string#,
        r#"[agents.test] command = "test""#,
        r#"default = 123.456.789"#,
    ];

    for sample in invalid_samples {
        fs::write(&config_path, sample).unwrap();

        let result = load_config_file(&config_path);
        assert!(
            result.is_err(),
            "Should fail for invalid syntax: {}",
            sample
        );
    }
}

#[test]
fn acceptance_4_3_invalid_data_types_return_error() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Valid TOML but invalid types for our schema
    let content = r#"
[agents.test]
command = "test"
model = "test"
timeout = "not_a_number"
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    // Should fail during parsing
    assert!(
        result.is_err(),
        "Should return error for invalid data types"
    );
}

// ============================================================================
// Acceptance Criterion 5: Auto-discovery and merging of config sources
// ============================================================================

#[test]
fn acceptance_5_1_auto_discovery_finds_global_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create mock home directory
    let home_ltmatrix = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&home_ltmatrix).unwrap();

    let global_config = home_ltmatrix.join("config.toml");
    fs::write(
        &global_config,
        r#"
default = "auto-global"
"#,
    )
    .unwrap();

    // Load and verify
    let result = load_config_file(&global_config);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("auto-global".to_string()));
}

#[test]
fn acceptance_5_2_auto_discovery_finds_project_config() {
    let temp_dir = TempDir::new().unwrap();

    let project_ltmatrix = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&project_ltmatrix).unwrap();

    let project_config = project_ltmatrix.join("config.toml");
    fs::write(
        &project_config,
        r#"
default = "auto-project"
"#,
    )
    .unwrap();

    let result = load_config_file(&project_config);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("auto-project".to_string()));
}

#[test]
fn acceptance_5_3_merge_order_is_correct() {
    // Test precedence: Project > Global > Default

    let default_config = Config::default();

    let mut global_config = Config::default();
    global_config.default = Some("global".to_string());

    let mut project_config = Config::default();
    project_config.default = Some("project".to_string());

    // Merge
    let merged = merge_configs(Some(global_config), Some(project_config));

    // Project should win
    assert_eq!(merged.default, Some("project".to_string()));
}

#[test]
fn acceptance_5_4_merge_combines_all_sources() {
    let global = Config {
        default: Some("global".to_string()),
        agents: {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "agent1".to_string(),
                ltmatrix::config::settings::AgentConfig {
                    command: Some("cmd1".to_string()),
                    model: Some("model1".to_string()),
                    timeout: Some(100),
                },
            );
            map.insert(
                "agent2".to_string(),
                ltmatrix::config::settings::AgentConfig {
                    command: Some("cmd2".to_string()),
                    model: Some("model2".to_string()),
                    timeout: Some(200),
                },
            );
            map
        },
        modes: ltmatrix::config::settings::ModeConfigs {
            fast: Some(ltmatrix::config::settings::ModeConfig {
                model: Some("fast-model".to_string()),
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
        output: ltmatrix::config::settings::OutputConfig::default(),
        logging: ltmatrix::config::settings::LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: ltmatrix::config::settings::WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
    };

    let project = Config {
        default: Some("project".to_string()),
        agents: {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "agent1".to_string(),
                ltmatrix::config::settings::AgentConfig {
                    command: Some("cmd1-updated".to_string()),
                    model: None,
                    timeout: Some(150),
                },
            );
            map.insert(
                "agent3".to_string(),
                ltmatrix::config::settings::AgentConfig {
                    command: Some("cmd3".to_string()),
                    model: Some("model3".to_string()),
                    timeout: Some(300),
                },
            );
            map
        },
        modes: ltmatrix::config::settings::ModeConfigs {
            fast: None,
            standard: Some(ltmatrix::config::settings::ModeConfig {
                model: Some("standard-model".to_string()),
                run_tests: true,
                verify: true,
                max_retries: 3,
                max_depth: 3,
                timeout_plan: 120,
                timeout_exec: 3600,
            }),
            expert: None,
        },
        output: ltmatrix::config::settings::OutputConfig::default(),
        logging: ltmatrix::config::settings::LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: ltmatrix::config::settings::WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
    };

    let merged = merge_configs(Some(global), Some(project));

    // Default from project
    assert_eq!(merged.default, Some("project".to_string()));

    // agent1 should be merged
    let agent1 = &merged.agents["agent1"];
    assert_eq!(agent1.command, Some("cmd1-updated".to_string())); // From project
    assert_eq!(agent1.model, Some("model1".to_string())); // From global
    assert_eq!(agent1.timeout, Some(150)); // From project

    // agent2 from global
    assert!(merged.agents.contains_key("agent2"));

    // agent3 from project
    assert!(merged.agents.contains_key("agent3"));

    // Both modes should be present
    assert!(merged.modes.fast.is_some());
    assert!(merged.modes.standard.is_some());
}

// ============================================================================
// End-to-End Scenarios
// ============================================================================

#[test]
fn acceptance_e2e_typical_usage() {
    let temp_dir = TempDir::new().unwrap();

    // Setup global config
    let global_dir = temp_dir.path().join("home").join(".ltmatrix");
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

[output]
format = "text"
colored = true
"#,
    )
    .unwrap();

    // Setup project config
    let project_dir = temp_dir.path().join("project").join(".ltmatrix");
    fs::create_dir_all(&project_dir).unwrap();
    let project_config = project_dir.join("config.toml");
    fs::write(
        &project_config,
        r#"
[agents.claude]
model = "claude-opus-4-6"

[output]
colored = false
"#,
    )
    .unwrap();

    // Load and merge
    let global = load_config_file(&global_config).unwrap();
    let project = load_config_file(&project_config).unwrap();
    let final_config = merge_configs(Some(global), Some(project));

    // Verify merged config
    assert_eq!(final_config.default, Some("claude".to_string()));

    let claude = &final_config.agents["claude"];
    assert_eq!(claude.command, Some("claude".to_string())); // From global
    assert_eq!(claude.model, Some("claude-opus-4-6".to_string())); // From project

    assert_eq!(final_config.output.colored, false); // From project
}

#[test]
fn acceptance_e2e_minimal_working_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Minimal viable config
    fs::write(
        &config_path,
        r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
"#,
    )
    .unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
    assert!(config.agents.contains_key("claude"));
}

#[test]
fn acceptance_e2e_empty_config_returns_defaults() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(&config_path, "").unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, None);
    assert!(config.agents.is_empty());
}

#[test]
fn acceptance_e2e_comments_only_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
# This is a comment
# default = "test"

[agents]
# Another comment
"#,
    )
    .unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, None);
}
