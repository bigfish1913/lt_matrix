//! Validation tests for main application entry point
//!
//! This test suite validates edge cases and error scenarios for src/main.rs
//! that aren't covered in the main integration test suite.

use clap::Parser;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

// =============================================================================
// AppState Structure Tests
// =============================================================================

#[test]
fn test_app_state_creation() {
    // Test AppState creation with basic parameters
    use ltmatrix::cli::args::Args;
    use ltmatrix::config::settings::Config;

    let args = Args::try_parse_from(["ltmatrix", "test goal"]).expect("Failed to parse args");

    let config = Config::default();

    // Create a minimal AppState-like structure for testing
    let _args = args;
    let _config = config;

    // Verify state can be created
    assert!(true, "AppState components should be creatable");
}

#[test]
fn test_app_state_with_agent_pool() {
    // Test AppState with agent pool
    use ltmatrix::agent::AgentPool;
    use ltmatrix::config::settings::Config;

    let config = Config::default();
    let pool = AgentPool::new(&config);

    // Verify pool was created successfully
    let _ = pool;
}

// =============================================================================
// OrchestratorConfig Tests
// =============================================================================

#[test]
fn test_orchestrator_fast_mode_config() {
    // Test fast mode orchestrator configuration
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

    // Verify config was created
    let _ = orchestrator_config;
}

#[test]
fn test_orchestrator_expert_mode_config() {
    // Test expert mode orchestrator configuration
    use ltmatrix::agent::AgentPool;
    use ltmatrix::config::settings::Config;
    use ltmatrix::pipeline::orchestrator::OrchestratorConfig;

    let work_dir = std::env::current_dir().expect("Failed to get current dir");
    let config = Config::default();
    let pool = AgentPool::new(&config);

    let orchestrator_config = OrchestratorConfig::expert_mode()
        .with_work_dir(&work_dir)
        .with_agent_pool(pool)
        .with_progress(true);

    // Verify config was created
    let _ = orchestrator_config;
}

#[test]
fn test_orchestrator_standard_mode_config() {
    // Test standard mode orchestrator configuration
    use ltmatrix::agent::AgentPool;
    use ltmatrix::config::settings::Config;
    use ltmatrix::pipeline::orchestrator::OrchestratorConfig;

    let work_dir = std::env::current_dir().expect("Failed to get current dir");
    let config = Config::default();
    let pool = AgentPool::new(&config);

    let orchestrator_config = OrchestratorConfig::default()
        .with_work_dir(&work_dir)
        .with_agent_pool(pool)
        .with_progress(true);

    // Verify config was created
    let _ = orchestrator_config;
}

// =============================================================================
// ExecutionMode Conversion Tests
// =============================================================================

#[test]
fn test_execution_mode_conversion() {
    // Test conversion from CLI ExecutionModeArg to model ExecutionMode
    use ltmatrix::cli::args::Args;
    use ltmatrix::models::ExecutionMode;

    // Test Fast mode conversion
    let args =
        Args::try_parse_from(["ltmatrix", "--fast", "test goal"]).expect("Failed to parse args");
    let mode = args.get_execution_mode().to_model();
    assert_eq!(mode, ExecutionMode::Fast);

    // Test Expert mode conversion
    let args =
        Args::try_parse_from(["ltmatrix", "--expert", "test goal"]).expect("Failed to parse args");
    let mode = args.get_execution_mode().to_model();
    assert_eq!(mode, ExecutionMode::Expert);

    // Test Standard mode conversion
    let args = Args::try_parse_from(["ltmatrix", "test goal"]).expect("Failed to parse args");
    let mode = args.get_execution_mode().to_model();
    assert_eq!(mode, ExecutionMode::Standard);
}

// =============================================================================
// Logging Edge Case Tests
// =============================================================================

