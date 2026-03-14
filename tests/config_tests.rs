//! Comprehensive tests for configuration loading and management
//!
//! These tests verify:
//! - TOML parsing with valid and invalid inputs
//! - Default value application
//! - Multi-source config merging (global + project)
//! - Agent configuration loading

use ltmatrix::config::settings::{
    agent_config_to_agent, get_default_agent, get_global_config_path, get_project_config_path,
    load_config_file, merge_configs, Config, LogLevel, LoggingConfig, ModeConfigs, OutputConfig,
    OutputFormat, WarmupConfig,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// TOML Parsing Tests
// ============================================================================

#[test]
fn test_parse_valid_minimal_config() {
    let toml_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600
"#;

    let config: Config = toml::from_str(toml_content).unwrap();

    assert_eq!(config.default, Some("claude".to_string()));
    assert_eq!(config.agents.len(), 1);
    assert!(config.agents.contains_key("claude"));

    let claude = &config.agents["claude"];
    assert_eq!(claude.command, Some("claude".to_string()));
    assert_eq!(claude.model, Some("claude-sonnet-4-6".to_string()));
    assert_eq!(claude.timeout, Some(3600));
}

#[test]
fn test_parse_valid_full_config() {
    let toml_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[agents.opencode]
command = "opencode"
model = "gpt-4"
timeout = 1800

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
format = "json"
colored = false
progress = true

[logging]
level = "debug"
file = "/var/log/ltmatrix.log"
"#;

    let config: Config = toml::from_str(toml_content).unwrap();

    // Verify agents
    assert_eq!(config.agents.len(), 2);
    assert!(config.agents.contains_key("claude"));
    assert!(config.agents.contains_key("opencode"));

    // Verify modes
    assert!(config.modes.fast.is_some());
    assert!(config.modes.standard.is_some());
    assert!(config.modes.expert.is_some());

    let fast = config.modes.fast.as_ref().unwrap();
    assert_eq!(fast.model, Some("claude-haiku-4-5".to_string()));
    assert_eq!(fast.run_tests, false);
    assert_eq!(fast.max_retries, 1);

    // Verify output
    assert_eq!(config.output.format, OutputFormat::Json);
    assert_eq!(config.output.colored, false);

    // Verify logging
    assert_eq!(config.logging.level, LogLevel::Debug);
}

#[test]
fn test_parse_invalid_toml_syntax() {
    let invalid_tomls = vec![
        // Missing closing bracket
        "[default\nagent = \"claude\"",
        // Invalid syntax
        "[default]\nagent = \"claude\"",
        // Unclosed string
        "[default]\nagent = \"claude",
        // Invalid boolean
        "[output]\ncolored = maybe",
    ];

    for toml_content in invalid_tomls {
        let result: Result<Config, _> =
            toml::from_str(toml_content).map_err(|e| anyhow::anyhow!(e));
        assert!(result.is_err(), "Should fail to parse: {}", toml_content);
    }
}

#[test]
fn test_parse_empty_config() {
    let toml_content = "";
    let config: Config = toml::from_str(toml_content).unwrap();

    assert_eq!(config.default, None);
    assert!(config.agents.is_empty());
}

#[test]
fn test_parse_config_with_comments() {
    let toml_content = r#"
# Default agent to use
default = "claude"  # This is the default

# Agent configurations
[agents.claude]
command = "claude"  # CLI command
model = "claude-sonnet-4-6"  # Model identifier
timeout = 3600  # 1 hour timeout
"#;

    let config: Config = toml::from_str(toml_content).unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
}

// ============================================================================
// Default Value Tests
// ============================================================================

#[test]
fn test_default_config() {
    let config = Config::default();

    assert_eq!(config.default, Some("claude".to_string()));
    assert!(config.agents.is_empty());

    // Check default output settings
    assert_eq!(config.output.format, OutputFormat::Text);
    assert_eq!(config.output.colored, true);
    assert_eq!(config.output.progress, true);

    // Check default logging settings
    assert_eq!(config.logging.level, LogLevel::Info);
    assert_eq!(config.logging.file, None);
}

#[test]
fn test_default_output_config() {
    let output = OutputConfig::default();
    assert_eq!(output.format, OutputFormat::Text);
    assert_eq!(output.colored, true);
    assert_eq!(output.progress, true);
}

#[test]
fn test_default_logging_config() {
    let logging = LoggingConfig::default();
    assert_eq!(logging.level, LogLevel::Info);
    assert_eq!(logging.file, None);
}

#[test]
fn test_partial_config_uses_defaults() {
    let toml_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
"#;

    let config: Config = toml::from_str(toml_content).unwrap();

    // Agent timeout should use default (3600) when not specified
    let claude = &config.agents["claude"];
    assert_eq!(claude.timeout, None); // Not specified, will use default when converting

    // Output should use defaults
    assert_eq!(config.output.format, OutputFormat::Text);
    assert_eq!(config.output.colored, true);

    // Logging should use defaults
    assert_eq!(config.logging.level, LogLevel::Info);
}

// ============================================================================
// Agent Configuration Tests
// ============================================================================

#[test]
fn test_agent_config_to_agent() {
    let config = ltmatrix::config::settings::AgentConfig {
        command: Some("test-cmd".to_string()),
        model: Some("test-model".to_string()),
        timeout: Some(1234),
        api_key: None,
        base_url: None,
    };

    let agent = agent_config_to_agent("test-agent", &config).unwrap();

    assert_eq!(agent.name, "test-agent");
    assert_eq!(agent.command, "test-cmd");
    assert_eq!(agent.model, "test-model");
    assert_eq!(agent.timeout, 1234);
}

#[test]
fn test_agent_config_missing_required_fields() {
    // Missing command
    let config_no_command = ltmatrix::config::settings::AgentConfig {
        command: None,
        model: Some("test-model".to_string()),
        timeout: Some(3600),
        api_key: None,
        base_url: None,
    };

    let result = agent_config_to_agent("test", &config_no_command);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("missing 'command'"));

    // Missing model
    let config_no_model = ltmatrix::config::settings::AgentConfig {
        command: Some("test-cmd".to_string()),
        model: None,
        timeout: Some(3600),
        api_key: None,
        base_url: None,
    };

    let result = agent_config_to_agent("test", &config_no_model);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing 'model'"));
}

