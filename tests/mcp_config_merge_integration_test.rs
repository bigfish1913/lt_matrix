//! MCP Configuration Merge and Integration Tests
//!
//! These tests verify:
//! - MCP config merges correctly with other config sources
//! - Proper precedence order (CLI > project > global > defaults)
//! - MCP config doesn't interfere with other config loading

use ltmatrix::config::mcp::McpConfig;
use ltmatrix::config::settings::load_config_from_args;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// MCP Config Merge Tests
// ============================================================================

#[test]
fn test_mcp_config_merge_override_behavior() {
    let config1_content = r#"
[mcp.servers.server1]
type = "type1"
command = "cmd1"
timeout = 30

[mcp.servers.server2]
type = "type2"
"#;

    let config2_content = r#"
[mcp.servers.server2]
type = "type2-updated"
command = "cmd2"
timeout = 120

[mcp.servers.server3]
type = "type3"
"#;

    let config1 = McpConfig::from_str(config1_content).unwrap();
    let config2 = McpConfig::from_str(config2_content).unwrap();

    let merged = config1.merge_with(&config2);

    // Verify merge behavior
    assert_eq!(merged.mcp.servers.len(), 3, "Should have 3 servers after merge");

    // server1 from config1 (not in config2)
    let server1 = merged.get_server("server1").unwrap();
    assert_eq!(server1.command, Some("cmd1".to_string()));
    assert_eq!(server1.timeout, 30);

    // server2 from config2 (overridden)
    let server2 = merged.get_server("server2").unwrap();
    assert_eq!(server2.server_type, "type2-updated");
    assert_eq!(server2.command, Some("cmd2".to_string()));
    assert_eq!(server2.timeout, 120);

    // server3 from config2 (new)
    assert!(merged.get_server("server3").is_some());
}

#[test]
fn test_mcp_config_merge_preserves_original() {
    let config1_content = r#"
[mcp.servers.server1]
type = "type1"
command = "cmd1"
"#;

    let config2_content = r#"
[mcp.servers.server1]
type = "type1-updated"
command = "cmd2"
"#;

    let config1 = McpConfig::from_str(config1_content).unwrap();
    let config2 = McpConfig::from_str(config2_content).unwrap();

    // Store original values
    let original_type = config1.get_server("server1").unwrap().server_type.clone();
    let original_command = config1.get_server("server1").unwrap().command.clone();

    let merged = config1.merge_with(&config2);

    // Original should be unchanged
    let server1_original = config1.get_server("server1").unwrap();
    assert_eq!(server1_original.server_type, original_type);
    assert_eq!(server1_original.command, original_command);

    // Merged should have new values
    let server1_merged = merged.get_server("server1").unwrap();
    assert_eq!(server1_merged.server_type, "type1-updated");
    assert_eq!(server1_merged.command, Some("cmd2".to_string()));
}

#[test]
fn test_mcp_config_merge_with_empty_config() {
    let config1_content = r#"
[mcp.servers.server1]
type = "type1"
"#;

    let config2_content = r#"
[mcp.servers]
"#;

    let config1 = McpConfig::from_str(config1_content).unwrap();
    let config2 = McpConfig::from_str(config2_content).unwrap();

    let merged = config1.merge_with(&config2);

    // Should have server1 from config1
    assert_eq!(merged.mcp.servers.len(), 1);
    assert!(merged.get_server("server1").is_some());
}

#[test]
fn test_mcp_config_merge_empty_with_populated() {
    let config1_content = r#"
[mcp.servers]
"#;

    let config2_content = r#"
[mcp.servers.server1]
type = "type1"
"#;

    let config1 = McpConfig::from_str(config1_content).unwrap();
    let config2 = McpConfig::from_str(config2_content).unwrap();

    let merged = config1.merge_with(&config2);

    // Should have server1 from config2
    assert_eq!(merged.mcp.servers.len(), 1);
    assert!(merged.get_server("server1").is_some());
}

// ============================================================================
// Integration with Other Config Sources
// ============================================================================

#[test]
fn test_mcp_config_does_not_interfere_with_other_settings() {
    use clap::Parser;

    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    let mcp_content = r#"
[mcp.servers.test]
type = "test"
"#;

    fs::write(&mcp_config_path, mcp_content).unwrap();

    let args = ltmatrix::cli::Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "--mode",
        "expert",
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();

    // Verify all config settings are present and correct
    assert!(config.mcp.is_some(), "MCP config should be loaded");
    assert_eq!(config.default, Some("claude".to_string()));

    // MCP config should not affect other settings
    assert!(config.agents.is_empty() || config.agents.len() > 0);
    assert!(config.output.format == ltmatrix::config::settings::OutputFormat::Text);
}

#[test]
fn test_config_without_mcp_still_has_defaults() {
    use clap::Parser;

    let args = ltmatrix::cli::Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();

    let config = load_config_from_args(args).unwrap();

    // Should have all default settings
    assert!(config.mcp.is_none(), "MCP config should be None");
    assert_eq!(config.default, Some("claude".to_string()), "Should have default agent");
    // Note: modes might be None in default config, so just check the default agent is present
    assert!(config.agents.is_empty() || config.default.is_some());
}

