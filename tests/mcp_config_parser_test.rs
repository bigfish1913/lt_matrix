//! MCP Configuration Parser Tests
//!
//! These tests verify:
//! - TOML parsing edge cases
//! - Validation error messages
//! - Default value application
//! - Serialization/deserialization roundtrip
//! - Edge cases in server configuration

use ltmatrix::config::mcp::{LoadedMcpConfig, McpConfig, McpServers};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// TOML Parsing Edge Cases
// ============================================================================

#[test]
fn test_empty_mcp_config_parses_successfully() {
    let config_content = r#"
[mcp.servers]
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    assert_eq!(config.mcp.servers.len(), 0);
    assert!(config.enabled_servers().is_empty());
}

#[test]
fn test_mcp_config_with_whitespace_only_type_fails() {
    let config_content = r#"
[mcp.servers.whitespace_server]
type = "   "
"#;

    let result = McpConfig::from_str(config_content);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("empty type"),
        "Error should mention empty type, got: {}",
        error
    );
}

#[test]
fn test_mcp_config_with_whitespace_only_command_fails() {
    let config_content = r#"
[mcp.servers.whitespace_command]
type = "test"
command = "   "
"#;

    let result = McpConfig::from_str(config_content);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("empty command"),
        "Error should mention empty command, got: {}",
        error
    );
}

#[test]
fn test_mcp_config_with_unicode_server_name() {
    let config_content = r#"
[mcp.servers."测试服务器"]
type = "test"
command = "test-cmd"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    assert!(config.get_server("测试服务器").is_some());
    let server = config.get_server("测试服务器").unwrap();
    assert_eq!(server.server_type, "test");
}

#[test]
fn test_mcp_config_with_unicode_type() {
    let config_content = r#"
[mcp.servers.test]
type = "测试类型"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert_eq!(server.server_type, "测试类型");
}

#[test]
fn test_mcp_config_with_special_characters_in_args() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
args = ["--flag=value with spaces", "--unicode=日本語", "--special=$HOME"]
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert_eq!(server.args.len(), 3);
    assert_eq!(server.args[0], "--flag=value with spaces");
    assert_eq!(server.args[1], "--unicode=日本語");
    assert_eq!(server.args[2], "--special=$HOME");
}

#[test]
fn test_mcp_config_with_empty_args_list() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
args = []
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert!(server.args.is_empty());
}

#[test]
fn test_mcp_config_with_many_args() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
args = ["arg1", "arg2", "arg3", "arg4", "arg5", "arg6", "arg7", "arg8", "arg9", "arg10"]
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert_eq!(server.args.len(), 10);
}

// ============================================================================
// Validation Error Messages
// ============================================================================

#[test]
fn test_validation_error_includes_server_name() {
    let config_content = r#"
[mcp.servers.my_custom_server_name]
type = ""
"#;

    let result = McpConfig::from_str(config_content);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("my_custom_server_name"),
        "Error should include server name, got: {}",
        error
    );
}

#[test]
fn test_validation_error_for_zero_timeout_includes_server_name() {
    let config_content = r#"
[mcp.servers.timeout_server]
type = "test"
timeout = 0
"#;

    let result = McpConfig::from_str(config_content);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("timeout_server"),
        "Error should include server name, got: {}",
        error
    );
    assert!(
        error.contains("zero timeout"),
        "Error should mention zero timeout, got: {}",
        error
    );
}

#[test]
fn test_validation_error_for_nonexistent_cwd_includes_path() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent-dir-12345");
    let cwd_str = nonexistent.to_string_lossy().replace('\\', "/");

    let config_content = format!(
        r#"
[mcp.servers.test]
type = "test"
cwd = "{}"
"#,
        cwd_str
    );

    let result = McpConfig::from_str(&config_content);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("working directory does not exist"),
        "Error should mention working directory does not exist, got: {}",
        error
    );
}

// ============================================================================
// Default Values
// ============================================================================

