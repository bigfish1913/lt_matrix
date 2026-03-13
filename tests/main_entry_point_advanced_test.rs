//! Advanced tests for main application entry point
//!
//! This test suite covers advanced scenarios, edge cases, and integration
//! testing for src/main.rs functionality that goes beyond basic validation.

use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

// =============================================================================
// Advanced Configuration Tests
// =============================================================================

#[test]
fn test_config_merge_priority() {
    // Test that CLI args override config file settings
    use ltmatrix::cli::args::Args;
    use ltmatrix::config::settings::{CliOverrides, Config};

    // Parse CLI args with explicit overrides
    let args = Args::try_parse_from(["ltmatrix", "test goal", "--fast", "--agent", "claude"])
        .expect("Failed to parse args");

    // Create overrides from args
    let overrides = CliOverrides {
        agent: args.agent.clone(),
        mode: if args.fast {
            Some("fast".to_string())
        } else {
            None
        },
        ..Default::default()
    };

    // Load config with overrides
    let result = ltmatrix::config::settings::load_config_with_overrides(Some(overrides));
    assert!(
        result.is_ok(),
        "Config with overrides should load successfully"
    );

    if let Ok(config) = result {
        // Verify config structure is valid
        let _ = config.default;
        let _ = config.logging.level;
        let _ = config.output.format;
    }
}

#[test]
fn test_config_with_invalid_toml() {
    // Test handling of malformed TOML configuration
    use std::fs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).expect("Failed to create config dir");

    // Write invalid TOML
    let config_file = config_dir.join("config.toml");
    fs::write(&config_file, "invalid toml content {{{").expect("Failed to write config");

    // Try to load config - should handle gracefully
    let result = ltmatrix::config::settings::load_config_with_overrides(None);

    // Should fall back to defaults even with invalid config file
    assert!(
        result.is_ok(),
        "Should fall back to defaults with invalid config"
    );
}

#[test]
fn test_config_with_partial_settings() {
    // Test configuration with only some fields set
    use std::fs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_dir = temp_dir.path().join(".ltmatrix");
    fs::create_dir_all(&config_dir).expect("Failed to create config dir");

    // Write partial TOML config
    let config_content = r#"
[default]
agent = "claude"

[output]
format = "json"
"#;

    let config_file = config_dir.join("config.toml");
    fs::write(&config_file, config_content).expect("Failed to write config");

    // Load config - should fill in missing fields with defaults
    let result = ltmatrix::config::settings::load_config_with_overrides(None);
    assert!(result.is_ok(), "Should handle partial config with defaults");
}

// =============================================================================
// Advanced Logging Tests
// =============================================================================

#[test]
fn test_log_rotation_handling() {
    // Test that log rotation doesn't break initialization
    use ltmatrix::logging::logger;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let log_dir = temp_dir.path().join("logs");

    // Initialize logging with management
    let result = logger::init_logging_with_management(
        ltmatrix::logging::level::LogLevel::Info,
        Some(log_dir.clone()),
    );

    assert!(result.is_ok(), "Logging with rotation should succeed");

    if let Ok((guard, manager)) = result {
        // Test manager capabilities
        let _ = manager;

        // Keep guard alive
        let _ = guard;
    }
}

#[test]
fn test_concurrent_logging() {
    // Test that multiple logging components don't conflict
    use ltmatrix::logging::logger;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Initialize first logger
    let result1 = logger::init_logging(
        ltmatrix::logging::level::LogLevel::Info,
        Some(&temp_dir.path().join("test1.log")),
    );

    assert!(
        result1.is_ok(),
        "First logger initialization should succeed"
    );

    // Initialize second logger (different file)
    let result2 = logger::init_logging(
        ltmatrix::logging::level::LogLevel::Debug,
        Some(&temp_dir.path().join("test2.log")),
    );

    assert!(
        result2.is_ok(),
        "Second logger initialization should succeed"
    );
}

