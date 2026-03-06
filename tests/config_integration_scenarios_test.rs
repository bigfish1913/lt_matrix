//! Integration and real-world scenario tests for config system
//!
//! This test suite focuses on:
//! - Complete end-to-end configuration workflows
//! - Real-world usage patterns and scenarios
//! - Cross-platform compatibility
//! - Performance with realistic config sizes
//! - Config system reliability under various conditions
//!
//! These tests ensure the config system works correctly in production scenarios.

use clap::Parser;
use ltmatrix::cli::Args;
use ltmatrix::config::settings::{
    load_config, load_config_file, load_config_from_args, load_config_with_overrides,
    merge_configs, CliOverrides, Config, LogLevel, OutputFormat, WarmupConfig,
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
// Real-World Scenario: Development vs Production
// ============================================================================

#[test]
fn test_scenario_development_workflow() {
    // Simulate a development environment with detailed logging and fast feedback
    let temp_dir = TempDir::new().unwrap();

    let dev_config = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[modes.fast]
model = "claude-haiku-4-5"
run_tests = false
max_retries = 1
max_depth = 2
timeout_plan = 30
timeout_exec = 900

[output]
format = "text"
colored = true
progress = true

[logging]
level = "debug"
"#;

    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, dev_config).unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    let result = load_config();
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.logging.level, LogLevel::Debug);
    assert_eq!(config.output.colored, true);
    assert_eq!(config.modes.fast.as_ref().unwrap().run_tests, false);
}

#[test]
fn test_scenario_production_workflow() {
    // Simulate a production environment with conservative settings
    let temp_dir = TempDir::new().unwrap();

    let prod_config = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 7200

[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
verify = true
max_retries = 5
max_depth = 3
timeout_plan = 300
timeout_exec = 7200

[output]
format = "json"
colored = false
progress = false

[logging]
level = "info"
file = "/var/log/ltmatrix/production.log"
"#;

    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, prod_config).unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    let result = load_config();
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.logging.level, LogLevel::Info);
    assert_eq!(config.output.colored, false);
    assert_eq!(config.modes.standard.as_ref().unwrap().run_tests, true);
    assert!(config.logging.file.is_some());
}

// ============================================================================
// Real-World Scenario: Team Collaboration
// ============================================================================

#[test]
fn test_scenario_team_shared_config_with_local_overrides() {
    // Simulate team setup with shared global config and local project overrides
    let temp_dir = TempDir::new().unwrap();

    // Shared global config
    let global_config = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[agents.opencode]
command = "opencode"
model = "gpt-4"
timeout = 1800

[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
verify = true
max_retries = 3
max_depth = 3
timeout_plan = 120
timeout_exec = 3600

[output]
format = "text"
colored = true

[logging]
level = "info"
"#;

    let global_dir = temp_dir.path().join("home").join(".ltmatrix");
    fs::create_dir_all(&global_dir).unwrap();
    let global_path = global_dir.join("config.toml");
    fs::write(&global_path, global_config).unwrap();

    // Local project overrides
    let project_config = r#"
[agents.claude]
model = "claude-opus-4-6"
timeout = 7200

[output]
colored = false

[logging]
level = "debug"
file = "/tmp/project-debug.log"
"#;

    let project_dir = temp_dir.path().join("project").join(".ltmatrix");
    fs::create_dir_all(&project_dir).unwrap();
    let project_path = project_dir.join("config.toml");
    fs::write(&project_path, project_config).unwrap();

    // Load and merge both configs
    let global = load_config_file(&global_path).unwrap();
    let project = load_config_file(&project_path).unwrap();
    let merged = merge_configs(Some(global), Some(project));

    // Verify merge behavior
    assert_eq!(
        merged.agents["claude"].model,
        Some("claude-opus-4-6".to_string())
    );
    assert_eq!(merged.agents["claude"].timeout, Some(7200));
    assert_eq!(merged.agents["opencode"].model, Some("gpt-4".to_string())); // From global
    assert_eq!(merged.output.colored, false); // Overridden by project
    assert_eq!(merged.logging.level, LogLevel::Debug); // Overridden by project
    assert_eq!(
        merged.logging.file,
        Some(PathBuf::from("/tmp/project-debug.log"))
    );
}

// ============================================================================
// Real-World Scenario: CI/CD Pipeline
// ============================================================================

#[test]
fn test_scenario_ci_cd_environment() {
    // Simulate CI/CD environment with no-color and JSON output
    let temp_dir = TempDir::new().unwrap();

    let ci_config = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[output]
format = "json"
colored = false
progress = false

[logging]
level = "warn"
file = "/var/log/ci/ltmatrix.log"
"#;

    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, ci_config).unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Simulate CI environment with CLI override
    let args = Args::try_parse_from(["ltmatrix", "--no-color", "--output", "json", "goal"])
        .expect("Failed to parse args");

    let overrides: CliOverrides = args.into();
    let result = load_config_with_overrides(Some(overrides));

    assert!(result.is_ok());
    let config = result.unwrap();

    assert_eq!(config.output.format, OutputFormat::Json);
    assert_eq!(config.output.colored, false);
    assert_eq!(config.logging.level, LogLevel::Warn);
}

