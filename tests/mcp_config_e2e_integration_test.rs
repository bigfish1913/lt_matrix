//! End-to-End MCP Configuration Integration Tests
//!
//! These tests verify the complete integration flow:
//! CLI --mcp-config flag → load_config_from_args → Config struct with MCP config
//!
//! Task acceptance criteria verified:
//! ✓ --mcp-config argument added to CLI
//! ✓ Config file loading/parsing works
//! ✓ Server configurations are validated
//! ✓ MCP config merges with default settings (non-fatal)

use clap::Parser;
use ltmatrix::cli::Args;
use ltmatrix::config::settings::load_config_from_args;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// End-to-End Integration Tests
// ============================================================================

#[test]
fn test_e2e_mcp_config_loaded_from_cli() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    // Create a valid MCP config
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

    // Parse CLI args
    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    // Load config from args
    let config = load_config_from_args(args).unwrap();

    // Verify MCP config is loaded and accessible
    assert!(config.mcp.is_some(), "Config should have MCP config loaded");

    let mcp = config.mcp.unwrap();
    assert_eq!(mcp.path, mcp_config_path);
    assert_eq!(mcp.config.mcp.servers.len(), 2, "Should have 2 MCP servers");

    // Verify server configurations
    let playwright = mcp.config.get_server("playwright").unwrap();
    assert_eq!(playwright.server_type, "playwright");
    assert_eq!(playwright.command, Some("npx".to_string()));
    assert_eq!(playwright.args, vec!["-y", "@playwright/mcp@latest"]);
    assert_eq!(playwright.timeout, 60);

    let browser = mcp.config.get_server("browser").unwrap();
    assert_eq!(browser.server_type, "browser");
    assert_eq!(browser.timeout, 30);
}

#[test]
fn test_e2e_mcp_config_optional_flag() {
    // --mcp-config is optional, should work without it
    let args = Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();

    let config = load_config_from_args(args).unwrap();

    assert!(
        config.mcp.is_none(),
        "Config should have no MCP config when flag not provided"
    );
}

#[test]
fn test_e2e_invalid_mcp_config_non_fatal() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("invalid-mcp-config.toml");

    // Create an INVALID MCP config (zero timeout)
    let invalid_content = r#"
[mcp.servers.bad-server]
type = "test"
timeout = 0
"#;

    fs::write(&mcp_config_path, invalid_content).unwrap();

    // Parse CLI args
    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    // Load config from args - should NOT fail
    let config = load_config_from_args(args);

    assert!(
        config.is_ok(),
        "Invalid MCP config should not prevent config loading - should be non-fatal"
    );

    let config = config.unwrap();

    // MCP config should be None due to validation failure
    assert!(
        config.mcp.is_none(),
        "MCP config should be None when validation fails"
    );

    // But other config should still work
    assert!(
        config.default.is_some() || config.agents.contains_key("claude"),
        "Other config settings should still be available"
    );
}

#[test]
fn test_e2e_nonexistent_mcp_config_file_non_fatal() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_path = temp_dir.path().join("does-not-exist.toml");

    // Parse CLI args with non-existent file
    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        nonexistent_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    // Load config from args - should NOT fail
    let config = load_config_from_args(args);

    assert!(
        config.is_ok(),
        "Non-existent MCP config file should not prevent config loading"
    );

    let config = config.unwrap();
    assert!(
        config.mcp.is_none(),
        "MCP config should be None when file doesn't exist"
    );
}

#[test]
fn test_e2e_malformed_toml_non_fatal() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("malformed.toml");

    // Write malformed TOML
    fs::write(&mcp_config_path, "[invalid toml content").unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    // Should not fail
    let config = load_config_from_args(args).unwrap();
    assert!(
        config.mcp.is_none(),
        "MCP config should be None for malformed TOML"
    );
}

#[test]
fn test_e2e_mcp_config_with_other_cli_flags() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    let mcp_content = r#"
[mcp.servers.test]
type = "test"
command = "test-command"
"#;

    fs::write(&mcp_config_path, mcp_content).unwrap();

    // Use multiple CLI flags together
    let args = Args::try_parse_from([
        "ltmatrix",
        "--agent",
        "claude",
        "--mode",
        "fast",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "--log-level",
        "debug",
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();

    // Verify all configs are present
    assert!(config.mcp.is_some(), "MCP config should be loaded");
    assert_eq!(config.default, Some("claude".to_string()));
    assert_eq!(
        config.logging.level,
        ltmatrix::config::settings::LogLevel::Debug
    );

    // Verify MCP config integrity
    let mcp = config.mcp.unwrap();
    assert_eq!(mcp.config.mcp.servers.len(), 1);
    assert!(mcp.config.get_server("test").is_some());
}

#[test]
fn test_e2e_mcp_config_enabled_servers_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    let config_content = r#"
[mcp.servers.server1]
type = "type1"
enabled = true

[mcp.servers.server2]
type = "type2"
enabled = false

[mcp.servers.server3]
type = "type3"
"#;

    fs::write(&mcp_config_path, config_content).unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();
    let mcp = config.mcp.unwrap();

    // Verify enabled servers filtering works
    assert_eq!(mcp.config.mcp.servers.len(), 3);
    assert!(mcp.config.is_server_enabled("server1"));
    assert!(!mcp.config.is_server_enabled("server2"));
    assert!(mcp.config.is_server_enabled("server3"));

    let enabled = mcp.config.enabled_servers();
    assert_eq!(enabled.len(), 2);
    assert!(enabled.contains_key("server1"));
    assert!(enabled.contains_key("server3"));
}

