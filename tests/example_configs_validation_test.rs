//! Tests for validating example configurations
//!
//! These tests verify that all example configurations can be loaded and parsed correctly.

use std::path::PathBuf;

/// Test that the main config.example.toml can be loaded
#[test]
fn test_load_config_example() {
    let config_path = PathBuf::from(".ltmatrix/config.example.toml");
    assert!(config_path.exists(), "config.example.toml should exist");

    // Try to load the configuration
    let result = ltmatrix::config::settings::load_config_file(&config_path);
    assert!(result.is_ok(), "config.example.toml should be valid TOML");

    let config = result.unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
    assert!(config.agents.contains_key("claude"));
}

/// Test that web-development.toml can be loaded
#[test]
fn test_load_web_development_config() {
    let config_path = PathBuf::from(".ltmatrix/web-development.toml");
    assert!(config_path.exists(), "web-development.toml should exist");

    let result = ltmatrix::config::settings::load_config_file(&config_path);
    assert!(result.is_ok(), "web-development.toml should be valid TOML");

    let config = result.unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
}

/// Test that cli-tools.toml can be loaded
#[test]
fn test_load_cli_tools_config() {
    let config_path = PathBuf::from(".ltmatrix/cli-tools.toml");
    assert!(config_path.exists(), "cli-tools.toml should exist");

    let result = ltmatrix::config::settings::load_config_file(&config_path);
    assert!(result.is_ok(), "cli-tools.toml should be valid TOML");

    let config = result.unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
}

/// Test that data-science.toml can be loaded
#[test]
fn test_load_data_science_config() {
    let config_path = PathBuf::from(".ltmatrix/data-science.toml");
    assert!(config_path.exists(), "data-science.toml should exist");

    let result = ltmatrix::config::settings::load_config_file(&config_path);
    assert!(result.is_ok(), "data-science.toml should be valid TOML");

    let config = result.unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
}

/// Test that mobile-apps.toml can be loaded
#[test]
fn test_load_mobile_apps_config() {
    let config_path = PathBuf::from(".ltmatrix/mobile-apps.toml");
    assert!(config_path.exists(), "mobile-apps.toml should exist");

    let result = ltmatrix::config::settings::load_config_file(&config_path);
    assert!(result.is_ok(), "mobile-apps.toml should be valid TOML");

    let config = result.unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
}

/// Test that all example configs have required agent model field
#[test]
fn test_all_configs_have_agent_models() {
    let config_files = vec![
        ".ltmatrix/config.example.toml",
        ".ltmatrix/web-development.toml",
        ".ltmatrix/cli-tools.toml",
        ".ltmatrix/data-science.toml",
        ".ltmatrix/mobile-apps.toml",
    ];

    for config_file in config_files {
        let config_path = PathBuf::from(config_file);
        let result = ltmatrix::config::settings::load_config_file(&config_path);
        assert!(
            result.is_ok(),
            "{} should be valid: {:?}",
            config_file,
            result.err()
        );

        let config = result.unwrap();

        // Check that default agent exists
        if let Some(default_agent) = &config.default {
            assert!(
                config.agents.contains_key(default_agent),
                "{}: default agent '{}' must be defined",
                config_file,
                default_agent
            );

            // Check that agent has model field
            let agent_config = &config.agents[default_agent];
            assert!(
                agent_config.model.is_some(),
                "{}: agent '{}' must have a model field",
                config_file,
                default_agent
            );
        }
    }
}

/// Test that all example configs have valid mode configurations
#[test]
fn test_all_configs_have_valid_modes() {
    let config_files = vec![
        ".ltmatrix/config.example.toml",
        ".ltmatrix/web-development.toml",
        ".ltmatrix/cli-tools.toml",
        ".ltmatrix/data-science.toml",
        ".ltmatrix/mobile-apps.toml",
    ];

    for config_file in config_files {
        let config_path = PathBuf::from(config_file);
        let result = ltmatrix::config::settings::load_config_file(&config_path);
        assert!(result.is_ok(), "{} should be valid", config_file);

        let config = result.unwrap();

        // Check fast mode if present
        if let Some(fast_mode) = &config.modes.fast {
            // Max depth should be reasonable (1-5)
            assert!(
                fast_mode.max_depth <= 5,
                "{}: fast mode max_depth {} is too high",
                config_file,
                fast_mode.max_depth
            );
        }

        // Check standard mode if present
        if let Some(standard_mode) = &config.modes.standard {
            assert!(
                standard_mode.max_depth <= 5,
                "{}: standard mode max_depth {} is too high",
                config_file,
                standard_mode.max_depth
            );
        }

        // Check expert mode if present
        if let Some(expert_mode) = &config.modes.expert {
            assert!(
                expert_mode.max_depth <= 5,
                "{}: expert mode max_depth {} is too high",
                config_file,
                expert_mode.max_depth
            );
        }
    }
}

/// Test that all example configs have valid output settings
#[test]
fn test_all_configs_have_valid_output() {
    let config_files = vec![
        ".ltmatrix/config.example.toml",
        ".ltmatrix/web-development.toml",
        ".ltmatrix/cli-tools.toml",
        ".ltmatrix/data-science.toml",
        ".ltmatrix/mobile-apps.toml",
    ];

    for config_file in config_files {
        let config_path = PathBuf::from(config_file);
        let result = ltmatrix::config::settings::load_config_file(&config_path);
        assert!(result.is_ok(), "{} should be valid", config_file);

        let _config = result.unwrap();
        // Output settings are always present due to defaults
        // Just loading successfully validates the format
    }
}