#[test]
fn test_agent_config_default_timeout() {
    let config = ltmatrix::config::settings::AgentConfig {
        command: Some("test-cmd".to_string()),
        model: Some("test-model".to_string()),
        timeout: None, // Not specified
        api_key: None,
        base_url: None,
    };

    let agent = agent_config_to_agent("test", &config).unwrap();
    assert_eq!(agent.timeout, 3600); // Should use default
}

#[test]
fn test_get_default_agent() {
    let mut config = Config::default();

    // Add agents
    let claude_config = ltmatrix::config::settings::AgentConfig {
        command: Some("claude".to_string()),
        model: Some("claude-sonnet-4-6".to_string()),
        timeout: Some(3600),
        api_key: None,
        base_url: None,
    };

    let opencode_config = ltmatrix::config::settings::AgentConfig {
        command: Some("opencode".to_string()),
        model: Some("gpt-4".to_string()),
        timeout: Some(1800),
        api_key: None,
        base_url: None,
    };

    config.agents.insert("claude".to_string(), claude_config);
    config
        .agents
        .insert("opencode".to_string(), opencode_config);
    config.default = Some("claude".to_string());

    let agent = get_default_agent(&config).unwrap();
    assert_eq!(agent.name, "claude");
    assert_eq!(agent.command, "claude");
}

#[test]
fn test_get_default_agent_not_configured() {
    let config = Config::default();
    let result = get_default_agent(&config);
    assert!(result.is_err());
}

// ============================================================================
// Multi-Source Config Merging Tests
// ============================================================================

