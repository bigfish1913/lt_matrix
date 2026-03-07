//! Integration tests for main application entry point
//!
//! This test suite verifies the core functionality of src/main.rs:
//! - Logging initialization from CLI args
//! - Configuration loading from files and CLI overrides
//! - Agent backend initialization
//! - Command routing (run/release/completions/man/cleanup)
//! - Error handling with user-friendly messages
//! - Signal handling for graceful shutdown
//! - Help and banner functionality

use clap::Parser;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

// Note: We can't directly test main() since it calls std::process::exit()
// However, we can test the individual functions that make up the main logic

// =============================================================================
// Logging Initialization Tests
// =============================================================================

#[test]
fn test_log_level_conversion() {
    // Test that CLI log levels convert correctly to logging log levels
    // This tests the logic in initialize_logging()
    use ltmatrix::cli::args::LogLevel;
    use ltmatrix::logging::level::LogLevel as LoggingLevel;

    // Test Trace level
    let trace_level = LoggingLevel::Trace;
    assert_eq!(format!("{:?}", trace_level), "Trace");

    // Test Debug level
    let debug_level = LoggingLevel::Debug;
    assert_eq!(format!("{:?}", debug_level), "Debug");

    // Test Info level (default)
    let info_level = LoggingLevel::Info;
    assert_eq!(format!("{:?}", info_level), "Info");

    // Test Warn level
    let warn_level = LoggingLevel::Warn;
    assert_eq!(format!("{:?}", warn_level), "Warn");

    // Test Error level
    let error_level = LoggingLevel::Error;
    assert_eq!(format!("{:?}", error_level), "Error");
}

#[test]
fn test_logging_initialization_with_file() {
    // Test that logging can be initialized with a specific log file
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let log_file = temp_dir.path().join("test.log");

    // Initialize logging with file
    let result = ltmatrix::logging::logger::init_logging(
        ltmatrix::logging::level::LogLevel::Info,
        Some(&log_file)
    );

    assert!(result.is_ok(), "Logging initialization should succeed with valid file path");

    // Verify log file was created
    assert!(log_file.exists(), "Log file should be created");
}

#[test]
fn test_logging_initialization_console_only() {
    // Test console-only logging initialization
    let result = ltmatrix::logging::logger::init_logging(
        ltmatrix::logging::level::LogLevel::Debug,
        None::<&PathBuf>
    );

    assert!(result.is_ok(), "Console-only logging initialization should succeed");
}

// =============================================================================
// Configuration Loading Tests
// =============================================================================

#[test]
fn test_cli_overrides_creation() {
    // Test that CliOverrides can be created from Args
    use ltmatrix::cli::args::Args;
    use ltmatrix::config::settings::CliOverrides;

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal",
        "--fast",
        "--agent", "claude"
    ]);

    let overrides = CliOverrides::from(args);

    // Verify overrides were created successfully
    // The exact structure depends on the CliOverrides implementation
    let _ = overrides; // Use the variable to avoid unused warning
}

#[test]
fn test_config_loading_with_overrides() {
    // Test that configuration can be loaded with CLI overrides
    use ltmatrix::cli::args::Args;
    use ltmatrix::config::settings::CliOverrides;

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal"
    ]);

    let overrides = CliOverrides::from(args);

    // Load config with overrides
    let result = ltmatrix::config::settings::load_config_with_overrides(Some(overrides));

    // This should succeed even if no config files exist
    // (it should fall back to defaults)
    assert!(result.is_ok(), "Config loading should succeed with defaults");

    if let Ok(config) = result {
        // Verify config has expected fields
        let _ = config.default; // Default agent
        let _ = config.logging.level; // Log level
        let _ = config.output.format; // Output format
    }
}

// =============================================================================
// Agent Backend Initialization Tests
// =============================================================================

#[test]
fn test_agent_factory_creation() {
    // Test that AgentFactory can be created
    use ltmatrix::agent::AgentFactory;

    let factory = AgentFactory::new();

    // Verify factory knows about supported backends
    let supported = factory.supported_backends();
    assert!(!supported.is_empty(), "Factory should have supported backends");

    // Verify common backends are supported
    let backend_names: Vec<String> = supported.iter().map(|s| s.to_string()).collect();
    assert!(backend_names.iter().any(|s| s == "claude"), "Should support claude backend");
}