// ============================================================================
// Cross-Platform Compatibility Tests
// ============================================================================

#[test]
fn test_windows_style_paths() {
    let temp_dir = TempDir::new().unwrap();

    // Windows paths in TOML need double backslashes or forward slashes
    let config_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"

[logging]
file = "C:\\Users\\test\\AppData\\Local\\ltmatrix\\log.txt"
"#;

    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = load_config_file(&config_path);
    // Windows paths in TOML need proper escaping
    // This test verifies the system handles path strings correctly
    assert!(result.is_ok());
}

#[test]
fn test_unix_style_paths() {
    let temp_dir = TempDir::new().unwrap();

    let config_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"

[logging]
file = "/home/user/.local/share/ltmatrix/log.txt"
"#;

    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(
        config.logging.file,
        Some(PathBuf::from("/home/user/.local/share/ltmatrix/log.txt"))
    );
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_large_config_file_performance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Create a config file with many agents and modes
    let mut content = String::from("default = \"agent0\"\n\n");

    // Add 100 agents
    for i in 0..100 {
        content.push_str(&format!(
            r#"
[agents.agent{:02}]
command = "command{:02}"
model = "model-{:02}"
timeout = {}
"#,
            i,
            i,
            i,
            (i + 1) * 100
        ));
    }

    // Add all three modes with extensive configuration
    content.push_str(
        r#"
[modes.fast]
model = "fast-model"
run_tests = false
verify = true
max_retries = 1
max_depth = 2
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
"#,
    );

    fs::write(&config_path, content).unwrap();

    // Measure load time
    let start = std::time::Instant::now();
    let result = load_config_file(&config_path);
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(
        elapsed.as_millis() < 1000,
        "Loading large config should be fast (< 1s)"
    );

    let config = result.unwrap();
    assert_eq!(config.agents.len(), 100);
    assert!(config.modes.fast.is_some());
    assert!(config.modes.standard.is_some());
    assert!(config.modes.expert.is_some());
}

// ============================================================================
// Config System Reliability Tests
// ============================================================================

#[test]
fn test_config_loading_with_concurrent_reads() {
    // Test that config loading is thread-safe
    let temp_dir = TempDir::new().unwrap();

    let config_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600
"#;

    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    // Spawn multiple threads that all read the same config
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let path = config_path.clone();
            std::thread::spawn(move || load_config_file(&path))
        })
        .collect();

    // All threads should successfully read the config
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }
}

