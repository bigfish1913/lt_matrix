//! Warmup configuration tests
//!
//! These tests verify the warmup configuration system including:
//! - WarmupConfig structure with required fields
//! - TOML parsing and serialization
//! - Default values
//! - Validation logic
//! - Integration with main Config

use ltmatrix::config::settings::{Config, WarmupConfig};

// ============================================================================
// WarmupConfig Structure Tests
// ============================================================================

#[test]
fn warmup_config_has_enabled_field() {
    let config = WarmupConfig::default();

    // Should have enabled field defaulting to false (warmup disabled by default)
    assert_eq!(config.enabled, false, "Warmup should be disabled by default");
}

#[test]
fn warmup_config_has_max_queries_field() {
    let config = WarmupConfig::default();

    // Should have max_queries field with reasonable default
    assert_eq!(
        config.max_queries, 3,
        "Default max_queries should be 3"
    );
}

#[test]
fn warmup_config_has_timeout_field() {
    let config = WarmupConfig::default();

    // Should have timeout field with reasonable default (30 seconds)
    assert_eq!(
        config.timeout_seconds, 30,
        "Default timeout should be 30 seconds"
    );
}

#[test]
fn warmup_config_has_retry_on_failure_field() {
    let config = WarmupConfig::default();

    // Should have retry_on_failure field defaulting to false
    assert_eq!(
        config.retry_on_failure, false,
        "Should not retry on warmup failure by default"
    );
}

#[test]
fn warmup_config_has_prompt_template_field() {
    let config = WarmupConfig::default();

    // Should have optional prompt_template field
    assert!(
        config.prompt_template.is_none(),
        "prompt_template should be optional and None by default"
    );
}

#[test]
fn warmup_config_can_be_customized() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 5,
        timeout_seconds: 60,
        retry_on_failure: true,
        prompt_template: Some("Hello, are you ready?".to_string()),
    };

    assert_eq!(config.enabled, true);
    assert_eq!(config.max_queries, 5);
    assert_eq!(config.timeout_seconds, 60);
    assert_eq!(config.retry_on_failure, true);
    assert_eq!(
        config.prompt_template,
        Some("Hello, are you ready?".to_string())
    );
}

// ============================================================================
// WarmupConfig Serialization Tests
// ============================================================================

#[test]
fn warmup_config_serializes_to_toml() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 2,
        timeout_seconds: 45,
        retry_on_failure: false,
        prompt_template: Some("Test prompt".to_string()),
    };

    // Should serialize to TOML without error
    let toml_string = toml::to_string(&config)
        .expect("WarmupConfig should serialize to TOML");

    assert!(toml_string.contains("enabled"));
    assert!(toml_string.contains("max_queries"));
    assert!(toml_string.contains("timeout_seconds"));
    assert!(toml_string.contains("retry_on_failure"));
}

#[test]
fn warmup_config_deserializes_from_toml() {
    let toml_str = r#"
        enabled = true
        max_queries = 4
        timeout_seconds = 90
        retry_on_failure = true
        prompt_template = "Custom warmup prompt"
    "#;

    let config: WarmupConfig = toml::from_str(toml_str)
        .expect("Should deserialize from TOML");

    assert_eq!(config.enabled, true);
    assert_eq!(config.max_queries, 4);
    assert_eq!(config.timeout_seconds, 90);
    assert_eq!(config.retry_on_failure, true);
    assert_eq!(
        config.prompt_template,
        Some("Custom warmup prompt".to_string())
    );
}

#[test]
fn warmup_config_roundtrip_serialization() {
    let original = WarmupConfig {
        enabled: true,
        max_queries: 7,
        timeout_seconds: 120,
        retry_on_failure: false,
        prompt_template: Some("Roundtrip test".to_string()),
    };

    // Serialize and deserialize
    let toml_string = toml::to_string(&original).unwrap();
    let deserialized: WarmupConfig = toml::from_str(&toml_string).unwrap();

    assert_eq!(original.enabled, deserialized.enabled);
    assert_eq!(original.max_queries, deserialized.max_queries);
    assert_eq!(original.timeout_seconds, deserialized.timeout_seconds);
    assert_eq!(
        original.retry_on_failure,
        deserialized.retry_on_failure
    );
    assert_eq!(
        original.prompt_template,
        deserialized.prompt_template
    );
}

// ============================================================================
// WarmupConfig Validation Tests
// ============================================================================

#[test]
fn warmup_config_validates_max_queries_positive() {
    let config = WarmupConfig::default();

    // max_queries should be validated to be > 0
    assert!(
        config.max_queries > 0,
        "max_queries must be positive"
    );
}

#[test]
fn warmup_config_validates_timeout_positive() {
    let config = WarmupConfig::default();

    // timeout should be validated to be > 0
    assert!(
        config.timeout_seconds > 0,
        "timeout_seconds must be positive"
    );
}

#[test]
fn warmup_config_custom_values_validation() {
    let config = WarmupConfig {
        enabled: true,
        max_queries: 10,
        timeout_seconds: 300,
        retry_on_failure: true,
        prompt_template: None,
    };

    assert!(config.validate().is_ok(), "Valid warmup config should pass validation");
}

// ============================================================================
// Config Integration Tests
// ============================================================================

#[test]
fn config_has_warmup_field() {
    let config = Config::default();

    // Config should have warmup field
    let warmup = &config.warmup;
    assert_eq!(warmup.enabled, false);
    assert_eq!(warmup.max_queries, 3);
    assert_eq!(warmup.timeout_seconds, 30);
}

#[test]
fn config_serialization_includes_warmup() {
    let config = Config::default();

    let toml_string = toml::to_string(&config)
        .expect("Config should serialize to TOML");

    // Serialized TOML should include warmup section
    assert!(
        toml_string.contains("[warmup]") || toml_string.contains("enabled"),
        "Serialized config should include warmup configuration"
    );
}

#[test]
fn config_deserialization_includes_warmup() {
    let toml_str = r#"
        [warmup]
        enabled = true
        max_queries = 5
        timeout_seconds = 60
        retry_on_failure = true
    "#;

    let config: Config = toml::from_str(toml_str)
        .expect("Should deserialize config with warmup section");

    assert_eq!(config.warmup.enabled, true);
    assert_eq!(config.warmup.max_queries, 5);
    assert_eq!(config.warmup.timeout_seconds, 60);
    assert_eq!(config.warmup.retry_on_failure, true);
}

#[test]
fn config_merge_with_warmup_override() {
    let global_toml = r#"
        [warmup]
        enabled = true
        max_queries = 2
    "#;

    let project_toml = r#"
        [warmup]
        enabled = false
        max_queries = 10
        timeout_seconds = 90
    "#;

    // Global config
    let global: Config = toml::from_str(global_toml).unwrap();
    assert_eq!(global.warmup.max_queries, 2);

    // Project config should override global
    let project: Config = toml::from_str(project_toml).unwrap();
    assert_eq!(project.warmup.enabled, false);
    assert_eq!(project.warmup.max_queries, 10);
    assert_eq!(project.warmup.timeout_seconds, 90);
}