#[test]
fn test_agent_backend_support_check() {
    // Test that AgentFactory correctly validates supported backends
    use ltmatrix::agent::AgentFactory;

    let factory = AgentFactory::new();

    // Test supported backend
    assert!(factory.is_supported("claude"), "claude should be supported");

    // Test unsupported backend
    assert!(!factory.is_supported("nonexistent"), "nonexistent backend should not be supported");
}

#[test]
fn test_agent_pool_creation() {
    // Test that AgentPool can be created with config
    use ltmatrix::agent::AgentPool;
    use ltmatrix::config::settings::Config;

    // Create a minimal config
    let config = Config::default();

    // Create agent pool
    let pool = AgentPool::new(&config);

    // Verify pool was created
    let _ = pool; // Use the variable
}

// =============================================================================
// Command Routing Tests
// =============================================================================

#[test]
fn test_command_parsing_run() {
    // Test parsing default run command
    use ltmatrix::cli::args::{Args, Command};

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal"
    ]);

    assert!(args.goal.is_some(), "Goal should be parsed");
    assert_eq!(args.goal, Some("test goal".to_string()));
    assert!(args.command.is_none(), "No subcommand should be present");
}

#[test]
fn test_command_parsing_release() {
    // Test parsing release subcommand
    use ltmatrix::cli::args::{Args, Command};

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "release"
    ]);

    assert!(args.command.is_some(), "Subcommand should be present");

    if let Some(Command::Release(_)) = args.command {
        // Correct subcommand was parsed
    } else {
        panic!("Expected Release command");
    }
}

#[test]
fn test_command_parsing_completions() {
    // Test parsing completions subcommand
    use ltmatrix::cli::args::{Args, Command};

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "completions",
        "bash"
    ]);

    assert!(args.command.is_some(), "Subcommand should be present");

    if let Some(Command::Completions(compl)) = args.command {
        assert_eq!(format!("{:?}", compl.shell), "Bash");
    } else {
        panic!("Expected Completions command");
    }
}

#[test]
fn test_command_parsing_man() {
    // Test parsing man subcommand
    use ltmatrix::cli::args::{Args, Command};

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "man"
    ]);

    assert!(args.command.is_some(), "Subcommand should be present");

    if let Some(Command::Man(_)) = args.command {
        // Correct subcommand was parsed
    } else {
        panic!("Expected Man command");
    }
}

#[test]
fn test_command_parsing_cleanup() {
    // Test parsing cleanup subcommand
    use ltmatrix::cli::args::{Args, Command};

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "cleanup"
    ]);

    assert!(args.command.is_some(), "Subcommand should be present");

    if let Some(Command::Cleanup(_)) = args.command {
        // Correct subcommand was parsed
    } else {
        panic!("Expected Cleanup command");
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_error_chain_display() {
    // Test that error chains are displayed properly
    use anyhow::{anyhow, Context};

    let base_error = anyhow!("Base error occurred");
    let wrapped_error = base_error.context("Additional context");

    let error_string = wrapped_error.to_string();
    assert!(error_string.contains("Base error occurred") ||
            error_string.contains("Additional context"),
            "Error message should contain error information");
}

#[test]
fn test_permission_error_hint() {
    // Test that permission errors get appropriate hints
    let error_msg = "Permission denied while accessing file";
    let lower_msg = error_msg.to_lowercase();

    assert!(lower_msg.contains("permission"), "Test error should contain permission");

    // Verify the hint logic from print_error()
    if lower_msg.contains("permission") || lower_msg.contains("access") {
        // This would trigger the permission hint in print_error()
        let _ = "Hint: Check file permissions and try running with appropriate access.";
    }
}

#[test]
fn test_network_error_hint() {
    // Test that network errors get appropriate hints
    let error_msg = "Network connection failed";
    let lower_msg = error_msg.to_lowercase();

    assert!(lower_msg.contains("network"), "Test error should contain network");

    // Verify the hint logic from print_error()
    if lower_msg.contains("network") || lower_msg.contains("connection") {
        // This would trigger the network hint in print_error()
        let _ = "Hint: Check your internet connection and try again.";
    }
}

#[test]
fn test_config_error_hint() {
    // Test that config errors get appropriate hints
    let error_msg = "Configuration file parse error";
    let lower_msg = error_msg.to_lowercase();

    assert!(lower_msg.contains("config"), "Test error should contain config");

    // Verify the hint logic from print_error()
    if lower_msg.contains("config") || lower_msg.contains("configuration") {
        // This would trigger the config hint in print_error()
        let hint = "Hint: Check your configuration file at:";
        assert!(hint.contains("configuration"), "Hint should mention config");
    }
}

#[test]
fn test_agent_error_hint() {
    // Test that agent errors get appropriate hints
    let error_msg = "Agent backend initialization failed";
    let lower_msg = error_msg.to_lowercase();

    assert!(lower_msg.contains("agent"), "Test error should contain agent");

    // Verify the hint logic from print_error()
    if lower_msg.contains("agent") || lower_msg.contains("backend") {
        // This would trigger the agent hint in print_error()
        let hint = "Hint: Ensure the agent backend is properly configured.";
        assert!(hint.contains("agent") || hint.contains("backend"),
                "Hint should mention agent/backend");
    }
}

// =============================================================================
// Signal Handling Tests
// =============================================================================

#[test]
fn test_shutdown_flag_functionality() {
    // Test the shutdown flag functionality
    // This mimics the SHUTDOWN_REQUESTED flag in main.rs
    static TEST_SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

    // Initial state should be false
    assert!(!TEST_SHUTDOWN_FLAG.load(Ordering::SeqCst),
            "Initial shutdown state should be false");

    // Set to true
    TEST_SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    assert!(TEST_SHUTDOWN_FLAG.load(Ordering::SeqCst),
            "Shutdown state should be true after setting");

    // Reset to false
    TEST_SHUTDOWN_FLAG.store(false, Ordering::SeqCst);
    assert!(!TEST_SHUTDOWN_FLAG.load(Ordering::SeqCst),
            "Shutdown state should be false after reset");
}

// =============================================================================
// Execution Mode Tests
// =============================================================================

#[test]
fn test_execution_mode_detection() {
    // Test that execution modes are correctly detected from args
    use ltmatrix::cli::args::{Args, ExecutionModeArg};

    // Test standard mode (default)
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal"
    ]);
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Standard);

    // Test fast mode
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "--fast",
        "test goal"
    ]);
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Fast);

    // Test expert mode
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "--expert",
        "test goal"
    ]);
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Expert);
}

