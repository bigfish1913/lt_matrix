//! Integration tests for Config Data Structures (Task: Design and implement config data structures)
//!
//! These tests verify the acceptance criteria:
//! - Config, AgentConfig, ModeConfig structs are defined
//! - Appropriate serde serialization traits
//! - Default values for all fields
//! - Agent backends, models, timeouts, retry limits

use ltmatrix::config::agent::AgentBackend;
use ltmatrix::config::agent::AgentConfig as AgentModuleConfig;
use ltmatrix::config::modes::{ExecutionMode, ModeConfig as ModeModuleConfig};
use ltmatrix::config::settings::{AgentConfig, Config, ModeConfig};

// ============================================================================
// Verification: Config Struct Exists with Required Fields
// ============================================================================

#[test]
fn test_config_struct_exists() {
    // Verify Config struct can be instantiated
    let config = Config::default();

    // Verify default agent is set
    assert_eq!(config.default, Some("claude".to_string()));

    // Verify agents HashMap exists
    assert!(config.agents.is_empty());

    // Verify modes configuration exists
    assert!(config.modes.fast.is_none());
    assert!(config.modes.standard.is_none());
    assert!(config.modes.expert.is_none());
}

#[test]
fn test_config_has_serde_traits() {
    // Verify Config implements Serialize/Deserialize via TOML roundtrip
    let config = Config::default();

    // Should serialize to TOML
    let toml_string = toml::to_string(&config).unwrap();
    assert!(!toml_string.is_empty());

    // Should deserialize from TOML
    let deserialized: Config = toml::from_str(&toml_string).unwrap();
    assert_eq!(deserialized.default, config.default);
}

// ============================================================================
// Verification: AgentConfig in settings.rs
// ============================================================================

#[test]
fn test_settings_agent_config_exists() {
    // Verify AgentConfig in settings.rs has required fields
    let agent_config = AgentConfig {
        command: Some("test-command".to_string()),
        model: Some("test-model".to_string()),
        timeout: Some(3600),
    };

    assert_eq!(agent_config.command, Some("test-command".to_string()));
    assert_eq!(agent_config.model, Some("test-model".to_string()));
    assert_eq!(agent_config.timeout, Some(3600));
}

// ============================================================================
// Verification: AgentConfig in agent.rs (Richer version)
// ============================================================================

#[test]
fn test_agent_module_config_exists() {
    // Verify AgentConfig in agent.rs with builder pattern
    let config = AgentModuleConfig::new("claude-sonnet-4-6")
        .with_timeout(1800)
        .with_max_retries(5);

    assert_eq!(config.model, "claude-sonnet-4-6");
    assert_eq!(config.timeout, 1800);
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.args.len(), 0);
    assert_eq!(config.env.len(), 0);
}

#[test]
fn test_agent_module_config_default_values() {
    let config = AgentModuleConfig::default();

    // Verify all fields have defaults
    assert_eq!(config.model, "");
    assert_eq!(config.command, None);
    assert_eq!(config.timeout, 3600); // 1 hour default
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.verbose, false);
    assert!(config.args.is_empty());
    assert!(config.env.is_empty());
}

#[test]
fn test_agent_backend_enum_exists() {
    // Verify Display implementation
    assert_eq!(AgentBackend::Claude.to_string(), "claude");
    assert_eq!(AgentBackend::OpenCode.to_string(), "opencode");
    assert_eq!(AgentBackend::KimiCode.to_string(), "kimi-code");
    assert_eq!(AgentBackend::Codex.to_string(), "codex");
}

#[test]
fn test_agent_backend_default() {
    assert_eq!(AgentBackend::default(), AgentBackend::Claude);
}

#[test]
fn test_agent_backend_serialization() {
    // Verify serde serialization works
    let backend = AgentBackend::Claude;
    let json = serde_json::to_string(&backend).unwrap();
    assert_eq!(json, "\"claude\"");

    let deserialized: AgentBackend = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, backend);
}

// ============================================================================
// Verification: ModeConfig Structs
// ============================================================================

#[test]
fn test_settings_mode_config_exists() {
    // Verify ModeConfig in settings.rs
    let mode_config = ModeConfig {
        model: Some("test-model".to_string()),
        run_tests: true,
        verify: false,
        max_retries: 2,
        max_depth: 3,
        timeout_plan: 90,
        timeout_exec: 1800,
    };

    assert_eq!(mode_config.model, Some("test-model".to_string()));
    assert_eq!(mode_config.run_tests, true);
    assert_eq!(mode_config.verify, false);
    assert_eq!(mode_config.max_retries, 2);
    assert_eq!(mode_config.max_depth, 3);
    assert_eq!(mode_config.timeout_plan, 90);
    assert_eq!(mode_config.timeout_exec, 1800);
}