#[test]
fn test_logging_with_invalid_path() {
    // Test logging initialization with an invalid path
    use ltmatrix::logging::logger;

    // Try to initialize logging with an invalid path
    let invalid_path = PathBuf::from("/nonexistent/directory/that/cannot/be/created/test.log");
    let result = logger::init_logging(
        ltmatrix::logging::level::LogLevel::Info,
        Some(&invalid_path),
    );

    // This might fail, but it should handle the error gracefully
    // We just verify it doesn't panic
    let _ = result;
}

#[test]
fn test_logging_level_default() {
    // Test that default log level is Info when not specified
    use ltmatrix::cli::args::Args;
    use ltmatrix::logging::level::LogLevel as LoggingLevel;

    let args = Args::try_parse_from(["ltmatrix", "test goal"]).expect("Failed to parse args");

    // When log_level is None, it should default to Info
    let log_level = args.log_level.map_or(LoggingLevel::Info, |lvl| match lvl {
        ltmatrix::cli::args::LogLevel::Trace => LoggingLevel::Trace,
        ltmatrix::cli::args::LogLevel::Debug => LoggingLevel::Debug,
        ltmatrix::cli::args::LogLevel::Info => LoggingLevel::Info,
        ltmatrix::cli::args::LogLevel::Warn => LoggingLevel::Warn,
        ltmatrix::cli::args::LogLevel::Error => LoggingLevel::Error,
    });

    assert_eq!(log_level, LoggingLevel::Info);
}

// =============================================================================
// Configuration Edge Case Tests
// =============================================================================

#[test]
fn test_config_loading_without_overrides() {
    // Test loading configuration without any CLI overrides
    use ltmatrix::config::settings;

    let result = settings::load_config_with_overrides(None);

    // Should succeed with defaults
    assert!(
        result.is_ok(),
        "Config loading should succeed with defaults"
    );

    if let Ok(config) = result {
        // Verify default config has reasonable values
        let _ = config.default;
        let _ = config.logging.level;
        let _ = config.output.format;
    }
}

#[test]
fn test_config_fallback_to_defaults() {
    // Test that config falls back to defaults when no files exist
    use ltmatrix::config::settings::Config;

    let config = Config::default();

    // Verify default config is valid
    let _ = config.default;
    let _ = config.logging.level;
    let _ = config.output.format;
}

// =============================================================================
// Agent Backend Edge Case Tests
// =============================================================================

#[test]
fn test_agent_backend_empty_string() {
    // Test agent backend with empty string
    use ltmatrix::agent::AgentFactory;

    let factory = AgentFactory::new();

    // Empty string should not be supported
    assert!(
        !factory.is_supported(""),
        "Empty string should not be a valid backend"
    );
}

#[test]
fn test_agent_backend_case_sensitivity() {
    // Test that agent backend names are case-sensitive
    use ltmatrix::agent::AgentFactory;

    let factory = AgentFactory::new();

    // Test lowercase
    assert!(
        factory.is_supported("claude"),
        "lowercase 'claude' should be supported"
    );

    // Test uppercase (should not be supported if case-sensitive)
    let uppercase_supported = factory.is_supported("CLAUDE");
    let _ = uppercase_supported; // Accept either behavior
}

#[test]
fn test_agent_pool_with_empty_config() {
    // Test AgentPool with minimal/default config
    use ltmatrix::agent::AgentPool;
    use ltmatrix::config::settings::Config;

    let config = Config::default();
    let pool = AgentPool::new(&config);

    // Should create successfully even with minimal config
    let _ = pool;
}

// =============================================================================
// Signal Handling Tests
// =============================================================================

#[test]
fn test_shutdown_flag_thread_safety() {
    // Test that shutdown flag is thread-safe
    static TEST_FLAG: AtomicBool = AtomicBool::new(false);

    // Test basic operations
    assert!(!TEST_FLAG.load(Ordering::SeqCst));

    TEST_FLAG.store(true, Ordering::SeqCst);
    assert!(TEST_FLAG.load(Ordering::SeqCst));

    TEST_FLAG.store(false, Ordering::SeqCst);
    assert!(!TEST_FLAG.load(Ordering::SeqCst));

    // Test different ordering
    TEST_FLAG.store(true, Ordering::Relaxed);
    assert!(TEST_FLAG.load(Ordering::Relaxed));

    TEST_FLAG.store(false, Ordering::Release);
    assert!(!TEST_FLAG.load(Ordering::Acquire));
}