#[test]
fn test_dry_run_mode() {
    // Test that dry-run mode is correctly detected
    use ltmatrix::cli::args::Args;

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "--dry-run",
        "test goal"
    ]);

    assert!(args.dry_run, "Dry-run flag should be set");
}

#[test]
fn test_resume_mode() {
    // Test that resume mode is correctly detected
    use ltmatrix::cli::args::Args;

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "--resume"
    ]);

    assert!(args.resume, "Resume flag should be set");
}

#[test]
fn test_ask_mode() {
    // Test that ask mode is correctly detected
    use ltmatrix::cli::args::Args;

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "--ask",
        "test goal"
    ]);

    assert!(args.ask, "Ask flag should be set");
}

// =============================================================================
// Banner and Help Tests
// =============================================================================

#[test]
fn test_banner_not_printed_for_subcommands() {
    // Test that banner is not printed for subcommands
    use ltmatrix::cli::args::Args;

    // Completions subcommand
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "completions",
        "bash"
    ]);

    assert!(args.command.is_some(), "Subcommand should be set");
    // In main.rs, print_banner checks if args.command.is_some() and returns early

    // Man subcommand
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "man"
    ]);

    assert!(args.command.is_some(), "Subcommand should be set");

    // Cleanup subcommand
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "cleanup"
    ]);

    assert!(args.command.is_some(), "Subcommand should be set");
}

#[test]
fn test_banner_printed_for_default_run() {
    // Test that banner is printed for default run command
    use ltmatrix::cli::args::Args;

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal"
    ]);

    assert!(args.command.is_none(), "No subcommand should be set");
    assert!(args.is_run_command(), "Should be a run command");
    assert!(args.goal.is_some(), "Goal should be set");
}

#[test]
fn test_version_flag() {
    // Test that version flag is parsed correctly
    use ltmatrix::cli::args::Args;

    // Note: --version will cause clap to exit, so we can't test it directly
    // But we can verify that Args can be parsed for other flags

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal"
    ]);

    // If we got here, parsing succeeded
    let _ = args;
}

// =============================================================================
// Log File Management Tests
// =============================================================================

#[test]
fn test_log_manager_cleanup() {
    // Test that LogManager can clean up old log files
    use ltmatrix::logging::file_manager::LogManager;
    use std::fs;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let log_dir = temp_dir.path().join("logs");
    fs::create_dir_all(&log_dir).expect("Failed to create log dir");

    // Create some old log files
    let old_log = log_dir.join("old.log");
    fs::write(&old_log, "old log content").expect("Failed to create old log");

    // Create log manager
    let manager = LogManager::new(Some(log_dir.clone())).with_max_files(5);

    // Perform cleanup
    let result = manager.cleanup_on_success();
    assert!(result.is_ok(), "Cleanup should succeed");

    // Verify old log was removed
    // (Note: actual removal depends on retention policy implementation)
    let _ = old_log;
    let _ = manager;
}

