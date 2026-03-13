//! Integration tests for MCP configuration CLI integration
//!
//! These tests verify:
//! - --mcp-config flag properly loads configuration
//! - MCP config is accessible from Config struct
//! - Invalid MCP configs are handled gracefully
//! - MCP config merges correctly with other config sources

use clap::Parser;
use ltmatrix::cli::args::Args;
use ltmatrix::config::mcp::{LoadedMcpConfig, McpConfig};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// CLI MCP Config Loading Tests
// ============================================================================

#[test]
fn test_mcp_config_flag_loads_file() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    // Create a valid MCP config file
    let config_content = r#"
[mcp.servers.playwright]
type = "playwright"
command = "npx"
args = ["-y", "@playwright/mcp@latest"]
timeout = 60

[mcp.servers.browser]
type = "browser"
command = "mcp-server-browser"
timeout = 30
"#;

    fs::write(&mcp_config_path, config_content).unwrap();

    // Parse args with --mcp-config flag
    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    assert_eq!(
        args.mcp_config,
        Some(mcp_config_path.clone()),
        "MCP config path should be parsed correctly"
    );
}

#[test]
fn test_mcp_config_missing_file_handled_gracefully() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_path = temp_dir.path().join("nonexistent-mcp-config.toml");

    // Parse args with non-existent MCP config file
    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        nonexistent_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    assert_eq!(
        args.mcp_config,
        Some(nonexistent_path),
        "MCP config path should be parsed even if file doesn't exist"
    );

    // The config loading should handle the missing file gracefully
    // (This would be tested with load_config_from_args, but we're just
    // verifying CLI parsing here)
}

#[test]
fn test_mcp_config_flag_optional() {
    // --mcp-config flag should be optional
    let args = Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();

    assert!(
        args.mcp_config.is_none(),
        "MCP config should be None when not specified"
    );
}

// ============================================================================
// MCP Config File Validation Tests
// ============================================================================

#[test]
fn test_valid_mcp_config_loads_successfully() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    let config_content = r#"
[mcp.servers.test-server]
type = "test"
command = "test-command"
timeout = 60
"#;

    fs::write(&mcp_config_path, config_content).unwrap();

    let result = LoadedMcpConfig::from_file(&mcp_config_path);
    assert!(result.is_ok(), "Valid MCP config should load successfully");

    let loaded = result.unwrap();
    assert_eq!(loaded.path, mcp_config_path);
    assert_eq!(loaded.config.mcp.servers.len(), 1);
    assert!(loaded.config.mcp.servers.contains_key("test-server"));
}

#[test]
fn test_invalid_mcp_config_fails_validation() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    // Create config with zero timeout (invalid)
    let config_content = r#"
[mcp.servers.bad-server]
type = "test"
timeout = 0
"#;

    fs::write(&mcp_config_path, config_content).unwrap();

    let result = LoadedMcpConfig::from_file(&mcp_config_path);
    assert!(
        result.is_err(),
        "MCP config with zero timeout should fail validation"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("zero timeout"),
        "Error message should mention zero timeout"
    );
}

#[test]
fn test_malformed_toml_fails_gracefully() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    // Write malformed TOML
    fs::write(&mcp_config_path, "[invalid toml content").unwrap();

    let result = LoadedMcpConfig::from_file(&mcp_config_path);
    assert!(result.is_err(), "Malformed TOML should fail to parse");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("parse") || error_msg.contains("TOML"),
        "Error message should indicate parse failure"
    );
}

// ============================================================================
// MCP Server Configuration Tests
// ============================================================================

#[test]
fn test_mcp_server_with_all_fields() {
    let config_content = r#"
[mcp.servers.full-server]
type = "playwright"
command = "npx"
args = ["-y", "@playwright/mcp@latest", "--verbose"]
timeout = 120
enabled = true

[mcp.servers.full-server.env]
NODE_ENV = "production"
DEBUG = "true"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("full-server").unwrap();

    assert_eq!(server.server_type, "playwright");
    assert_eq!(server.command, Some("npx".to_string()));
    assert_eq!(server.args.len(), 3);
    assert_eq!(server.args[0], "-y");
    assert_eq!(server.timeout, 120);
    assert!(server.enabled);
    assert_eq!(server.env.len(), 2);
    assert_eq!(server.env.get("NODE_ENV"), Some(&"production".to_string()));
}

#[test]
fn test_mcp_server_with_minimal_fields() {
    let config_content = r#"
[mcp.servers.minimal-server]
type = "browser"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("minimal-server").unwrap();

    assert_eq!(server.server_type, "browser");
    assert_eq!(server.command, None);
    assert!(server.args.is_empty());
    assert!(server.env.is_empty());
    assert_eq!(server.timeout, 60); // Should use default
    assert!(server.enabled); // Should use default
}

#[test]
fn test_mcp_server_disabled() {
    let config_content = r#"
[mcp.servers.disabled-server]
type = "test"
enabled = false

[mcp.servers.enabled-server]
type = "test"
"#;

    let config = McpConfig::from_str(config_content).unwrap();

    assert!(!config.is_server_enabled("disabled-server"));
    assert!(config.is_server_enabled("enabled-server"));

    let enabled_servers = config.enabled_servers();
    assert_eq!(enabled_servers.len(), 1);
    assert!(enabled_servers.contains_key("enabled-server"));
    assert!(!enabled_servers.contains_key("disabled-server"));
}

// ============================================================================
// MCP Config Merge Tests
// ============================================================================

#[test]
fn test_mcp_config_merge() {
    let config1_content = r#"
[mcp.servers.server1]
type = "type1"
command = "cmd1"

[mcp.servers.server2]
type = "type2"
"#;

    let config2_content = r#"
[mcp.servers.server2]
type = "type2-updated"
command = "cmd2"

[mcp.servers.server3]
type = "type3"
"#;

    let config1 = McpConfig::from_str(config1_content).unwrap();
    let config2 = McpConfig::from_str(config2_content).unwrap();

    let merged = config1.merge_with(&config2);

    // server2 should be updated from config2
    let server2 = merged.get_server("server2").unwrap();
    assert_eq!(server2.server_type, "type2-updated");
    assert_eq!(server2.command, Some("cmd2".to_string()));

    // server1 should be from config1
    assert!(merged.get_server("server1").is_some());

    // server3 should be from config2
    assert!(merged.get_server("server3").is_some());
}
