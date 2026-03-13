//! Comprehensive edge case and validation failure tests for warmup configuration
//!
//! These tests verify:
//! - Validation failure scenarios
//! - Edge cases for warmup configuration values
//! - Error handling for invalid inputs
//! - Boundary conditions

use ltmatrix::config::settings::{Config, WarmupConfig};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Validation Failure Tests
// ============================================================================

#[test]
fn warmup_validate_fails_with_zero_max_queries() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 0, // Invalid: must be > 0
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "Validation should fail for max_queries = 0"
    );
    assert!(result
        .unwrap_err()
        .contains("max_queries must be greater than 0"));
}

#[test]
fn warmup_validate_fails_with_zero_timeout() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 3,
        timeout_seconds: 0, // Invalid: must be > 0
        retry_on_failure: false,
        prompt_template: None,
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "Validation should fail for timeout_seconds = 0"
    );
    assert!(result
        .unwrap_err()
        .contains("timeout_seconds must be greater than 0"));
}

#[test]
fn warmup_validate_fails_with_empty_prompt_template() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 3,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: Some("   ".to_string()), // Only whitespace
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "Validation should fail for empty/whitespace prompt_template"
    );
    assert!(result
        .unwrap_err()
        .contains("prompt_template cannot be empty"));
}

#[test]
fn warmup_validate_succeeds_with_none_prompt_template() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 3,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None, // None is valid
    };

    let result = config.validate();
    assert!(
        result.is_ok(),
        "Validation should succeed when prompt_template is None"
    );
}

#[test]
fn warmup_validate_succeeds_with_valid_prompt_template() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 3,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: Some("Valid prompt template".to_string()),
    };

    let result = config.validate();
    assert!(
        result.is_ok(),
        "Validation should succeed for valid prompt_template"
    );
}

// ============================================================================
// Boundary Value Tests
// ============================================================================

#[test]
fn warmup_minimum_valid_values() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1,     // Minimum valid value
        timeout_seconds: 1, // Minimum valid value
        retry_on_failure: false,
        prompt_template: Some("x".to_string()), // Shortest non-empty string
    };

    assert!(
        config.validate().is_ok(),
        "Minimum boundary values should be valid"
    );
}

#[test]
fn warmup_maximum_reasonable_values() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 1000,      // High but valid
        timeout_seconds: 86400, // 24 hours
        retry_on_failure: true,
        prompt_template: Some("A".repeat(10000)), // Very long prompt
    };

    assert!(
        config.validate().is_ok(),
        "High boundary values should be valid"
    );
}

#[test]
fn warmup_config_with_all_disabled() {
    let config = WarmupConfig {
        enabled: false,
        max_queries: 3,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    };

    assert!(
        config.validate().is_ok(),
        "Disabled warmup config should still validate"
    );
    assert_eq!(config.enabled, false);
}

#[test]
fn warmup_config_with_all_enabled() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 5,
        timeout_seconds: 60,
        retry_on_failure: true,
        prompt_template: Some("Custom prompt".to_string()),
    };

    assert!(
        config.validate().is_ok(),
        "All features enabled should be valid"
    );
    assert_eq!(config.enabled, true);
    assert_eq!(config.retry_on_failure, true);
    assert!(config.prompt_template.is_some());
}

// ============================================================================
// TOML Parsing Edge Cases
// ============================================================================

#[test]
fn warmup_parse_toml_with_missing_optional_fields() {
    let toml_str = r#"
        [warmup]
        enabled = true
        max_queries = 3
        timeout_seconds = 30
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.warmup.enabled, true);
    assert_eq!(config.warmup.max_queries, 3);
    assert_eq!(config.warmup.timeout_seconds, 30);
    assert_eq!(config.warmup.retry_on_failure, false); // Default
    assert_eq!(config.warmup.prompt_template, None); // Default
}

#[test]
fn warmup_parse_toml_with_all_fields() {
    let toml_str = r#"
        [warmup]
        enabled = true
        max_queries = 10
        timeout_seconds = 120
        retry_on_failure = true
        prompt_template = "Hello from warmup"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.warmup.enabled, true);
    assert_eq!(config.warmup.max_queries, 10);
    assert_eq!(config.warmup.timeout_seconds, 120);
    assert_eq!(config.warmup.retry_on_failure, true);
    assert_eq!(
        config.warmup.prompt_template,
        Some("Hello from warmup".to_string())
    );
}

#[test]
fn warmup_parse_toml_with_empty_warmup_section() {
    let toml_str = r#"
        [warmup]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    // All fields should use defaults
    assert_eq!(config.warmup.enabled, false);
    assert_eq!(config.warmup.max_queries, 3);
    assert_eq!(config.warmup.timeout_seconds, 30);
    assert_eq!(config.warmup.retry_on_failure, false);
}