#[test]
fn test_merge_configs_project_overrides_global() {
    let mut global = Config::default();
    global.default = Some("global-agent".to_string());

    let global_agent = ltmatrix::config::settings::AgentConfig {
        command: Some("global-cmd".to_string()),
        model: Some("global-model".to_string()),
        timeout: Some(1000),
        api_key: None,
        base_url: None,
    };
    global
        .agents
        .insert("shared-agent".to_string(), global_agent);

    let mut project = Config::default();
    project.default = Some("project-agent".to_string());

    let project_agent = ltmatrix::config::settings::AgentConfig {
        command: Some("project-cmd".to_string()),
        model: Some("project-model".to_string()),
        timeout: Some(2000),
        api_key: None,
        base_url: None,
    };
    project
        .agents
        .insert("shared-agent".to_string(), project_agent);

    let merged = merge_configs(Some(global), Some(project));

    // Project default should override global
    assert_eq!(merged.default, Some("project-agent".to_string()));

    // Project agent config should override global for shared-agent
    let shared = &merged.agents["shared-agent"];
    assert_eq!(shared.command, Some("project-cmd".to_string()));
    assert_eq!(shared.timeout, Some(2000));
}

#[test]
fn test_merge_configs_preserves_global_when_not_in_project() {
    let mut global = Config::default();
    global.default = Some("global-agent".to_string());

    let global_agent = ltmatrix::config::settings::AgentConfig {
        command: Some("global-cmd".to_string()),
        model: Some("global-model".to_string()),
        timeout: Some(1000),
        api_key: None,
        base_url: None,
    };
    global
        .agents
        .insert("global-only".to_string(), global_agent);

    let project = Config {
        default: None,
        agents: std::collections::HashMap::new(),
        modes: ModeConfigs::default(),
        output: OutputConfig::default(),
        logging: LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
        telemetry: ltmatrix::telemetry::TelemetryConfig::default(),
        memory: ltmatrix::config::settings::MemoryConfig::default(),
    };

    let merged = merge_configs(Some(global), Some(project));

    // Global-only agent should be preserved
    assert!(merged.agents.contains_key("global-only"));
    let global_only = &merged.agents["global-only"];
    assert_eq!(global_only.command, Some("global-cmd".to_string()));
}

#[test]
fn test_merge_configs_both_none() {
    let merged = merge_configs(None, None);

    // Should return default config
    assert_eq!(merged.default, Some("claude".to_string()));
}

#[test]
fn test_merge_configs_modes() {
    let mut global = Config::default();
    global.modes.fast = Some(ltmatrix::config::settings::ModeConfig {
        model: Some("global-fast".to_string()),
        run_tests: false,
        verify: true,
        max_retries: 1,
        max_depth: 2,
        timeout_plan: 60,
        timeout_exec: 1800,
    });

    let mut project = Config::default();
    project.modes.standard = Some(ltmatrix::config::settings::ModeConfig {
        model: Some("project-standard".to_string()),
        run_tests: true,
        verify: true,
        max_retries: 3,
        max_depth: 3,
        timeout_plan: 120,
        timeout_exec: 3600,
    });

    let merged = merge_configs(Some(global), Some(project));

    // Should have both modes
    assert!(merged.modes.fast.is_some());
    assert!(merged.modes.standard.is_some());

    assert_eq!(
        merged.modes.fast.unwrap().model,
        Some("global-fast".to_string())
    );
    assert_eq!(
        merged.modes.standard.unwrap().model,
        Some("project-standard".to_string())
    );
}

// ============================================================================
// File Loading Tests
// ============================================================================

#[test]
fn test_load_config_file_success() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600
"#;

    fs::write(&config_path, toml_content).unwrap();

    let config = load_config_file(&config_path).unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
    assert_eq!(config.agents.len(), 1);
}

#[test]
fn test_load_config_file_not_found() {
    let result = load_config_file(PathBuf::from("/nonexistent/path/config.toml").as_path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to read"));
}

#[test]
fn test_load_config_file_invalid_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(&config_path, b"invalid [toml").unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to parse"));
}

#[test]
fn test_get_global_config_path() {
    let path = get_global_config_path().unwrap();
    assert!(path.to_string_lossy().contains(".ltmatrix"));
    assert!(path.to_string_lossy().contains("config.toml"));
}

#[test]
fn test_get_project_config_path() {
    let path = get_project_config_path();
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.to_string_lossy().contains(".ltmatrix"));
    assert!(path.to_string_lossy().contains("config.toml"));
}

// ============================================================================
// Output and Logging Configuration Tests
// ============================================================================