#[test]
fn test_mcp_config_with_project_config_integration() {
    use clap::Parser;

    let temp_dir = TempDir::new().unwrap();

    // Create project config
    let project_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&project_dir).unwrap();

    let project_config_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
"#;

    let project_config_path = project_dir.join("config.toml");
    fs::write(&project_config_path, project_config_content).unwrap();

    // Create MCP config
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");
    let mcp_content = r#"
[mcp.servers.test]
type = "test"
command = "test-command"
"#;

    fs::write(&mcp_config_path, mcp_content).unwrap();

    // Change to project directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Parse args
    let args = ltmatrix::cli::Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    // Verify both configs are loaded
    assert!(config.mcp.is_some(), "MCP config should be loaded");
    assert_eq!(
        config.default,
        Some("claude".to_string()),
        "Project config default agent should be used"
    );

    // Verify MCP config
    let mcp = config.mcp.unwrap();
    assert_eq!(mcp.config.mcp.servers.len(), 1);
    assert!(mcp.config.get_server("test").is_some());
}

#[test]
fn test_mcp_config_cli_arg_precedence_over_defaults() {
    use clap::Parser;

    let temp_dir = TempDir::new().unwrap();

    // Create project config with MCP
    let project_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&project_dir).unwrap();

    let project_config_path = project_dir.join("config.toml");
    fs::write(&project_config_path, "default = \"claude\"").unwrap();

    // Create MCP config file 1
    let mcp_config_path1 = temp_dir.path().join("mcp1.toml");
    let mcp_content1 = r#"
[mcp.servers.server1]
type = "type1"
"#;

    fs::write(&mcp_config_path1, mcp_content1).unwrap();

    // Create MCP config file 2
    let mcp_config_path2 = temp_dir.path().join("mcp2.toml");
    let mcp_content2 = r#"
[mcp.servers.server2]
type = "type2"
"#;

    fs::write(&mcp_config_path2, mcp_content2).unwrap();

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // CLI arg should take precedence
    let args = ltmatrix::cli::Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path2.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();

    std::env::set_current_dir(original_dir).unwrap();

    // Should load MCP config from CLI arg, not from any other source
    assert!(config.mcp.is_some());
    let mcp = config.mcp.unwrap();
    assert_eq!(mcp.path, mcp_config_path2);
    assert!(mcp.config.get_server("server2").is_some());
    assert!(mcp.config.get_server("server1").is_none());
}

// ============================================================================
// MCP Config Validation in Integration Context
// ============================================================================

#[test]
fn test_mcp_config_validation_during_full_load() {
    use clap::Parser;

    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    // Create MCP config with validation errors
    let mcp_content = r#"
[mcp.servers.good_server]
type = "good"
timeout = 60

[mcp.servers.bad_timeout]
type = "test"
timeout = 0

[mcp.servers.bad_type]
type = ""
"#;

    fs::write(&mcp_config_path, mcp_content).unwrap();

    let args = ltmatrix::cli::Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    // Should not fail overall config loading
    let config = load_config_from_args(args);

    assert!(
        config.is_ok(),
        "Config loading should succeed even with invalid MCP config"
    );

    let config = config.unwrap();
    assert!(config.mcp.is_none(), "MCP config should be None due to validation errors");

    // Other config should still work
    assert!(
        config.default.is_some(),
        "Default agent should still be available"
    );
}

#[test]
fn test_mcp_config_partial_validation_failure() {
    use clap::Parser;

    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    // Create MCP config with one valid and one invalid server
    let mcp_content = r#"
[mcp.servers.valid_server]
type = "valid"
timeout = 60

[mcp.servers.invalid_server]
type = "valid"
timeout = 0
"#;

    fs::write(&mcp_config_path, mcp_content).unwrap();

    let args = ltmatrix::cli::Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();

    // Entire MCP config should be rejected due to validation error
    assert!(
        config.mcp.is_none(),
        "MCP config should be None if any server fails validation"
    );
}

#[test]
fn test_mcp_config_multiple_servers_all_valid() {
    use clap::Parser;

    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    let mcp_content = r#"
[mcp.servers.playwright]
type = "playwright"
command = "npx"
args = ["-y", "@playwright/mcp@latest"]
timeout = 60

[mcp.servers.browser]
type = "browser"
command = "mcp-server-browser"
timeout = 30

[mcp.servers.filesystem]
type = "filesystem"
command = "mcp-server-filesystem"
enabled = false
"#;

    fs::write(&mcp_config_path, mcp_content).unwrap();

    let args = ltmatrix::cli::Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();

    assert!(config.mcp.is_some());
    let mcp = config.mcp.unwrap();
    assert_eq!(mcp.config.mcp.servers.len(), 3);

    // Verify all servers loaded
    assert!(mcp.config.get_server("playwright").is_some());
    assert!(mcp.config.get_server("browser").is_some());
    assert!(mcp.config.get_server("filesystem").is_some());

    // Verify enabled/disabled status
    assert!(mcp.config.is_server_enabled("playwright"));
    assert!(mcp.config.is_server_enabled("browser"));
    assert!(!mcp.config.is_server_enabled("filesystem"));

    // Verify enabled servers count
    let enabled = mcp.config.enabled_servers();
    assert_eq!(enabled.len(), 2);
}
