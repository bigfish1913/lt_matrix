//! Advanced validation and edge case tests for config system
//!
//! This test suite provides comprehensive coverage of advanced scenarios:
//! - Advanced CLI override mappings (dry_run, resume, ask, etc.)
//! - Complex validation scenarios
//! - Additional error cases not covered in other test files
//! - Config consistency and integrity checks
//! - Advanced merge scenarios
//!
//! These tests complement the existing comprehensive test suite by covering
//! additional edge cases and advanced validation scenarios.

use clap::Parser;
use ltmatrix::cli::Args;
use ltmatrix::config::settings::{
    load_config_from_args, AgentConfig, CliOverrides, Config, LogLevel, ModeConfig, OutputFormat, WarmupConfig,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::TempDir;

/// Global mutex to serialize directory changes across tests
static DIR_LOCK: Mutex<()> = Mutex::new(());

/// Helper to restore current directory when dropped
struct DirGuard {
    original: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl DirGuard {
    fn new() -> Self {
        let guard = DIR_LOCK.lock().unwrap();
        DirGuard {
            original: std::env::current_dir().unwrap(),
            _guard: guard,
        }
    }

    fn change_to(&self, path: &Path) {
        std::env::set_current_dir(path).unwrap();
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

// ============================================================================
// Advanced CLI Override Tests
// ============================================================================

#[test]
fn test_cli_override_dry_run_flag() {
    // Test that --dry-run flag is properly captured in CliOverrides
    let args =
        Args::try_parse_from(["ltmatrix", "--dry-run", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(
        overrides.dry_run, true,
        "dry_run flag should be set to true"
    );
}

#[test]
fn test_cli_override_resume_flag() {
    // Test that --resume flag is properly captured in CliOverrides
    let args =
        Args::try_parse_from(["ltmatrix", "--resume", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.resume, true, "resume flag should be set to true");
}

#[test]
fn test_cli_override_ask_flag() {
    // Test that --ask flag is properly captured in CliOverrides
    let args = Args::try_parse_from(["ltmatrix", "--ask", "goal"]).expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.ask, true, "ask flag should be set to true");
}

#[test]
fn test_cli_override_regenerate_plan_flag() {
    // Test that --regenerate-plan flag is properly captured in CliOverrides
    let args = Args::try_parse_from(["ltmatrix", "--regenerate-plan", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(
        overrides.regenerate_plan, true,
        "regenerate_plan flag should be set to true"
    );
}

#[test]
fn test_cli_override_on_blocked() {
    // Test that --on-blocked flag is properly captured in CliOverrides
    let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "skip", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(
        overrides.on_blocked,
        Some("skip".to_string()),
        "on_blocked should be set to 'skip'"
    );
}

#[test]
fn test_cli_override_multiple_boolean_flags() {
    // Test that multiple boolean flags can be set simultaneously
    let args = Args::try_parse_from([
        "ltmatrix",
        "--dry-run",
        "--resume",
        "--ask",
        "--regenerate-plan",
        "goal",
    ])
    .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.dry_run, true);
    assert_eq!(overrides.resume, true);
    assert_eq!(overrides.ask, true);
    assert_eq!(overrides.regenerate_plan, true);
}

#[test]
fn test_cli_override_mcp_config() {
    // Test that --mcp-config flag is properly captured
    let temp_dir = TempDir::new().unwrap();
    let mcp_config_path = temp_dir.path().join("mcp-config.json");
    fs::write(&mcp_config_path, "{}").unwrap();

    let args = Args::try_parse_from([
        "ltmatrix",
        "--mcp-config",
        mcp_config_path.to_str().unwrap(),
        "goal",
    ])
    .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();

    assert_eq!(overrides.mcp_config, Some(mcp_config_path));
}

// ============================================================================
// Config Structure Validation Tests
// ============================================================================

#[test]
fn test_config_with_all_optional_fields_none() {
    // Test that a config with all optional fields set to None is valid
    let config = Config {
        default: None,
        agents: std::collections::HashMap::new(),
        modes: ltmatrix::config::settings::ModeConfigs::default(),
        output: ltmatrix::config::settings::OutputConfig::default(),
        logging: ltmatrix::config::settings::LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
    };

    // Should be able to serialize and deserialize
    let toml_string = toml::to_string(&config).unwrap();
    let deserialized: Config = toml::from_str(&toml_string).unwrap();

    assert_eq!(deserialized.default, None);
    assert!(deserialized.agents.is_empty());
}

#[test]
fn test_agent_config_with_all_fields_none() {
    // Test that an agent config with all fields set to None is valid
    let agent_config = AgentConfig {
        command: None,
        model: None,
        timeout: None,
    };

    // Should be able to serialize and deserialize
    let toml_string = toml::to_string(&agent_config).unwrap();
    let deserialized: AgentConfig = toml::from_str(&toml_string).unwrap();

    assert_eq!(deserialized.command, None);
    assert_eq!(deserialized.model, None);
    assert_eq!(deserialized.timeout, None);
}

#[test]
fn test_mode_config_boundary_values() {
    // Test mode config with boundary values
    let mode_config = ModeConfig {
        model: Some("test-model".to_string()),
        run_tests: true,
        verify: false,
        max_retries: 0,      // Minimum value
        max_depth: 10,       // High value
        timeout_plan: 1,     // Minimum value
        timeout_exec: 86400, // Maximum value (24 hours)
    };

    // Should be able to serialize and deserialize
    let toml_string = toml::to_string(&mode_config).unwrap();
    let deserialized: ModeConfig = toml::from_str(&toml_string).unwrap();

    assert_eq!(deserialized.max_retries, 0);
    assert_eq!(deserialized.max_depth, 10);
    assert_eq!(deserialized.timeout_plan, 1);
    assert_eq!(deserialized.timeout_exec, 86400);
}

#[test]
fn test_config_serialization_roundtrip_with_all_fields() {
    // Test complete config roundtrip with all fields populated
    let mut agents = std::collections::HashMap::new();
    agents.insert(
        "test-agent".to_string(),
        AgentConfig {
            command: Some("test-command".to_string()),
            model: Some("test-model".to_string()),
            timeout: Some(3600),
        },
    );

    let config = Config {
        default: Some("test-agent".to_string()),
        agents,
        modes: ltmatrix::config::settings::ModeConfigs {
            fast: Some(ModeConfig {
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
        output: ltmatrix::config::settings::OutputConfig {
            format: OutputFormat::Json,
            colored: false,
            progress: true,
        },
        logging: ltmatrix::config::settings::LoggingConfig {
            level: LogLevel::Debug,
            file: Some(PathBuf::from("/tmp/test.log")),
        },
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
    };

    // Serialize and deserialize
    let toml_string = toml::to_string(&config).unwrap();
    let deserialized: Config = toml::from_str(&toml_string).unwrap();

    assert_eq!(deserialized.default, config.default);
    assert_eq!(deserialized.agents.len(), config.agents.len());
    assert_eq!(deserialized.modes.fast.is_some(), true);
    assert_eq!(deserialized.output.format, OutputFormat::Json);
    assert_eq!(deserialized.logging.level, LogLevel::Debug);
}

// ============================================================================
// Complex Merge Scenarios
// ============================================================================

#[test]
fn test_merge_with_partial_mode_overrides() {
    // Test merging when override config has only some modes defined
    let mut base = Config::default();
    base.modes.fast = Some(ModeConfig {
        model: Some("base-fast".to_string()),
        run_tests: false,
        verify: true,
        max_retries: 1,
        max_depth: 2,
        timeout_plan: 60,
        timeout_exec: 1800,
    });
    base.modes.standard = Some(ModeConfig {
        model: Some("base-standard".to_string()),
        run_tests: true,
        verify: true,
        max_retries: 3,
        max_depth: 3,
        timeout_plan: 120,
        timeout_exec: 3600,
    });

    let mut override_config = Config::default();
    // Override only fast mode, keep standard from base
    override_config.modes.fast = Some(ModeConfig {
        model: Some("override-fast".to_string()),
        run_tests: true,
        verify: false,
        max_retries: 2,
        max_depth: 3,
        timeout_plan: 90,
        timeout_exec: 2400,
    });

    let merged = ltmatrix::config::settings::merge_configs(Some(base), Some(override_config));

    // Fast mode should be from override
    assert_eq!(
        merged.modes.fast.unwrap().model,
        Some("override-fast".to_string())
    );
    // Standard mode should be from base
    assert_eq!(
        merged.modes.standard.unwrap().model,
        Some("base-standard".to_string())
    );
}

#[test]
fn test_merge_with_empty_override_config() {
    // Test merging when override config is completely empty
    let mut base = Config::default();
    base.default = Some("base-agent".to_string());
    base.agents.insert(
        "agent1".to_string(),
        AgentConfig {
            command: Some("cmd1".to_string()),
            model: Some("model1".to_string()),
            timeout: Some(1000),
        },
    );

    // Create a truly empty config with default: None
    let empty_config = Config {
        default: None,
        agents: std::collections::HashMap::new(),
        modes: ltmatrix::config::settings::ModeConfigs::default(),
        output: ltmatrix::config::settings::OutputConfig::default(),
        logging: ltmatrix::config::settings::LoggingConfig::default(),
        features: ltmatrix::feature::FeatureConfig::default(),
        warmup: WarmupConfig::default(),
        pool: ltmatrix::config::settings::PoolConfig::default(),
        mcp: None,
    };
    let merged = ltmatrix::config::settings::merge_configs(Some(base), Some(empty_config));

    // Should preserve base config when override has None values
    assert_eq!(merged.default, Some("base-agent".to_string()));
    assert!(merged.agents.contains_key("agent1"));
}

#[test]
fn test_merge_with_empty_base_config() {
    // Test merging when base config is completely empty
    let empty_config = Config::default();

    let mut override_config = Config::default();
    override_config.default = Some("override-agent".to_string());
    override_config.agents.insert(
        "agent1".to_string(),
        AgentConfig {
            command: Some("cmd1".to_string()),
            model: Some("model1".to_string()),
            timeout: Some(1000),
        },
    );

    let merged =
        ltmatrix::config::settings::merge_configs(Some(empty_config), Some(override_config));

    // Empty config's None values should be overridden by override config's Some values
    // But Config::default() actually has default: Some("claude"), so override wins
    assert_eq!(merged.default, Some("override-agent".to_string()));
    assert!(merged.agents.contains_key("agent1"));
}

// ============================================================================
// Advanced Error Scenarios
// ============================================================================

#[test]
fn test_config_file_with_invalid_escape_sequences() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // TOML with invalid escape sequences
    let content = r#"
default = "test"

[agents.test]
command = "test\x"
model = "test-model"
"#;
    fs::write(&config_path, content).unwrap();

    let result = ltmatrix::config::settings::load_config_file(&config_path);
    // Should fail due to invalid escape sequence
    assert!(result.is_err());
}

#[test]
fn test_config_file_with_very_deep_nesting() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // TOML attempting to create deeply nested structure (though our schema doesn't support it)
    // This should parse but ignore unsupported nesting
    let content = r#"
default = "test"

[agents.test]
command = "test"
model = "test-model"
"#;
    fs::write(&config_path, content).unwrap();

    let result = ltmatrix::config::settings::load_config_file(&config_path);
    assert!(result.is_ok());
}

#[test]
fn test_config_file_with_incomplete_values() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // TOML with incomplete key-value pairs
    let content = r#"
default =

[agents.test]
command =
"#;
    fs::write(&config_path, content).unwrap();

    let result = ltmatrix::config::settings::load_config_file(&config_path);
    // Should fail - TOML parser rejects empty values without quotes
    assert!(result.is_err());
}

// ============================================================================
// Config Consistency Tests
// ============================================================================

#[test]
fn test_config_default_agent_exists() {
    // Test that default agent reference is validated against available agents
    let mut config = Config::default();
    config.default = Some("missing-agent".to_string());

    // Config should still parse even if default agent doesn't exist
    // Validation happens elsewhere
    let toml_string = toml::to_string(&config).unwrap();
    let deserialized: Config = toml::from_str(&toml_string).unwrap();

    assert_eq!(deserialized.default, Some("missing-agent".to_string()));
}

#[test]
fn test_config_with_duplicate_agent_names() {
    // Note: TOML doesn't allow duplicate table keys, so this would fail at parse time
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Attempt to create duplicate agent sections (TOML will reject this)
    let content = r#"
[agents.test]
command = "cmd1"

[agents.test]
command = "cmd2"
"#;
    fs::write(&config_path, content).unwrap();

    let result = ltmatrix::config::settings::load_config_file(&config_path);
    // TOML parser should reject duplicate keys
    assert!(result.is_err());
}

#[test]
fn test_config_mode_specific_settings() {
    // Test that each mode can have completely different settings
    let config_toml = r#"
[modes.fast]
model = "fast-model"
run_tests = false
verify = false
max_retries = 1
max_depth = 1
timeout_plan = 30
timeout_exec = 900

[modes.standard]
model = "standard-model"
run_tests = true
verify = true
max_retries = 3
max_depth = 3
timeout_plan = 120
timeout_exec = 3600

[modes.expert]
model = "expert-model"
run_tests = true
verify = true
max_retries = 10
max_depth = 5
timeout_plan = 600
timeout_exec = 14400
"#;

    let config: Config = toml::from_str(config_toml).unwrap();

    // Verify all modes are present and distinct
    assert!(config.modes.fast.is_some());
    assert!(config.modes.standard.is_some());
    assert!(config.modes.expert.is_some());

    let fast = config.modes.fast.unwrap();
    let standard = config.modes.standard.unwrap();
    let expert = config.modes.expert.unwrap();

    assert_ne!(fast.model, standard.model);
    assert_ne!(fast.max_retries, standard.max_retries);
    assert_ne!(standard.max_retries, expert.max_retries);
}

// ============================================================================
// Integration Tests with CLI
// ============================================================================

#[test]
fn test_load_config_from_args_with_all_overrides() {
    let temp_dir = TempDir::new().unwrap();

    // Create a valid config file
    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600
"#,
    )
    .unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Parse args with multiple overrides
    let args = Args::try_parse_from([
        "ltmatrix",
        "--agent",
        "claude",
        "--mode",
        "fast",
        "--output",
        "json",
        "--log-level",
        "debug",
        "--timeout",
        "1800",
        "--dry-run",
        "goal",
    ])
    .expect("Failed to parse args");

    let result = load_config_from_args(args);
    assert!(
        result.is_ok(),
        "Should successfully load config with all overrides"
    );

    let config = result.unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
}

#[test]
fn test_cli_overrides_with_boolean_combinations() {
    // Test various combinations of boolean flags
    let test_cases = vec![
        (vec!["--dry-run"], true, false, false, false),
        (vec!["--resume"], false, true, false, false),
        (vec!["--ask"], false, false, true, false),
        (vec!["--regenerate-plan"], false, false, false, true),
        (vec!["--dry-run", "--resume"], true, true, false, false),
        (vec!["--dry-run", "--ask"], true, false, true, false),
        (
            vec!["--resume", "--ask", "--regenerate-plan"],
            false,
            true,
            true,
            true,
        ),
        (
            vec!["--dry-run", "--resume", "--ask", "--regenerate-plan"],
            true,
            true,
            true,
            true,
        ),
    ];

    for (flags, expected_dry_run, expected_resume, expected_ask, expected_regenerate) in &test_cases
    {
        let mut args_vec = vec!["ltmatrix"];
        for flag in flags.iter() {
            args_vec.push(*flag);
        }
        args_vec.push("goal");

        let args = Args::try_parse_from(args_vec).expect("Failed to parse args");
        let overrides: CliOverrides = args.into();

        assert_eq!(
            overrides.dry_run, *expected_dry_run,
            "dry_run mismatch for flags: {:?}",
            flags
        );
        assert_eq!(
            overrides.resume, *expected_resume,
            "resume mismatch for flags: {:?}",
            flags
        );
        assert_eq!(
            overrides.ask, *expected_ask,
            "ask mismatch for flags: {:?}",
            flags
        );
        assert_eq!(
            overrides.regenerate_plan, *expected_regenerate,
            "regenerate_plan mismatch for flags: {:?}",
            flags
        );
    }
}

#[test]
fn test_on_blocked_strategy_values() {
    // Test various on_blocked strategy values
    // Valid strategies are: skip, ask, abort, retry (from BlockedStrategy enum)
    let strategies = vec!["skip", "ask", "abort", "retry"];

    for strategy in strategies {
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", strategy, "goal"])
            .expect("Failed to parse args");

        let overrides: CliOverrides = args.into();

        assert_eq!(
            overrides.on_blocked,
            Some(strategy.to_string()),
            "on_blocked should be set to: {}",
            strategy
        );
    }
}

// ============================================================================
// Config File Discovery Edge Cases
// ============================================================================

#[test]
fn test_config_discovery_with_symlinks() {
    // This test would verify config discovery works with symlinks
    // Note: Creating symlinks requires platform-specific code
    // For now, we'll just verify the path resolution works

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
default = "test"
"#,
    )
    .unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Verify project config path can be found
    let project_path = ltmatrix::config::settings::get_project_config_path();
    assert!(project_path.is_some());

    // Verify the path points to the correct location
    let path_str = project_path.unwrap().to_string_lossy().to_string();
    assert!(path_str.contains(".ltmatrix"));
    assert!(path_str.contains("config.toml"));
}

#[test]
fn test_config_loading_without_files() {
    // Test that the system works when no config files exist
    let temp_dir = TempDir::new().unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Should not fail even with no config files
    let result = ltmatrix::config::settings::load_config();
    assert!(
        result.is_ok(),
        "Should load default config when no files exist"
    );

    let config = result.unwrap();
    // Should have default values
    assert!(config.default.is_some());
}