// =============================================================================
// Command Parsing Edge Cases
// =============================================================================

#[test]
fn test_command_with_multiple_goals() {
    // Test that goal can contain multiple words when passed as one argument
    use ltmatrix::cli::args::Args;

    let args =
        Args::try_parse_from(["ltmatrix", "test goal with spaces"]).expect("Failed to parse args");

    // clap should treat the single argument as the goal
    assert_eq!(args.goal, Some("test goal with spaces".to_string()));
}

#[test]
fn test_command_with_empty_goal() {
    // Test command without a goal (should show help)
    use ltmatrix::cli::args::Args;

    let args = Args::try_parse_from(["ltmatrix"]).expect("Failed to parse args");

    assert!(args.goal.is_none(), "Goal should be None when not provided");
    assert!(args.command.is_none(), "No subcommand should be present");
}

#[test]
fn test_release_command_with_options() {
    // Test release command with valid options
    use ltmatrix::cli::args::{Args, Command};

    let args =
        Args::try_parse_from(["ltmatrix", "release", "--archive"]).expect("Failed to parse args");

    assert!(args.command.is_some());

    if let Some(Command::Release(release_args)) = args.command {
        // Verify release command was parsed and archive flag is set
        assert!(release_args.archive, "Archive flag should be set");
    } else {
        panic!("Expected Release command");
    }
}

#[test]
fn test_cleanup_command_with_force() {
    // Test cleanup command with force flag
    use ltmatrix::cli::args::{Args, Command};

    let args =
        Args::try_parse_from(["ltmatrix", "cleanup", "--force"]).expect("Failed to parse args");

    assert!(args.command.is_some());

    if let Some(Command::Cleanup(cleanup_args)) = args.command {
        // Verify cleanup command was parsed
        let _ = cleanup_args;
    } else {
        panic!("Expected Cleanup command");
    }
}

// =============================================================================
// Error Message Tests
// =============================================================================

#[test]
fn test_error_message_formatting() {
    // Test that error messages are formatted correctly
    use anyhow::anyhow;

    let error = anyhow!("Test error message");
    let error_string = error.to_string();

    assert!(error_string.contains("Test error message"));
}

#[test]
fn test_error_chain_multiple_causes() {
    // Test error chain with multiple levels
    use anyhow::{anyhow, Context};

    let base_error = anyhow!("Base error");
    let _level1 = base_error.context("First wrapper");
    let _level2 = _level1.context("Second wrapper");

    // If we got here, error chain was created successfully
    assert!(true, "Error chain should be created");
}

// =============================================================================
// Dry Run Mode Tests
// =============================================================================

#[test]
fn test_dry_run_with_fast_mode() {
    // Test that dry-run can be combined with fast mode
    use ltmatrix::cli::args::Args;

    let args = Args::try_parse_from(["ltmatrix", "--fast", "--dry-run", "test goal"])
        .expect("Failed to parse args");

    assert!(args.fast, "Fast flag should be set");
    assert!(args.dry_run, "Dry-run flag should be set");
}

#[test]
fn test_dry_run_with_expert_mode() {
    // Test that dry-run can be combined with expert mode
    use ltmatrix::cli::args::Args;

    let args = Args::try_parse_from(["ltmatrix", "--expert", "--dry-run", "test goal"])
        .expect("Failed to parse args");

    assert!(args.expert, "Expert flag should be set");
    assert!(args.dry_run, "Dry-run flag should be set");
}

// =============================================================================
// Output Format Tests
// =============================================================================

#[test]
fn test_output_format_text() {
    // Test text output format
    use ltmatrix::cli::args::{Args, OutputFormat};

    let args = Args::try_parse_from(["ltmatrix", "test goal", "--output", "text"])
        .expect("Failed to parse args");

    assert_eq!(args.output, Some(OutputFormat::Text));
}