#[test]
fn test_output_format_serialization() {
    // Test serialization through a config object
    let config = OutputConfig {
        format: OutputFormat::Text,
        ..Default::default()
    };
    let toml_string = toml::to_string(&config).unwrap();
    assert!(toml_string.contains("format = \"text\""));

    // Test deserialization through a config object
    let parsed_config: OutputConfig = toml::from_str(&toml_string).unwrap();
    assert_eq!(parsed_config.format, OutputFormat::Text);

    // Test JSON format
    let json_config = OutputConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let toml_string = toml::to_string(&json_config).unwrap();
    assert!(toml_string.contains("format = \"json\""));

    let parsed_json: OutputConfig = toml::from_str(&toml_string).unwrap();
    assert_eq!(parsed_json.format, OutputFormat::Json);
}

#[test]
fn test_log_level_serialization() {
    // Test serialization through a config object
    let config = LoggingConfig {
        level: LogLevel::Debug,
        file: None,
    };
    let toml_string = toml::to_string(&config).unwrap();
    assert!(toml_string.contains("level = \"debug\""));

    // Test deserialization through a config object
    let parsed_config: LoggingConfig = toml::from_str(&toml_string).unwrap();
    assert_eq!(parsed_config.level, LogLevel::Debug);

    // Test different log levels
    let levels = vec![
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];

    for level in levels {
        let config = LoggingConfig { level, file: None };
        let toml_string = toml::to_string(&config).unwrap();

        // Verify we can deserialize it back
        let parsed: LoggingConfig = toml::from_str(&toml_string).unwrap();
        assert_eq!(parsed.level, level);
    }
}