#[test]
fn warmup_parse_toml_with_partial_warmup_config() {
    let toml_str = r#"
        [warmup]
        enabled = true
        max_queries = 5
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.warmup.enabled, true);
    assert_eq!(config.warmup.max_queries, 5);
    assert_eq!(config.warmup.timeout_seconds, 30); // Default
    assert_eq!(config.warmup.retry_on_failure, false); // Default
}

#[test]
fn warmup_parse_toml_prompt_template_with_special_characters() {
    let toml_str = r#"
        [warmup]
        enabled = true
        prompt_template = "Hello\nWorld\t!\r\nTest"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(
        config.warmup.prompt_template,
        Some("Hello\nWorld\t!\r\nTest".to_string())
    );
}

#[test]
fn warmup_parse_toml_prompt_template_with_unicode() {
    let toml_str = r#"
        [warmup]
        prompt_template = "Hello 世界 🌍 Test"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(
        config.warmup.prompt_template,
        Some("Hello 世界 🌍 Test".to_string())
    );
}

// ============================================================================
// File Loading Tests
// ============================================================================

#[test]
fn warmup_load_config_from_file_with_warmup() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
        default = "claude"

        [warmup]
        enabled = true
        max_queries = 5
        timeout_seconds = 60
        retry_on_failure = true
        prompt_template = "Custom warmup"

        [agents.claude]
        command = "claude"
        model = "claude-sonnet-4-6"
    "#;

    fs::write(&config_path, toml_content).unwrap();

    let config = ltmatrix::config::settings::load_config_file(&config_path).unwrap();
    assert_eq!(config.warmup.enabled, true);
    assert_eq!(config.warmup.max_queries, 5);
    assert_eq!(config.warmup.timeout_seconds, 60);
    assert_eq!(config.warmup.retry_on_failure, true);
    assert_eq!(
        config.warmup.prompt_template,
        Some("Custom warmup".to_string())
    );
}

#[test]
fn warmup_load_config_from_file_with_invalid_warmup_validation() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Note: TOML parsing will succeed, but validation should fail
    let toml_content = r#"
        [warmup]
        enabled = true
        max_queries = 0  # Invalid: will fail validation
    "#;

    fs::write(&config_path, toml_content).unwrap();

    let config = ltmatrix::config::settings::load_config_file(&config_path).unwrap();
    let result = config.warmup.validate();
    assert!(
        result.is_err(),
        "Config with max_queries=0 should fail validation"
    );
}

// ============================================================================
// Config Merge Tests with Warmup
// ============================================================================

#[test]
fn warmup_merge_global_and_project_configs() {
    use ltmatrix::config::settings::merge_configs;

    let global_toml = r#"
        [warmup]
        enabled = true
        max_queries = 2
        timeout_seconds = 30
    "#;

    let project_toml = r#"
        [warmup]
        enabled = false
        max_queries = 10
        timeout_seconds = 90
        retry_on_failure = true
    "#;

    let global: Config = toml::from_str(global_toml).unwrap();
    let project: Config = toml::from_str(project_toml).unwrap();

    let merged = merge_configs(Some(global), Some(project));

    // Project config should override global
    assert_eq!(merged.warmup.enabled, false);
    assert_eq!(merged.warmup.max_queries, 10);
    assert_eq!(merged.warmup.timeout_seconds, 90);
    assert_eq!(merged.warmup.retry_on_failure, true);
}

#[test]
fn warmup_merge_global_only() {
    use ltmatrix::config::settings::merge_configs;

    let global_toml = r#"
        [warmup]
        enabled = true
        max_queries = 5
        prompt_template = "Global warmup"
    "#;

    let global: Config = toml::from_str(global_toml).unwrap();
    let merged = merge_configs(Some(global), None); // No project config

    // Global config should be used when there's no project config
    assert_eq!(merged.warmup.enabled, true);
    assert_eq!(merged.warmup.max_queries, 5);
    assert_eq!(
        merged.warmup.prompt_template,
        Some("Global warmup".to_string())
    );
}

#[test]
fn warmup_merge_project_only() {
    use ltmatrix::config::settings::merge_configs;

    let project_toml = r#"
        [warmup]
        enabled = true
        timeout_seconds = 120
    "#;

    let global = Config::default();
    let project: Config = toml::from_str(project_toml).unwrap();

    let merged = merge_configs(Some(global), Some(project));

    // Project config should override defaults
    assert_eq!(merged.warmup.enabled, true);
    assert_eq!(merged.warmup.timeout_seconds, 120);
    assert_eq!(merged.warmup.max_queries, 3); // Default
}

#[test]
fn warmup_merge_both_empty() {
    use ltmatrix::config::settings::merge_configs;

    let global = Config::default();
    let project = Config::default();

    let merged = merge_configs(Some(global), Some(project));

    // Should use defaults
    assert_eq!(merged.warmup.enabled, false);
    assert_eq!(merged.warmup.max_queries, 3);
    assert_eq!(merged.warmup.timeout_seconds, 30);
}