#[test]
fn test_mode_module_config_exists() {
    // Verify ModeConfig in modes.rs with builder
    let config = ModeModuleConfig::standard_mode();

    assert_eq!(config.model, "claude-sonnet-4-6");
    assert_eq!(config.max_depth, 3);
    assert_eq!(config.max_retries, 3);
    assert!(config.run_tests);
    assert!(config.verify);
    assert_eq!(config.timeout_plan, 120);
    assert_eq!(config.timeout_exec, 3600);
}

#[test]
fn test_mode_module_config_default_values() {
    let config = ModeModuleConfig::default();

    // Verify all fields have defaults
    assert_eq!(config.model, "");
    assert_eq!(config.model_fast, None);
    assert_eq!(config.model_smart, None);
    assert_eq!(config.max_depth, 3);
    assert_eq!(config.max_retries, 3);
    assert!(config.run_tests); // Default: true
    assert!(config.verify); // Default: true
    assert_eq!(config.timeout_plan, 120);
    assert_eq!(config.timeout_exec, 3600);
}

#[test]
fn test_execution_mode_enum_exists() {
    // Verify all execution modes are defined
    assert_eq!(ExecutionMode::default(), ExecutionMode::Standard);

    // Verify Display implementation
    assert_eq!(ExecutionMode::Fast.to_string(), "fast");
    assert_eq!(ExecutionMode::Standard.to_string(), "standard");
    assert_eq!(ExecutionMode::Expert.to_string(), "expert");
}

// ============================================================================
// Verification: Default Values for Timeouts and Retries
// ============================================================================

#[test]
fn test_default_timeout_values() {
    // AgentConfig default timeout
    let agent_config = AgentModuleConfig::default();
    assert_eq!(agent_config.timeout, 3600); // 1 hour in seconds

    // ModeConfig default timeouts
    let mode_config = ModeModuleConfig::default();
    assert_eq!(mode_config.timeout_plan, 120); // 2 minutes
    assert_eq!(mode_config.timeout_exec, 3600); // 1 hour
}

#[test]
fn test_default_retry_limits() {
    // AgentConfig default retries
    let agent_config = AgentModuleConfig::default();
    assert_eq!(agent_config.max_retries, 3);

    // ModeConfig default retries
    let mode_config = ModeModuleConfig::default();
    assert_eq!(mode_config.max_retries, 3);
}

// ============================================================================
// Verification: Predefined Agent Models
// ============================================================================

#[test]
fn test_claude_model_presets() {
    // Verify Claude model presets exist
    let sonnet = AgentModuleConfig::claude_sonnet();
    assert_eq!(sonnet.model, "claude-sonnet-4-6");
    assert_eq!(sonnet.timeout, 3600);
    assert_eq!(sonnet.max_retries, 3);

    let opus = AgentModuleConfig::claude_opus();
    assert_eq!(opus.model, "claude-opus-4-6");
    assert_eq!(opus.timeout, 7200); // Longer for complex tasks
    assert_eq!(opus.max_retries, 3);

    let haiku = AgentModuleConfig::claude_haiku();
    assert_eq!(haiku.model, "claude-haiku-4-5");
    assert_eq!(haiku.timeout, 1800); // Shorter for fast tasks
    assert_eq!(haiku.max_retries, 2);
}

#[test]
fn test_other_backend_presets() {
    // Verify OpenCode preset
    let opencode = AgentModuleConfig::opencode_default();
    assert_eq!(opencode.model, "gpt-4");
    assert_eq!(opencode.command, Some("opencode".to_string()));

    // Verify KimiCode preset
    let kimicode = AgentModuleConfig::kimicode_default();
    assert_eq!(kimicode.model, "moonshot-v1");
    assert_eq!(kimicode.command, Some("kimi-code".to_string()));

    // Verify Codex preset
    let codex = AgentModuleConfig::codex_default();
    assert_eq!(codex.model, "code-davinci-002");
    assert_eq!(codex.command, Some("codex".to_string()));
}

// ============================================================================
// Verification: Predefined Mode Configurations
// ============================================================================

#[test]
fn test_fast_mode_configuration() {
    let fast = ModeModuleConfig::fast_mode();

    assert_eq!(fast.model, "claude-haiku-4-5");
    assert_eq!(fast.max_depth, 2);
    assert_eq!(fast.max_retries, 1);
    assert!(!fast.run_tests); // Skips tests for speed
    assert!(fast.verify);
    assert_eq!(fast.timeout_plan, 60);
    assert_eq!(fast.timeout_exec, 1800);
}

#[test]
fn test_standard_mode_configuration() {
    let standard = ModeModuleConfig::standard_mode();

    assert_eq!(standard.model, "claude-sonnet-4-6");
    assert_eq!(standard.model_fast, Some("claude-sonnet-4-6".to_string()));
    assert_eq!(standard.model_smart, Some("claude-opus-4-6".to_string()));
    assert_eq!(standard.max_depth, 3);
    assert_eq!(standard.max_retries, 3);
    assert!(standard.run_tests);
    assert!(standard.verify);
    assert_eq!(standard.timeout_plan, 120);
    assert_eq!(standard.timeout_exec, 3600);
}