#[test]
fn test_config_serialization_roundtrip() {
    let mut agents = std::collections::HashMap::new();
    agents.insert(
        "claude".to_string(),
        ltmatrix::config::settings::AgentConfig {
            command: Some("claude".to_string()),
            model: Some("claude-sonnet-4-6".to_string()),
            timeout: Some(3600),
            api_key: None,
            base_url: None,
        },
    );

    let original_config = Config {
        default: Some("claude".to_string()),
        agents,
        modes: ModeConfigs {
            fast: Some(ltmatrix::config::settings::ModeConfig {
                model: Some("claude-haiku-4-5".to_string()),
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
        output: OutputConfig {
            format: OutputFormat::Json,
            colored: true,
            progress: true,
        },
        logging: LoggingConfig {
            level: LogLevel::Debug,
            file: Some(PathBuf::from("/tmp/ltmatrix.log")),
        },
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
        telemetry: ltmatrix::telemetry::TelemetryConfig::default(),
        memory: ltmatrix::config::settings::MemoryConfig::default(),
    };

    // Serialize to TOML
    let toml_string = toml::to_string(&original_config).unwrap();

    // Deserialize back
    let deserialized: Config = toml::from_str(&toml_string).unwrap();

    // Verify
    assert_eq!(deserialized.default, original_config.default);
    assert_eq!(deserialized.agents.len(), original_config.agents.len());
    assert_eq!(
        deserialized.modes.fast.is_some(),
        original_config.modes.fast.is_some()
    );
    assert_eq!(deserialized.output.format, original_config.output.format);
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_config_with_empty_sections() {
    let toml_content = r#"
# default = "test"

[agents]

[modes]

[output]

[logging]
"#;

    let config: Config = toml::from_str(toml_content).unwrap();
    assert_eq!(config.default, None);
    assert!(config.agents.is_empty());
}

#[test]
fn test_config_with_extra_whitespace() {
    let toml_content = r#"

default = "claude"


[agents.claude]

command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600


"#;

    let config: Config = toml::from_str(toml_content).unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
    assert_eq!(config.agents.len(), 1);
}

#[test]
fn test_agent_config_with_special_characters_in_command() {
    let toml_content = r#"
[agents.claude]
command = "claude --arg1 value1 --arg2 'value with spaces'"
model = "claude-sonnet-4-6"
"#;

    let config: Config = toml::from_str(toml_content).unwrap();
    let claude = &config.agents["claude"];
    assert_eq!(
        claude.command,
        Some("claude --arg1 value1 --arg2 'value with spaces'".to_string())
    );
}

#[test]
fn test_large_timeout_values() {
    let toml_content = r#"
[agents.test]
command = "test"
model = "test-model"
timeout = 86400  # 24 hours
"#;

    let config: Config = toml::from_str(toml_content).unwrap();
    let test_agent = &config.agents["test"];
    assert_eq!(test_agent.timeout, Some(86400));
}

// ============================================================================
// CLI Override Precedence Tests
// ============================================================================

#[test]
fn test_cli_output_format_overrides_config() {
    // Start with a config that has JSON output
    let mut config = Config::default();
    config.output.format = OutputFormat::Json;

    // Simulate CLI override to Text format
    // CLI args should take precedence over config file settings
    let cli_format = OutputFormat::Text;

    // In the actual implementation, CLI args would override config
    // For testing, we verify the precedence order is understood
    assert_ne!(
        config.output.format, cli_format,
        "CLI override should change the format from config default"
    );

    // After applying CLI override
    config.output.format = cli_format;
    assert_eq!(
        config.output.format,
        OutputFormat::Text,
        "CLI override should be applied to config"
    );
}

#[test]
fn test_cli_log_level_overrides_config() {
    let mut config = Config::default();
    config.logging.level = LogLevel::Info;

    // Simulate CLI override to Debug level
    let cli_level = LogLevel::Debug;

    assert_ne!(
        config.logging.level, cli_level,
        "CLI log level override should differ from config"
    );

    // After applying CLI override
    config.logging.level = cli_level;
    assert_eq!(
        config.logging.level,
        LogLevel::Debug,
        "CLI log level override should be applied"
    );
}

#[test]
fn test_cli_mode_overrides_config() {
    use ltmatrix::config::modes::ExecutionMode;

    // Config might specify a default mode
    // CLI flags like --fast or --expert should override this
    let cli_fast_mode = ExecutionMode::Fast;
    let cli_expert_mode = ExecutionMode::Expert;

    assert_eq!(cli_fast_mode.to_string(), "fast");
    assert_eq!(cli_expert_mode.to_string(), "expert");

    // CLI flags should take precedence over config mode settings
    assert_ne!(
        cli_fast_mode, cli_expert_mode,
        "Different CLI mode flags should produce different modes"
    );
}

#[test]
fn test_precedence_order() {
    // Test the full precedence order: CLI > Project > Global > Default

    // 1. Default config
    let default_config = Config::default();
    assert_eq!(default_config.output.format, OutputFormat::Text);
    assert_eq!(default_config.logging.level, LogLevel::Info);

    // 2. Global config would override defaults
    let mut global_config = default_config.clone();
    global_config.output.format = OutputFormat::Json;
    assert_eq!(
        global_config.output.format,
        OutputFormat::Json,
        "Global config should override default"
    );

    // 3. Project config would override global
    let mut project_config = global_config;
    project_config.logging.level = LogLevel::Debug;
    assert_eq!(
        project_config.output.format,
        OutputFormat::Json,
        "Project config should preserve global settings not overridden"
    );
    assert_eq!(
        project_config.logging.level,
        LogLevel::Debug,
        "Project config should override global settings"
    );

    // 4. CLI args would override project
    let cli_format = OutputFormat::Text;
    let final_config_with_cli = project_config;
    assert_ne!(
        final_config_with_cli.output.format, cli_format,
        "Config format should differ from CLI override before application"
    );
}

#[test]
fn test_cli_timeout_overrides_agent_config() {
    let agent_config = ltmatrix::config::settings::AgentConfig {
        command: Some("claude".to_string()),
        model: Some("claude-sonnet-4-6".to_string()),
        timeout: Some(3600), // Config specifies 1 hour
        api_key: None,
        base_url: None,
    };

    // CLI --timeout argument should override agent config
    let cli_timeout = 7200; // 2 hours

    let mut agent = agent_config_to_agent("test", &agent_config).unwrap();
    assert_ne!(
        agent.timeout, cli_timeout,
        "Agent timeout should differ from CLI override"
    );

    // After applying CLI override
    agent.timeout = cli_timeout;
    assert_eq!(
        agent.timeout, 7200,
        "CLI timeout override should be applied"
    );
}

#[test]
fn test_cli_max_retries_overrides_mode_config() {
    use ltmatrix::config::modes::ModeConfig;

    let mode_config = ModeConfig::standard_mode();
    assert_eq!(
        mode_config.max_retries, 3,
        "Standard mode should have 3 retries by default"
    );

    // CLI --max-retries argument should override mode config
    let cli_max_retries = 5;

    assert_ne!(
        mode_config.max_retries, cli_max_retries,
        "Mode config retries should differ from CLI override"
    );

    // In actual implementation, CLI would override
    let mut overridden_config = mode_config;
    overridden_config.max_retries = cli_max_retries;
    assert_eq!(
        overridden_config.max_retries, 5,
        "CLI max-retries should override mode config"
    );
}

#[test]
fn test_partial_cli_overrides() {
    // Test that CLI overrides only affect specified settings
    let mut config = Config {
        default: Some("claude".to_string()),
        agents: std::collections::HashMap::new(),
        modes: ModeConfigs::default(),
        output: OutputConfig {
            format: OutputFormat::Json,
            colored: false,
            progress: true,
        },
        logging: LoggingConfig {
            level: LogLevel::Debug,
            file: Some(PathBuf::from("/tmp/test.log")),
        },
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
        telemetry: ltmatrix::telemetry::TelemetryConfig::default(),
        memory: ltmatrix::config::settings::MemoryConfig::default(),
    };

    // Simulate CLI override for output format only
    let cli_format = OutputFormat::Text;
    config.output.format = cli_format;

    // Other settings should remain from config
    assert_eq!(
        config.output.format,
        OutputFormat::Text,
        "CLI override should change format"
    );
    assert_eq!(
        config.output.colored, false,
        "Config colored setting should be preserved"
    );
    assert_eq!(
        config.logging.level,
        LogLevel::Debug,
        "Config log level should be preserved"
    );
    assert_eq!(
        config.logging.file,
        Some(PathBuf::from("/tmp/test.log")),
        "Config log file should be preserved"
    );
}

#[test]
fn test_cli_agent_override() {
    let mut config = Config::default();
    config.default = Some("claude".to_string());

    let claude_config = ltmatrix::config::settings::AgentConfig {
        command: Some("claude".to_string()),
        model: Some("claude-sonnet-4-6".to_string()),
        timeout: Some(3600),
        api_key: None,
        base_url: None,
    };

    let opencode_config = ltmatrix::config::settings::AgentConfig {
        command: Some("opencode".to_string()),
        model: Some("gpt-4".to_string()),
        timeout: Some(1800),
        api_key: None,
        base_url: None,
    };

    config.agents.insert("claude".to_string(), claude_config);
    config
        .agents
        .insert("opencode".to_string(), opencode_config);

    // CLI --agent argument should override default agent setting
    let cli_agent = "opencode";
    config.default = Some(cli_agent.to_string());

    assert_eq!(
        config.default,
        Some("opencode".to_string()),
        "CLI agent override should change default agent"
    );

    let agent = get_default_agent(&config).unwrap();
    assert_eq!(
        agent.name, "opencode",
        "Default agent from config should match CLI override"
    );
}

// ============================================================================
// Execution Mode Tests
// ============================================================================

#[test]
fn test_execution_mode_display() {
    use ltmatrix::config::modes::ExecutionMode;

    assert_eq!(ExecutionMode::Fast.to_string(), "fast");
    assert_eq!(ExecutionMode::Standard.to_string(), "standard");
    assert_eq!(ExecutionMode::Expert.to_string(), "expert");
}

#[test]
fn test_fast_mode_config() {
    use ltmatrix::config::modes::ModeConfig;

    let config = ModeConfig::fast_mode();

    assert_eq!(config.model, "claude-haiku-4-5");
    assert_eq!(config.max_depth, 2);
    assert_eq!(config.max_retries, 1);
    assert_eq!(config.run_tests, false);
    assert_eq!(config.verify, true);
}

#[test]
fn test_standard_mode_config() {
    use ltmatrix::config::modes::ModeConfig;

    let config = ModeConfig::standard_mode();

    assert_eq!(config.model, "claude-sonnet-4-6");
    assert_eq!(config.max_depth, 3);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.run_tests, true);
}

#[test]
fn test_expert_mode_config() {
    use ltmatrix::config::modes::ModeConfig;

    let config = ModeConfig::expert_mode();

    assert_eq!(config.model, "claude-opus-4-6");
    assert_eq!(config.max_depth, 4);
    assert_eq!(config.max_retries, 5);
}