#[test]
fn test_mcp_server_default_timeout() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert_eq!(server.timeout, 60, "Default timeout should be 60 seconds");
}

#[test]
fn test_mcp_server_default_enabled() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert!(server.enabled, "Default enabled should be true");
}

#[test]
fn test_mcp_server_default_args() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert!(server.args.is_empty(), "Default args should be empty");
}

#[test]
fn test_mcp_server_default_env() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert!(server.env.is_empty(), "Default env should be empty");
}

#[test]
fn test_mcp_server_default_command() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert!(server.command.is_none(), "Default command should be None");
}

#[test]
fn test_mcp_server_default_cwd() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert!(server.cwd.is_none(), "Default cwd should be None");
}

// ============================================================================
// Serialization/Deserialization Roundtrip
// ============================================================================

#[test]
fn test_mcp_config_roundtrip_simple() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
command = "test-cmd"
timeout = 30
enabled = false
"#;

    let config = McpConfig::from_str(config_content).unwrap();

    // Serialize back to TOML
    let serialized = toml::to_string(&config).unwrap();

    // Parse again
    let reparsed = McpConfig::from_str(&serialized).unwrap();

    // Verify they're equivalent
    assert_eq!(config.mcp.servers.len(), reparsed.mcp.servers.len());
    let original_server = config.get_server("test").unwrap();
    let reparsed_server = reparsed.get_server("test").unwrap();
    assert_eq!(original_server.server_type, reparsed_server.server_type);
    assert_eq!(original_server.command, reparsed_server.command);
    assert_eq!(original_server.timeout, reparsed_server.timeout);
    assert_eq!(original_server.enabled, reparsed_server.enabled);
}

#[test]
fn test_mcp_config_roundtrip_with_env() {
    let config_content = r#"
[mcp.servers.test]
type = "test"

[mcp.servers.test.env]
KEY1 = "value1"
KEY2 = "value2"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let serialized = toml::to_string(&config).unwrap();
    let reparsed = McpConfig::from_str(&serialized).unwrap();

    let original_server = config.get_server("test").unwrap();
    let reparsed_server = reparsed.get_server("test").unwrap();

    assert_eq!(original_server.env.len(), reparsed_server.env.len());
    assert_eq!(
        original_server.env.get("KEY1"),
        reparsed_server.env.get("KEY1")
    );
    assert_eq!(
        original_server.env.get("KEY2"),
        reparsed_server.env.get("KEY2")
    );
}

// ============================================================================
// Enabled Servers
// ============================================================================

#[test]
fn test_is_server_enabled_returns_false_for_nonexistent() {
    let config_content = r#"
[mcp.servers.existing]
type = "test"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    assert!(
        !config.is_server_enabled("nonexistent"),
        "is_server_enabled should return false for nonexistent server"
    );
}

#[test]
fn test_is_server_enabled_returns_true_when_enabled() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
enabled = true
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    assert!(
        config.is_server_enabled("test"),
        "is_server_enabled should return true when explicitly enabled"
    );
}

#[test]
fn test_is_server_enabled_returns_false_when_disabled() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
enabled = false
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    assert!(
        !config.is_server_enabled("test"),
        "is_server_enabled should return false when explicitly disabled"
    );
}

#[test]
fn test_enabled_servers_excludes_disabled() {
    let config_content = r#"
[mcp.servers.enabled1]
type = "test1"
enabled = true

[mcp.servers.enabled2]
type = "test2"

[mcp.servers.disabled1]
type = "test3"
enabled = false

[mcp.servers.disabled2]
type = "test4"
enabled = false
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let enabled = config.enabled_servers();

    assert_eq!(enabled.len(), 2);
    assert!(enabled.contains_key("enabled1"));
    assert!(enabled.contains_key("enabled2"));
    assert!(!enabled.contains_key("disabled1"));
    assert!(!enabled.contains_key("disabled2"));
}