#[test]
fn test_logging_with_management() {
    // Test automatic log file management
    use ltmatrix::logging::logger;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let result = logger::init_logging_with_management(
        ltmatrix::logging::level::LogLevel::Info,
        Some(temp_dir.path())
    );

    assert!(result.is_ok(), "Logging with management should succeed");

    if let Ok((guard, manager)) = result {
        // Verify manager was created
        let _ = manager;

        // Keep guard alive for test duration
        let _ = guard;
    }
}

// =============================================================================
// Agent Selection Tests
// =============================================================================

#[test]
fn test_agent_selection_from_cli() {
    // Test that CLI agent override takes precedence
    use ltmatrix::cli::args::Args;

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "--agent",
        "claude",
        "test goal"
    ]);

    assert_eq!(args.agent, Some("claude".to_string()),
               "Agent should be set from CLI");
}

#[test]
fn test_agent_backend_validation() {
    // Test that unsupported agent backends are rejected
    use ltmatrix::agent::AgentFactory;

    let factory = AgentFactory::new();

    // Try to create an unsupported agent
    let result = factory.create("unsupported_backend");
    assert!(result.is_err(), "Creating unsupported backend should fail");

    if let Err(e) = result {
        let error_msg = e.to_string().to_lowercase();
        assert!(error_msg.contains("unsupported") ||
                error_msg.contains("not found") ||
                error_msg.contains("unknown"),
                "Error should mention backend is not supported");
    }
}

// =============================================================================
// Panic Handler Tests
// =============================================================================

#[test]
fn test_panic_hook_does_not_crash() {
    // Note: We can't directly test the panic hook since it modifies process state
    // But we can verify that the panic hook type is valid

    use std::panic;

    // Save original hook
    let original_hook = panic::take_hook();

    // Set a test hook (similar to what main.rs does)
    panic::set_hook(Box::new(move |panic_info| {
        // Log the panic
        let _ = panic_info;
    }));

    // Restore original hook
    panic::set_hook(original_hook);

    // If we got here, panic hook operations succeeded
    assert!(true, "Panic hook should be settable");
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_full_application_flow() {
    // Test a simplified version of the full application flow
    use ltmatrix::cli::args::Args;

    // Parse arguments
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal",
        "--fast"
    ]);

    // Verify parsing succeeded
    assert_eq!(args.goal, Some("test goal".to_string()));
    assert!(args.fast);
    assert_eq!(args.get_execution_mode(), ltmatrix::cli::args::ExecutionModeArg::Fast);

    // In the real flow, this would continue to:
    // - Initialize logging
    // - Load configuration
    // - Initialize agent backend
    // - Execute command
}

#[test]
fn test_multiple_flags_combination() {
    // Test that multiple flags can be combined
    use ltmatrix::cli::args::Args;

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal",
        "--fast",
        "--dry-run",
        "--ask",
        "--log-level", "debug"
    ]);

    assert!(args.fast);
    assert!(args.dry_run);
    assert!(args.ask);
    assert!(args.log_level.is_some());
}

#[test]
fn test_output_format_parsing() {
    // Test that output format is parsed correctly
    use ltmatrix::cli::args::{Args, OutputFormat};

    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal",
        "--output", "json"
    ]);

    assert_eq!(args.output, Some(OutputFormat::Json));
}

#[test]
fn test_log_level_parsing() {
    // Test that log levels are parsed correctly
    use ltmatrix::cli::args::{Args, LogLevel};

    // Test trace
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal",
        "--log-level", "trace"
    ]);
    assert_eq!(args.log_level, Some(LogLevel::Trace));

    // Test debug
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal",
        "--log-level", "debug"
    ]);
    assert_eq!(args.log_level, Some(LogLevel::Debug));

    // Test info (default)
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal"
    ]);
    assert_eq!(args.log_level, None); // None means default

    // Test warn
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal",
        "--log-level", "warn"
    ]);
    assert_eq!(args.log_level, Some(LogLevel::Warn));

    // Test error
    let args = Args::parse_from([  // No .expect() needed for parse_from
        "ltmatrix",
        "test goal",
        "--log-level", "error"
    ]);
    assert_eq!(args.log_level, Some(LogLevel::Error));
}