#[test]
fn test_expert_mode_configuration() {
    let expert = ModeModuleConfig::expert_mode();

    assert_eq!(expert.model, "claude-opus-4-6");
    assert_eq!(expert.max_depth, 4);
    assert_eq!(expert.max_retries, 5);
    assert!(expert.run_tests);
    assert!(expert.verify);
    assert_eq!(expert.timeout_plan, 300);
    assert_eq!(expert.timeout_exec, 7200);
}

// ============================================================================
// Verification: Builder Pattern Methods
// ============================================================================

#[test]
fn test_agent_config_builder_methods() {
    let config = AgentModuleConfig::new("test-model")
        .with_command("custom-command")
        .with_timeout(999)
        .with_arg("--verbose")
        .with_arg("--debug")
        .with_env("API_KEY", "secret")
        .with_max_retries(10)
        .with_verbose(true);

    assert_eq!(config.model, "test-model");
    assert_eq!(config.command, Some("custom-command".to_string()));
    assert_eq!(config.timeout, 999);
    assert_eq!(config.args.len(), 2);
    assert_eq!(config.env.len(), 1);
    assert_eq!(config.max_retries, 10);
    assert!(config.verbose);
}

#[test]
fn test_mode_config_builder_methods() {
    let config = ModeModuleConfig::new("base-model")
        .with_fast_model("fast-model")
        .with_smart_model("smart-model")
        .with_max_depth(10)
        .with_max_retries(7)
        .with_tests(false)
        .with_verify(false)
        .with_timeout_plan(600)
        .with_timeout_exec(14400);

    assert_eq!(config.model, "base-model");
    assert_eq!(config.model_fast, Some("fast-model".to_string()));
    assert_eq!(config.model_smart, Some("smart-model".to_string()));
    assert_eq!(config.max_depth, 10);
    assert_eq!(config.max_retries, 7);
    assert!(!config.run_tests);
    assert!(!config.verify);
    assert_eq!(config.timeout_plan, 600);
    assert_eq!(config.timeout_exec, 14400);
}

// ============================================================================
// Verification: Serde Serialization for All Types
// ============================================================================

#[test]
fn test_agent_config_serialization_roundtrip() {
    let original = AgentModuleConfig::claude_opus();

    // Serialize to JSON
    let json = serde_json::to_string(&original).unwrap();

    // Deserialize back
    let restored: AgentModuleConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.model, original.model);
    assert_eq!(restored.timeout, original.timeout);
    assert_eq!(restored.max_retries, original.max_retries);
}

#[test]
fn test_mode_config_serialization_roundtrip() {
    let original = ModeModuleConfig::standard_mode();

    // Serialize to JSON
    let json = serde_json::to_string(&original).unwrap();

    // Deserialize back
    let restored: ModeModuleConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.model, original.model);
    assert_eq!(restored.max_depth, original.max_depth);
    assert_eq!(restored.run_tests, original.run_tests);
}

#[test]
fn test_execution_mode_serialization() {
    let modes = vec![
        ExecutionMode::Fast,
        ExecutionMode::Standard,
        ExecutionMode::Expert,
    ];

    for mode in modes {
        let json = serde_json::to_string(&mode).unwrap();
        let restored: ExecutionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, mode);
    }
}

// ============================================================================
// Edge Cases and Validation
// ============================================================================

#[test]
fn test_agent_config_get_command() {
    let config = AgentModuleConfig::claude_default();

    // Should return default command for each backend
    assert_eq!(config.get_command(&AgentBackend::Claude), "claude");
    assert_eq!(config.get_command(&AgentBackend::OpenCode), "opencode");
    assert_eq!(config.get_command(&AgentBackend::KimiCode), "kimi-code");
    assert_eq!(config.get_command(&AgentBackend::Codex), "codex");

    // Should return custom command if set
    let custom_config = AgentModuleConfig::new("test").with_command("my-custom-agent");
    assert_eq!(
        custom_config.get_command(&AgentBackend::Claude),
        "my-custom-agent"
    );
}

#[test]
fn test_multiple_agents_in_config() {
    let mut config = Config::default();

    // Add multiple agents
    config.agents.insert(
        "claude".to_string(),
        AgentConfig {
            command: Some("claude".to_string()),
            model: Some("claude-sonnet-4-6".to_string()),
            timeout: Some(3600),
        },
    );

    config.agents.insert(
        "opencode".to_string(),
        AgentConfig {
            command: Some("opencode".to_string()),
            model: Some("gpt-4".to_string()),
            timeout: Some(1800),
        },
    );

    assert_eq!(config.agents.len(), 2);
    assert!(config.agents.contains_key("claude"));
    assert!(config.agents.contains_key("opencode"));
}

#[test]
fn test_zero_values_in_config() {
    // Test that zero values are handled correctly
    let config = AgentModuleConfig::new("test")
        .with_timeout(0)
        .with_max_retries(0);

    assert_eq!(config.timeout, 0);
    assert_eq!(config.max_retries, 0);
}