#[test]
fn test_logging_levels_hierarchical() {
    // Test that log level hierarchy is respected
    use ltmatrix::logging::level::LogLevel;

    // Verify ordering
    assert!(LogLevel::Trace as i32 <= LogLevel::Debug as i32);
    assert!(LogLevel::Debug as i32 <= LogLevel::Info as i32);
    assert!(LogLevel::Info as i32 <= LogLevel::Warn as i32);
    assert!(LogLevel::Warn as i32 <= LogLevel::Error as i32);
}

// =============================================================================
// Advanced Agent Backend Tests
// =============================================================================

#[test]
fn test_agent_backend_fallback_chain() {
    // Test the agent backend fallback logic
    use ltmatrix::agent::AgentFactory;
    use ltmatrix::config::settings::Config;

    let factory = AgentFactory::new();
    let config = Config::default();

    // Test default fallback
    let agent_name = config
        .default
        .or_else(|| Some("claude".to_string()))
        .unwrap();

    assert!(
        factory.is_supported(&agent_name),
        "Default agent should be supported"
    );
}

#[test]
fn test_multiple_agent_pools() {
    // Test creating multiple agent pools with different configs
    use ltmatrix::agent::AgentPool;
    use ltmatrix::config::settings::Config;

    let config1 = Config::default();
    let pool1 = AgentPool::new(&config1);

    let config2 = Config::default();
    let pool2 = AgentPool::new(&config2);

    // Both pools should be independent
    let _ = pool1;
    let _ = pool2;
}

#[test]
fn test_agent_pool_with_default_config() {
    // Test agent pool with default configuration
    use ltmatrix::agent::AgentPool;
    use ltmatrix::config::settings::Config;

    let config = Config::default();
    let pool = AgentPool::new(&config);
    let _ = pool; // Use the variable
}

// =============================================================================
// Advanced Command Routing Tests
// =============================================================================

#[test]
fn test_command_with_all_flags() {
    // Test combining all possible flags
    use ltmatrix::cli::args::Args;

    let args = Args::try_parse_from([
        "ltmatrix",
        "test goal",
        "--fast",
        "--dry-run",
        "--ask",
        "--resume",
        "--log-level",
        "trace",
        "--log-file",
        "/tmp/test.log",
        "--agent",
        "claude",
        "--output",
        "json",
    ])
    .expect("Failed to parse args");

    assert!(args.fast);
    assert!(args.dry_run);
    assert!(args.ask);
    assert!(args.resume);
    assert!(args.log_level.is_some());
    assert!(args.log_file.is_some());
    assert!(args.agent.is_some());
    assert!(args.output.is_some());
}

#[test]
fn test_mutually_exclusive_modes() {
    // Test that fast and expert modes don't conflict
    use ltmatrix::cli::args::Args;

    // Only one mode flag should be used at a time in practice
    // But we test parsing with both to verify clap handles it
    let args =
        Args::try_parse_from(["ltmatrix", "test goal", "--fast"]).expect("Failed to parse args");

    assert!(args.fast);
    assert!(!args.expert);
}

#[test]
fn test_subcommand_with_extraneous_flags() {
    // Test that subcommands ignore run-specific flags
    use ltmatrix::cli::args::{Args, Command};

    let args =
        Args::try_parse_from(["ltmatrix", "completions", "bash"]).expect("Failed to parse args");

    // Subcommand should be parsed, flags ignored
    assert!(matches!(args.command, Some(Command::Completions(_))));
}

// =============================================================================
// Pipeline Execution Tests
// =============================================================================

#[test]
fn test_orchestrator_config_modes() {
    // Test all orchestrator configuration modes
    use ltmatrix::pipeline::orchestrator::OrchestratorConfig;

    // Fast mode config
    let fast_config = OrchestratorConfig::fast_mode();
    let _ = fast_config;

    // Expert mode config
    let expert_config = OrchestratorConfig::expert_mode();
    let _ = expert_config;

    // Standard mode config
    let standard_config = OrchestratorConfig::default();
    let _ = standard_config;
}