#[test]
fn test_config_serialization_idempotence() {
    // Test that serialize -> deserialize -> serialize produces the same result
    let original_config = Config {
        default: Some("claude".to_string()),
        agents: {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "claude".to_string(),
                ltmatrix::config::settings::AgentConfig {
                    command: Some("claude".to_string()),
                    model: Some("claude-sonnet-4-6".to_string()),
                    timeout: Some(3600),
                },
            );
            map
        },
        modes: ltmatrix::config::settings::ModeConfigs {
            fast: Some(ltmatrix::config::settings::ModeConfig {
                model: Some("fast".to_string()),
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
    };

    // First serialization
    let toml1 = toml::to_string(&original_config).unwrap();

    // Deserialize and serialize again
    let config1: Config = toml::from_str(&toml1).unwrap();
    let toml2 = toml::to_string(&config1).unwrap();

    // Deserialize again
    let config2: Config = toml::from_str(&toml2).unwrap();

    // All should be equivalent
    assert_eq!(config1.default, config2.default);
    assert_eq!(config1.agents.len(), config2.agents.len());
    assert_eq!(config1.output.format, config2.output.format);
}

// ============================================================================
// Complete Workflow Tests
// ============================================================================

#[test]
fn test_complete_workflow_new_project_setup() {
    // Test the complete workflow of setting up a new project
    let temp_dir = TempDir::new().unwrap();
    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Step 1: No config exists - should use defaults
    let result = load_config();
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.default, Some("claude".to_string()));

    // Step 2: Create a minimal project config
    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");

    fs::write(
        &config_path,
        r#"
default = "my-agent"

[agents.my-agent]
command = "my-agent"
model = "my-model"
timeout = 1800
"#,
    )
    .unwrap();

    // Step 3: Reload and verify new config is used
    let result = load_config();
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.default, Some("my-agent".to_string()));
    assert!(config.agents.contains_key("my-agent"));
}

#[test]
fn test_complete_workflow_cli_override_workflow() {
    // Test typical workflow: project config + CLI overrides
    let temp_dir = TempDir::new().unwrap();

    // Setup project config
    let config_content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600

[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
verify = true
max_retries = 3
timeout_plan = 120
timeout_exec = 3600

[output]
format = "text"
colored = true

[logging]
level = "info"
"#;

    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    let _guard = DirGuard::new();
    _guard.change_to(temp_dir.path());

    // Apply CLI overrides for specific run
    let args = Args::try_parse_from([
        "ltmatrix",
        "--mode",
        "fast",
        "--output",
        "json",
        "--log-level",
        "debug",
        "--timeout",
        "1800",
        "implement feature",
    ])
    .expect("Failed to parse args");

    let result = load_config_from_args(args);
    assert!(result.is_ok());

    let config = result.unwrap();
    // Verify CLI overrides are applied
    assert_eq!(config.default, Some("claude".to_string())); // From file
    assert_eq!(config.output.format, OutputFormat::Json); // CLI override
    assert_eq!(config.logging.level, LogLevel::Debug); // CLI override
}

// ============================================================================
// Config File Edge Cases
// ============================================================================

#[test]
fn test_config_with_mixed_line_endings() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Mix CRLF and LF line endings
    let content =
        "default = \"test\"\r\n\r\n[agents.test]\r\ncommand = \"test\"\nmodel = \"test-model\"\r\n";
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("test".to_string()));
}

#[test]
fn test_config_with_unicode_bom() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // UTF-8 BOM followed by valid TOML
    let content =
        "\u{FEFF}default = \"test\"\n\n[agents.test]\ncommand = \"test\"\nmodel = \"test-model\"\n";
    fs::write(&config_path, content.as_bytes()).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("test".to_string()));
}

#[test]
fn test_config_with_trailing_whitespace() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
default = "test"

[agents.test]
command = "test"
model = "test-model"
"#;
    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("test".to_string()));
}

// ============================================================================
// Config Validation Integration Tests
// ============================================================================

#[test]
fn test_config_with_all_sections_populated() {
    // Test a complete config with all sections populated
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
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
timeout_plan = 30
timeout_exec = 900

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
max_retries = 10
max_depth = 5
timeout_plan = 600
timeout_exec = 14400

[output]
format = "json"
colored = true
progress = true

[logging]
level = "debug"
file = "/tmp/ltmatrix.log"
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.agents.len(), 2);
    assert!(config.modes.fast.is_some());
    assert!(config.modes.standard.is_some());
    assert!(config.modes.expert.is_some());
    assert_eq!(config.output.format, OutputFormat::Json);
    assert_eq!(config.logging.level, LogLevel::Debug);
}

#[test]
fn test_config_minimal_valid() {
    // Test the minimal valid config
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let content = r#"
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
"#;

    fs::write(&config_path, content).unwrap();

    let result = load_config_file(&config_path);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.default, Some("claude".to_string()));
    assert!(config.agents.contains_key("claude"));
}