// ============================================================================
// Get Server
// ============================================================================

#[test]
fn test_get_server_returns_none_for_nonexistent() {
    let config_content = r#"
[mcp.servers.existing]
type = "test"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    assert!(
        config.get_server("nonexistent").is_none(),
        "get_server should return None for nonexistent server"
    );
}

#[test]
fn test_get_server_returns_reference() {
    let config_content = r#"
[mcp.servers.test]
type = "test-type"
command = "test-command"
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();
    assert_eq!(server.server_type, "test-type");
    assert_eq!(server.command, Some("test-command".to_string()));
}

// ============================================================================
// File Loading
// ============================================================================

#[test]
fn test_load_mcp_config_from_file_success() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mcp-config.toml");

    let config_content = r#"
[mcp.servers.test]
type = "test"
command = "test-cmd"
"#;

    fs::write(&config_path, config_content).unwrap();

    let loaded = LoadedMcpConfig::from_file(&config_path).unwrap();
    assert_eq!(loaded.path, config_path);
    assert_eq!(loaded.config.mcp.servers.len(), 1);
    assert!(loaded.config.get_server("test").is_some());
}

#[test]
fn test_load_mcp_config_nonexistent_file_error() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent.toml");

    let result = LoadedMcpConfig::from_file(&nonexistent);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("Failed to read"),
        "Error should mention failed read, got: {}",
        error
    );
}

#[test]
fn test_load_mcp_config_invalid_toml_error() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");

    fs::write(&config_path, "[invalid toml[").unwrap();

    let result = LoadedMcpConfig::from_file(&config_path);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("parse") || error.contains("TOML"),
        "Error should mention parse failure, got: {}",
        error
    );
}

// ============================================================================
// Reload Functionality
// ============================================================================

#[test]
fn test_reload_picks_up_changes() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mcp-config.toml");

    let initial_content = r#"
[mcp.servers.server1]
type = "type1"
"#;

    fs::write(&config_path, initial_content).unwrap();

    let loaded = LoadedMcpConfig::from_file(&config_path).unwrap();
    assert_eq!(loaded.config.mcp.servers.len(), 1);

    // Update the file
    let updated_content = r#"
[mcp.servers.server1]
type = "type1"

[mcp.servers.server2]
type = "type2"

[mcp.servers.server3]
type = "type3"
"#;

    fs::write(&config_path, updated_content).unwrap();

    let reloaded = loaded.reload().unwrap();
    assert_eq!(reloaded.config.mcp.servers.len(), 3);
    assert!(reloaded.config.get_server("server1").is_some());
    assert!(reloaded.config.get_server("server2").is_some());
    assert!(reloaded.config.get_server("server3").is_some());
}

// ============================================================================
// Merge Functionality
// ============================================================================

#[test]
fn test_merge_empty_with_empty() {
    let config1 = McpConfig::from_str("[mcp.servers]").unwrap();
    let config2 = McpConfig::from_str("[mcp.servers]").unwrap();

    let merged = config1.merge_with(&config2);
    assert_eq!(merged.mcp.servers.len(), 0);
}

#[test]
fn test_merge_preserves_all_from_both() {
    let config1_content = r#"
[mcp.servers.server_a]
type = "type_a"

[mcp.servers.server_b]
type = "type_b"
"#;

    let config2_content = r#"
[mcp.servers.server_c]
type = "type_c"

[mcp.servers.server_d]
type = "type_d"
"#;

    let config1 = McpConfig::from_str(config1_content).unwrap();
    let config2 = McpConfig::from_str(config2_content).unwrap();

    let merged = config1.merge_with(&config2);

    assert_eq!(merged.mcp.servers.len(), 4);
    assert!(merged.get_server("server_a").is_some());
    assert!(merged.get_server("server_b").is_some());
    assert!(merged.get_server("server_c").is_some());
    assert!(merged.get_server("server_d").is_some());
}