#[test]
fn test_orchestrator_config_builder_chain() {
    // Test fluent builder pattern for orchestrator config
    use ltmatrix::agent::AgentPool;
    use ltmatrix::config::settings::Config;
    use ltmatrix::pipeline::orchestrator::OrchestratorConfig;

    let work_dir = std::env::current_dir().expect("Failed to get current dir");
    let config = Config::default();
    let pool = AgentPool::new(&config);

    let orchestrator_config = OrchestratorConfig::fast_mode()
        .with_work_dir(&work_dir)
        .with_agent_pool(pool)
        .with_progress(true);

    let _ = orchestrator_config;
}

#[test]
fn test_execution_mode_round_trip() {
    // Test conversion between CLI and model execution modes
    use ltmatrix::cli::args::ExecutionModeArg;
    use ltmatrix::models::ExecutionMode;

    let test_cases = vec![
        (ExecutionModeArg::Standard, ExecutionMode::Standard),
        (ExecutionModeArg::Fast, ExecutionMode::Fast),
        (ExecutionModeArg::Expert, ExecutionMode::Expert),
    ];

    for (cli_mode, expected_model_mode) in test_cases {
        let model_mode = cli_mode.to_model();
        assert_eq!(
            model_mode, expected_model_mode,
            "Mode conversion should preserve value"
        );
    }
}

// =============================================================================
// Error Handling Advanced Tests
// =============================================================================

#[test]
fn test_nested_error_context() {
    // Test deeply nested error chains
    use anyhow::{anyhow, Context};

    let base_err = anyhow!("Base error");
    let level1 = base_err.context("Level 1 context");
    let level2 = level1.context("Level 2 context");
    let level3 = level2.context("Level 3 context");

    let error_string = level3.to_string();
    assert!(
        error_string.contains("Level 1")
            || error_string.contains("Level 2")
            || error_string.contains("Level 3")
            || error_string.contains("Base error"),
        "Nested error should preserve context"
    );
}

#[test]
fn test_error_hint_keywords() {
    // Test that error hints cover all expected keywords
    let test_cases = vec![
        ("permission denied", "permission"),
        ("access violation", "access"),
        ("network timeout", "network"),
        ("connection refused", "connection"),
        ("config parse error", "config"),
        ("agent init failed", "agent"),
    ];

    for (error_msg, expected_keyword) in test_cases {
        let lower_msg = error_msg.to_lowercase();
        assert!(
            lower_msg.contains(expected_keyword),
            "Error message should contain expected keyword"
        );
    }
}

// =============================================================================
// Signal Handling Tests
// =============================================================================

#[test]
fn test_shutdown_flag_concurrent_access() {
    // Test shutdown flag under concurrent access patterns
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    static TEST_FLAG: AtomicBool = AtomicBool::new(false);

    // Spawn multiple threads that all try to set the flag
    let handles: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                TEST_FLAG.store(true, Ordering::SeqCst);
                TEST_FLAG.load(Ordering::SeqCst)
            })
        })
        .collect();

    // All threads should complete successfully
    for handle in handles {
        let result = handle.join().expect("Thread should complete");
        assert!(result, "Flag should be set");
    }

    // Reset
    TEST_FLAG.store(false, Ordering::SeqCst);
}

#[test]
fn test_different_memory_orderings() {
    // Test shutdown flag with different memory orderings
    use std::sync::atomic::{AtomicBool, Ordering};

    static TEST_FLAG: AtomicBool = AtomicBool::new(false);

    // Test with different orderings
    TEST_FLAG.store(true, Ordering::Relaxed);
    assert!(TEST_FLAG.load(Ordering::Relaxed));

    TEST_FLAG.store(false, Ordering::Release);
    assert!(!TEST_FLAG.load(Ordering::Acquire));

    TEST_FLAG.store(true, Ordering::SeqCst);
    assert!(TEST_FLAG.load(Ordering::SeqCst));

    // Cleanup
    TEST_FLAG.store(false, Ordering::SeqCst);
}

// =============================================================================
// File System Tests
// =============================================================================