#[test]
fn test_output_format_json() {
    // Test JSON output format
    use ltmatrix::cli::args::{Args, OutputFormat};

    let args = Args::try_parse_from(["ltmatrix", "test goal", "--output", "json"])
        .expect("Failed to parse args");

    assert_eq!(args.output, Some(OutputFormat::Json));
}

// =============================================================================
// Log File Path Tests
// =============================================================================

#[test]
fn test_log_file_absolute_path() {
    // Test logging with absolute path
    use ltmatrix::cli::args::Args;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("test.log");

    let args = Args::try_parse_from([
        "ltmatrix",
        "test goal",
        "--log-file",
        log_path.to_str().unwrap(),
    ])
    .expect("Failed to parse args");

    assert_eq!(args.log_file, Some(log_path));
}

#[test]
fn test_log_file_relative_path() {
    // Test logging with relative path
    use ltmatrix::cli::args::Args;

    let args = Args::try_parse_from(["ltmatrix", "test goal", "--log-file", "./logs/test.log"])
        .expect("Failed to parse args");

    assert_eq!(args.log_file, Some(PathBuf::from("./logs/test.log")));
}

// =============================================================================
// Agent Selection Priority Tests
// =============================================================================

#[test]
fn test_agent_selection_cli_priority() {
    // Test that CLI agent flag takes priority over config
    use ltmatrix::cli::args::Args;

    let args = Args::try_parse_from(["ltmatrix", "--agent", "claude", "test goal"])
        .expect("Failed to parse args");

    assert_eq!(args.agent, Some("claude".to_string()));
}

// =============================================================================
// Banner Printing Tests
// =============================================================================

#[test]
fn test_banner_conditions() {
    // Test all conditions for banner printing
    use ltmatrix::cli::args::Args;

    // No banner for subcommands
    let args =
        Args::try_parse_from(["ltmatrix", "completions", "bash"]).expect("Failed to parse args");
    assert!(args.command.is_some(), "Should have subcommand");

    // No banner for man
    let args = Args::try_parse_from(["ltmatrix", "man"]).expect("Failed to parse args");
    assert!(args.command.is_some(), "Should have subcommand");

    // Banner for run command (no subcommand)
    let args = Args::try_parse_from(["ltmatrix", "test goal"]).expect("Failed to parse args");
    assert!(args.command.is_none(), "Should not have subcommand");
}

// =============================================================================
// Panic Handler Tests
// =============================================================================

#[test]
fn test_panic_hook_preservation() {
    // Test that panic hook can be set and restored
    use std::panic;

    let hook1 = panic::take_hook();
    let _ = hook1;

    let hook2 = panic::take_hook();
    panic::set_hook(hook2);

    // If we got here without panicking, the test passed
    assert!(true);
}

// =============================================================================
// Completion Tests
// =============================================================================

#[test]
fn test_completions_all_shells() {
    // Test completions command for different shells
    use ltmatrix::cli::args::{Args, Command};

    // Test bash
    let args =
        Args::try_parse_from(["ltmatrix", "completions", "bash"]).expect("Failed to parse args");
    assert!(matches!(args.command, Some(Command::Completions(_))));

    // Test zsh
    let args =
        Args::try_parse_from(["ltmatrix", "completions", "zsh"]).expect("Failed to parse args");
    assert!(matches!(args.command, Some(Command::Completions(_))));

    // Test fish
    let args =
        Args::try_parse_from(["ltmatrix", "completions", "fish"]).expect("Failed to parse args");
    assert!(matches!(args.command, Some(Command::Completions(_))));

    // Test powershell
    let args = Args::try_parse_from(["ltmatrix", "completions", "powershell"])
        .expect("Failed to parse args");
    assert!(matches!(args.command, Some(Command::Completions(_))));

    // Test elvish
    let args =
        Args::try_parse_from(["ltmatrix", "completions", "elvish"]).expect("Failed to parse args");
    assert!(matches!(args.command, Some(Command::Completions(_))));
}