#[test]
fn test_merge_completely_replaces_conflicting_servers() {
    let config1_content = r#"
[mcp.servers.shared]
type = "original_type"
command = "original_cmd"
timeout = 30
enabled = false

[mcp.servers.only_in_first]
type = "first_type"
"#;

    let config2_content = r#"
[mcp.servers.shared]
type = "replacement_type"
command = "replacement_cmd"
timeout = 120
enabled = true

[mcp.servers.only_in_second]
type = "second_type"
"#;

    let config1 = McpConfig::from_str(config1_content).unwrap();
    let config2 = McpConfig::from_str(config2_content).unwrap();

    let merged = config1.merge_with(&config2);

    // The shared server should be completely replaced by config2's version
    let shared = merged.get_server("shared").unwrap();
    assert_eq!(shared.server_type, "replacement_type");
    assert_eq!(shared.command, Some("replacement_cmd".to_string()));
    assert_eq!(shared.timeout, 120);
    assert!(shared.enabled);

    // Both unique servers should be present
    assert!(merged.get_server("only_in_first").is_some());
    assert!(merged.get_server("only_in_second").is_some());
}

// ============================================================================
// Timeout Validation
// ============================================================================

#[test]
fn test_timeout_of_one_is_valid() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
timeout = 1
"#;

    let result = McpConfig::from_str(config_content);
    assert!(
        result.is_ok(),
        "Timeout of 1 should be valid (minimum non-zero)"
    );
}

#[test]
fn test_very_large_timeout_is_valid() {
    let config_content = r#"
[mcp.servers.test]
type = "test"
timeout = 86400
"#;

    let result = McpConfig::from_str(config_content);
    assert!(result.is_ok(), "Very large timeout should be valid");

    let config = result.unwrap();
    let server = config.get_server("test").unwrap();
    assert_eq!(server.timeout, 86400);
}

// ============================================================================
// Environment Variables
// ============================================================================

#[test]
fn test_env_variables_with_special_values() {
    let config_content = r#"
[mcp.servers.test]
type = "test"

[mcp.servers.test.env]
EMPTY = ""
WITH_SPACES = "value with spaces"
WITH_EQUALS = "key=value"
WITH_QUOTES = "has \"quotes\" inside"
JSON_VALUE = '{"key": "value"}'
"#;

    let config = McpConfig::from_str(config_content).unwrap();
    let server = config.get_server("test").unwrap();

    assert_eq!(server.env.get("EMPTY"), Some(&"".to_string()));
    assert_eq!(
        server.env.get("WITH_SPACES"),
        Some(&"value with spaces".to_string())
    );
    assert_eq!(
        server.env.get("WITH_EQUALS"),
        Some(&"key=value".to_string())
    );
    assert_eq!(
        server.env.get("WITH_QUOTES"),
        Some(&"has \"quotes\" inside".to_string())
    );
    assert_eq!(
        server.env.get("JSON_VALUE"),
        Some(&"{\"key\": \"value\"}".to_string())
    );
}

#[test]
fn test_env_variables_with_many_entries() {
    let mut config_content = String::from(
        r#"
[mcp.servers.test]
type = "test"

[mcp.servers.test.env]
"#,
    );

    for i in 0..50 {
        config_content.push_str(&format!("KEY_{} = \"value_{}\"\n", i, i));
    }

    let config = McpConfig::from_str(&config_content).unwrap();
    let server = config.get_server("test").unwrap();

    assert_eq!(server.env.len(), 50);
    for i in 0..50 {
        assert_eq!(
            server.env.get(&format!("KEY_{}", i)),
            Some(&format!("value_{}", i))
        );
    }
}

// ============================================================================
// Default Implementation
// ============================================================================

#[test]
fn test_mcp_config_default() {
    let config = McpConfig::default();
    assert!(config.mcp.servers.is_empty());
}

#[test]
fn test_mcp_servers_default() {
    let servers = McpServers::default();
    assert!(servers.servers.is_empty());
}