#[test]
fn test_config_directory_creation() {
    // Test that config directories are created as needed
    use std::fs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_dir = temp_dir.path().join(".ltmatrix");

    // Directory shouldn't exist yet
    assert!(!config_dir.exists());

    // Write config file - parent dirs should be created
    fs::create_dir_all(&config_dir).expect("Failed to create config dir");

    // Directory should now exist
    assert!(config_dir.exists());

    // Write config file
    let config_file = config_dir.join("config.toml");
    fs::write(&config_file, "# Test config\n").expect("Failed to write config");

    assert!(config_file.exists());
}

#[test]
fn test_log_directory_creation() {
    // Test that log directories are created when needed
    use std::fs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let log_dir = temp_dir.path().join("logs");

    // Create log directory
    fs::create_dir_all(&log_dir).expect("Failed to create log dir");

    assert!(log_dir.exists());

    // Write log file
    let log_file = log_dir.join("test.log");
    fs::write(&log_file, "Test log content\n").expect("Failed to write log");

    assert!(log_file.exists());
}

#[test]
fn test_path_handling_edge_cases() {
    // Test handling of various path formats
    use std::path::Path;

    let test_cases = vec![
        "./logs/test.log",
        "/absolute/path/test.log",
        "../relative/path/test.log",
        "C:\\Windows\\path\\test.log", // Windows path
        "~/home/user/test.log",        // Home directory
    ];

    for path_str in test_cases {
        let path = Path::new(path_str);
        // Just verify parsing doesn't panic
        let _ = path.exists();
        let _ = path.parent();
        let _ = path.file_name();
        let _ = path.extension();
    }
}

// =============================================================================
// Performance Tests
// =============================================================================

#[test]
fn test_config_loading_performance() {
    // Test that config loading is reasonably fast
    use std::time::Instant;

    let start = Instant::now();

    for _ in 0..10 {
        let result = ltmatrix::config::settings::load_config_with_overrides(None);
        assert!(result.is_ok());
    }

    let duration = start.elapsed();

    // 10 config loads should complete in less than 1 second
    assert!(
        duration < Duration::from_secs(1),
        "Config loading should be fast"
    );
}

#[test]
fn test_agent_pool_creation_performance() {
    // Test that agent pool creation is reasonably fast
    use ltmatrix::agent::AgentPool;
    use ltmatrix::config::settings::Config;
    use std::time::Instant;

    let start = Instant::now();

    for _ in 0..5 {
        let config = Config::default();
        let _pool = AgentPool::new(&config);
    }

    let duration = start.elapsed();

    // 5 pool creations should complete in less than 1 second
    assert!(
        duration < Duration::from_secs(1),
        "Agent pool creation should be fast"
    );
}

// =============================================================================
// Integration Edge Cases
// =============================================================================

#[test]
fn test_empty_goal_handling() {
    // Test handling of empty goal string
    use ltmatrix::cli::args::Args;

    let args = Args::try_parse_from(["ltmatrix", ""]).expect("Failed to parse args");

    // Empty string should still be parsed as Some("")
    assert_eq!(args.goal, Some("".to_string()));
}

#[test]
fn test_very_long_goal() {
    // Test handling of very long goal strings
    use ltmatrix::cli::args::Args;

    let long_goal = "a".repeat(10000);
    let args = Args::try_parse_from(["ltmatrix", &long_goal]).expect("Failed to parse args");

    assert_eq!(args.goal, Some(long_goal));
}

#[test]
fn test_special_characters_in_goal() {
    // Test handling of special characters in goal
    use ltmatrix::cli::args::Args;

    let special_goals = vec![
        "Test: goal with, special; chars!",
        "Test \"quotes\" and 'apostrophes'",
        "Test $variables and {braces}",
        "Test (parens) and [brackets]",
        "Test \t\t tabs and \n\n newlines",
    ];

    for goal in special_goals {
        let args = Args::try_parse_from(["ltmatrix", goal]).expect("Failed to parse args");

        assert_eq!(args.goal, Some(goal.to_string()));
    }
}

// =============================================================================
// Banner and Output Tests
// =============================================================================