// ============================================================================
// Serialization/Deserialization Roundtrip Tests
// ============================================================================

#[test]
fn warmup_roundtrip_with_all_fields() {
    let original = WarmupConfig {
        enabled: true,
        max_queries: 7,
        timeout_seconds: 120,
        retry_on_failure: true,
        prompt_template: Some("Roundtrip test prompt".to_string()),
    };

    let toml_string = toml::to_string(&original).unwrap();
    let deserialized: WarmupConfig = toml::from_str(&toml_string).unwrap();

    assert_eq!(original.enabled, deserialized.enabled);
    assert_eq!(original.max_queries, deserialized.max_queries);
    assert_eq!(original.timeout_seconds, deserialized.timeout_seconds);
    assert_eq!(original.retry_on_failure, deserialized.retry_on_failure);
    assert_eq!(original.prompt_template, deserialized.prompt_template);
}

#[test]
fn warmup_roundtrip_with_minimal_fields() {
    let original = WarmupConfig {
        enabled: false,
        max_queries: 3,
        timeout_seconds: 30,
        retry_on_failure: false,
        prompt_template: None,
    };

    let toml_string = toml::to_string(&original).unwrap();
    let deserialized: WarmupConfig = toml::from_str(&toml_string).unwrap();

    assert_eq!(original.enabled, deserialized.enabled);
    assert_eq!(original.max_queries, deserialized.max_queries);
    assert_eq!(original.timeout_seconds, deserialized.timeout_seconds);
    assert_eq!(original.retry_on_failure, deserialized.retry_on_failure);
    assert_eq!(original.prompt_template, deserialized.prompt_template);
}

// ============================================================================
// Integration with Other Config Sections
// ============================================================================

#[test]
fn warmup_config_alongside_other_sections() {
    let toml_str = r#"
        default = "claude"

        [warmup]
        enabled = true
        max_queries = 5

        [agents.claude]
        command = "claude"
        model = "claude-sonnet-4-6"

        [output]
        format = "json"

        [logging]
        level = "debug"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();

    // Warmup config
    assert_eq!(config.warmup.enabled, true);
    assert_eq!(config.warmup.max_queries, 5);

    // Other sections should still work
    assert_eq!(config.default, Some("claude".to_string()));
    assert!(config.agents.contains_key("claude"));
    assert_eq!(
        config.output.format,
        ltmatrix::config::settings::OutputFormat::Json
    );
    assert_eq!(
        config.logging.level,
        ltmatrix::config::settings::LogLevel::Debug
    );
}

#[test]
fn warmup_config_in_full_config_file() {
    let toml_str = r#"
        default = "claude"

        [warmup]
        enabled = false
        max_queries = 3
        timeout_seconds = 30
        retry_on_failure = false

        [agents.claude]
        command = "claude"
        model = "claude-sonnet-4-6"
        timeout = 3600

        [modes.fast]
        model = "claude-haiku-4-5"
        run_tests = false

        [output]
        format = "text"
        colored = true

        [logging]
        level = "info"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();

    // Verify warmup is integrated with other config
    assert!(
        config.warmup.validate().is_ok(),
        "Warmup config should be valid"
    );
    assert_eq!(config.warmup.enabled, false);
    assert_eq!(config.agents.len(), 1);
    assert!(config.modes.fast.is_some());
}

// ============================================================================
// Default Values Consistency Tests
// ============================================================================

#[test]
fn warmup_default_values_are_consistent() {
    let config1 = WarmupConfig::default();
    let config2 = WarmupConfig::default();

    assert_eq!(config1.enabled, config2.enabled);
    assert_eq!(config1.max_queries, config2.max_queries);
    assert_eq!(config1.timeout_seconds, config2.timeout_seconds);
    assert_eq!(config1.retry_on_failure, config2.retry_on_failure);
    assert_eq!(config1.prompt_template, config2.prompt_template);
}

#[test]
fn warmup_default_values_pass_validation() {
    let config = WarmupConfig::default();
    assert!(
        config.validate().is_ok(),
        "Default warmup config should be valid"
    );
}

#[test]
fn config_default_includes_warmup_defaults() {
    let config = Config::default();
    let warmup = WarmupConfig::default();

    assert_eq!(config.warmup.enabled, warmup.enabled);
    assert_eq!(config.warmup.max_queries, warmup.max_queries);
    assert_eq!(config.warmup.timeout_seconds, warmup.timeout_seconds);
    assert_eq!(config.warmup.retry_on_failure, warmup.retry_on_failure);
    assert_eq!(config.warmup.prompt_template, warmup.prompt_template);
}