#[test]
fn test_e2e_mcp_config_with_environment_variables() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    let config_content = r#"
[mcp.servers.server1]
type = "test"
command = "cmd"

[mcp.servers.server1.env]
API_KEY = "secret-key"
DEBUG = "true"
"#;

    fs::write(&mcp_config_path, config_content).unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();
    let mcp = config.mcp.unwrap();

    let server = mcp.config.get_server("server1").unwrap();
    assert_eq!(server.env.len(), 2);
    assert_eq!(server.env.get("API_KEY"), Some(&"secret-key".to_string()));
    assert_eq!(server.env.get("DEBUG"), Some(&"true".to_string()));
}

#[test]
fn test_e2e_mcp_config_with_working_directory() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");
    let work_dir = temp_dir.path();

    let cwd_str = work_dir.to_string_lossy().replace('\\', "/");
    let config_content = format!(
        r#"
[mcp.servers.server1]
type = "test"
command = "cmd"
cwd = "{}"
"#,
        cwd_str
    );

    fs::write(&mcp_config_path, config_content).unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();
    let mcp = config.mcp.unwrap();

    let server = mcp.config.get_server("server1").unwrap();
    assert_eq!(server.cwd, Some(work_dir.to_path_buf()));
}

#[test]
fn test_e2e_mcp_config_validation_working_directory_must_exist() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    // Use a non-existent directory
    let nonexistent = temp_dir.path().join("does-not-exist");
    let cwd_str = nonexistent.to_string_lossy().replace('\\', "/");

    let config_content = format!(
        r#"
[mcp.servers.server1]
type = "test"
command = "cmd"
cwd = "{}"
"#,
        cwd_str
    );

    fs::write(&mcp_config_path, config_content).unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    // Should fail validation (non-fatal)
    let config = load_config_from_args(args).unwrap();
    assert!(
        config.mcp.is_none(),
        "MCP config should be None when cwd doesn't exist"
    );
}

#[test]
fn test_e2e_mcp_config_custom_timeout_values() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    let config_content = r#"
[mcp.servers.fast_server]
type = "fast"
timeout = 10

[mcp.servers.slow_server]
type = "slow"
timeout = 300
"#;

    fs::write(&mcp_config_path, config_content).unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();
    let mcp = config.mcp.unwrap();

    let fast_server = mcp.config.get_server("fast_server").unwrap();
    assert_eq!(fast_server.timeout, 10);

    let slow_server = mcp.config.get_server("slow_server").unwrap();
    assert_eq!(slow_server.timeout, 300);
}

#[test]
fn test_e2e_mcp_config_default_values() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    // Minimal config - only type is required
    let config_content = r#"
[mcp.servers.minimal]
type = "minimal"
"#;

    fs::write(&mcp_config_path, config_content).unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();
    let mcp = config.mcp.unwrap();

    let server = mcp.config.get_server("minimal").unwrap();
    assert_eq!(server.server_type, "minimal");
    assert_eq!(server.command, None); // Optional
    assert!(server.args.is_empty()); // Default
    assert!(server.env.is_empty()); // Default
    assert_eq!(server.timeout, 60); // Default value
    assert!(server.enabled); // Default value
}

#[test]
fn test_e2e_mcp_config_reload_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.toml");

    let initial_content = r#"
[mcp.servers.server1]
type = "type1"
"#;

    fs::write(&mcp_config_path, initial_content).unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "test goal",
    ])
    .unwrap();

    let config = load_config_from_args(args).unwrap();
    let mcp = config.mcp.unwrap();
    assert_eq!(mcp.config.mcp.servers.len(), 1);

    // Update the file
    let updated_content = r#"
[mcp.servers.server1]
type = "type1"

[mcp.servers.server2]
type = "type2"
"#;

    fs::write(&mcp_config_path, updated_content).unwrap();

    // Reload
    let reloaded = mcp.reload().unwrap();
    assert_eq!(reloaded.config.mcp.servers.len(), 2);
}