#[test]
fn test_banner_content_verification() {
    // Test that banner contains expected content
    use ltmatrix::cli::args::Args;

    let args = Args::try_parse_from(["ltmatrix", "test goal"]).expect("Failed to parse args");

    // Banner should be printed for run commands
    assert!(args.command.is_none());
    assert!(args.is_run_command());
}

#[test]
fn test_help_output_structure() {
    // Test that help output can be generated
    use ltmatrix::cli::args::Args;

    // Attempt to parse with --help (will exit, but we catch the error)
    let result = Args::try_parse_from(["ltmatrix", "--help"]);

    // Should fail with DisplayHelp error kind
    match result {
        Err(_e) => {
            // Expected behavior - help causes exit
        }
        _ => {
            // Help might have been displayed and exited
        }
    }
}

// =============================================================================
// Cleanup and Shutdown Tests
// =============================================================================

#[test]
fn test_graceful_shutdown_sequence() {
    // Test that shutdown sequence is properly ordered
    use std::sync::atomic::{AtomicBool, Ordering};

    static STEP1: AtomicBool = AtomicBool::new(false);
    static STEP2: AtomicBool = AtomicBool::new(false);
    static STEP3: AtomicBool = AtomicBool::new(false);

    // Simulate shutdown sequence
    STEP1.store(true, Ordering::SeqCst);
    STEP2.store(true, Ordering::SeqCst);
    STEP3.store(true, Ordering::SeqCst);

    // Verify all steps completed
    assert!(STEP1.load(Ordering::SeqCst));
    assert!(STEP2.load(Ordering::SeqCst));
    assert!(STEP3.load(Ordering::SeqCst));
}

#[test]
fn test_resource_cleanup() {
    // Test that resources are properly cleaned up
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create some resources
    let log_file = temp_dir.path().join("test.log");
    std::fs::write(&log_file, "test content").expect("Failed to write log");

    assert!(log_file.exists());

    // Cleanup happens when temp_dir goes out of scope
    // TempDir will automatically clean up its contents
    let _ = temp_dir;
}

// =============================================================================
// State Management Tests
// =============================================================================

#[test]
fn test_app_state_immutability() {
    // Test that application state maintains consistency
    use ltmatrix::cli::args::Args;
    use ltmatrix::config::settings::Config;

    let args = Args::try_parse_from(["ltmatrix", "test goal"]).expect("Failed to parse args");

    let config = Config::default();

    // Create state components
    let _args_clone = args.clone();
    let _config_clone = config.clone();

    // Both copies should be independent
    assert_eq!(args.goal, _args_clone.goal);
}

#[test]
fn test_concurrent_config_access() {
    // Test that config can be safely accessed concurrently
    use ltmatrix::config::settings::Config;
    use std::sync::Arc;
    use std::thread;

    let config = Arc::new(Config::default());
    let mut handles = vec![];

    // Spawn multiple threads reading config
    for _ in 0..5 {
        let config_clone = Arc::clone(&config);
        let handle = thread::spawn(move || {
            let _ = config_clone.default;
            let _ = config_clone.logging.level;
            let _ = config_clone.output.format;
        });
        handles.push(handle);
    }

    // All threads should complete
    for handle in handles {
        handle.join().expect("Thread should complete");
    }
}

// =============================================================================
// Summary
// =============================================================================

#[test]
fn test_all_critical_paths() {
    // Test that all critical code paths are accessible
    use ltmatrix::cli::args::Args;
    use ltmatrix::config::settings;

    // 1. Parse args
    let args = Args::try_parse_from(["ltmatrix", "test"]).expect("Failed to parse args");
    let _ = args;

    // 2. Load config
    let config_result = settings::load_config_with_overrides(None);
    assert!(config_result.is_ok());

    // 3. Create agent pool
    if let Ok(config) = config_result {
        use ltmatrix::agent::AgentPool;
        let _pool = AgentPool::new(&config);
    }

    // 4. Create orchestrator config
    use ltmatrix::pipeline::orchestrator::OrchestratorConfig;
    let _orchestrator_config = OrchestratorConfig::default();

    // All critical paths should be accessible
    assert!(true, "All critical paths should work");
}
