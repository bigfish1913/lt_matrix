// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! ltmatrix - Long-Time Agent Orchestrator
//!
//! This is the main entry point for the ltmatrix application.
//! It handles initialization, configuration loading, logging setup,
//! signal handling, and routes to appropriate subcommands.

#![allow(clippy::too_many_lines)]

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, error, info, instrument, warn};

use ltmatrix::agent::{AgentFactory, AgentPool};
use ltmatrix::cli::args::{Args, Command};
use ltmatrix::config::settings::{self, CliOverrides, Config};
use ltmatrix::logging::file_manager::LogManager;
use ltmatrix::logging::level::LogLevel as LoggingLevel;
use ltmatrix::logging::logger;
use ltmatrix::models::ExecutionMode;
use ltmatrix::pipeline::orchestrator::{OrchestratorConfig, PipelineOrchestrator};

// =============================================================================
// Application State
// =============================================================================

/// Global shutdown flag
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Application state shared across components
struct AppState {
    /// Configuration loaded from files and CLI overrides
    config: Config,
    /// Command-line arguments
    args: Args,
    /// Agent pool for session management
    agent_pool: Option<AgentPool>,
}

impl AppState {
    /// Create new application state
    fn new(config: Config, args: Args) -> Self {
        AppState {
            config,
            args,
            agent_pool: None,
        }
    }

    /// Set the agent pool
    fn with_agent_pool(mut self, pool: AgentPool) -> Self {
        self.agent_pool = Some(pool);
        self
    }
}

// =============================================================================
// Main Entry Point
// =============================================================================

fn main() {
    // Set up panic handler to ensure we exit with proper error code
    let _original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        error!("Application panic: {}", panic_info);
        let _ = std::io::stdout().flush();
        std::process::exit(1);
    }));

    // Parse command-line arguments
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            // Check if this is a help/version request
            if e.kind() == clap::error::ErrorKind::DisplayHelp
                || e.kind() == clap::error::ErrorKind::DisplayVersion
            {
                // Use clap's exit handler which prints help/version and exits correctly
                e.exit();
            }
            // Other errors should be reported
            eprintln!("Error parsing arguments: {}", e);
            std::process::exit(1);
        }
    };

    // Run the application with proper error handling
    let result = run_application(args);

    // Handle final result
    match result {
        Ok(exit_code) => {
            debug!("Application exiting with code: {}", exit_code);
            std::process::exit(exit_code);
        }
        Err(e) => {
            print_error(&e);
            std::process::exit(1);
        }
    }
}

/// Run the main application logic
///
/// This function:
/// 1. Initializes logging from CLI arguments
/// 2. Loads configuration from files and CLI overrides
/// 3. Initializes agent backends
/// 4. Routes to appropriate subcommands
/// 5. Handles graceful shutdown on signals
#[instrument(skip(args))]
fn run_application(args: Args) -> Result<i32> {
    // Print banner for version info
    print_banner(&args);

    // Initialize logging from CLI arguments
    let (log_manager, _log_level) = initialize_logging(&args)?;

    // Load configuration from files and CLI overrides
    let config = load_configuration(&args)?;

    // Initialize agent backends
    let agent_pool = initialize_agent_backend(&config, &args)?;

    // Create application state
    let mut app_state = AppState::new(config, args.clone()).with_agent_pool(agent_pool);

    // Set up signal handling for graceful shutdown
    setup_signal_handlers();

    info!("ltmatrix v{} starting", env!("CARGO_PKG_VERSION"));

    // Route to appropriate subcommand
    let exit_code = match &args.command {
        Some(Command::Release(release_args)) => {
            execute_release_command(&mut app_state, release_args)?
        }
        Some(Command::Completions(completions_args)) => {
            execute_completions_command(&mut app_state, completions_args)?
        }
        Some(Command::Man(man_args)) => execute_man_command(&mut app_state, man_args)?,
        Some(Command::Cleanup(cleanup_args)) => {
            execute_cleanup_command(&mut app_state, cleanup_args)?
        }
        Some(Command::Memory(memory_args)) => execute_memory_command(&mut app_state, memory_args)?,
        None => {
            // Default: run the main pipeline
            execute_run_command(&mut app_state)?
        }
    };

    // Cleanup on successful completion
    cleanup_on_success(log_manager)?;

    Ok(exit_code)
}

// =============================================================================
// Initialization Functions
// =============================================================================

/// Print application banner
fn print_banner(args: &Args) {
    // Only print banner if we're running in default mode
    // (not for subcommands like completions, man, etc.)
    if args.command.is_some() {
        return;
    }

    println!("ltmatrix - Long-Time Agent Orchestrator");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    if args.is_run_command() && args.goal.is_some() {
        println!();
    }
}

/// Initialize logging from CLI arguments
///
/// Sets up console and/or file logging based on:
/// - `--log-level` argument (defaults to INFO)
/// - `--log-file` argument (optional file path)
/// - Configuration file settings
fn initialize_logging(args: &Args) -> Result<(Option<LogManager>, LoggingLevel)> {
    // Convert CLI log level to logging log level
    let log_level = args.log_level.map_or(LoggingLevel::Info, |lvl| match lvl {
        ltmatrix::cli::args::LogLevel::Trace => LoggingLevel::Trace,
        ltmatrix::cli::args::LogLevel::Debug => LoggingLevel::Debug,
        ltmatrix::cli::args::LogLevel::Info => LoggingLevel::Info,
        ltmatrix::cli::args::LogLevel::Warn => LoggingLevel::Warn,
        ltmatrix::cli::args::LogLevel::Error => LoggingLevel::Error,
    });

    // Determine if we should use file logging
    let log_manager = if args.log_file.is_some() {
        // User specified a specific log file
        let log_file = args.log_file.as_ref().unwrap();
        let _guard = logger::init_logging(log_level, Some(log_file.as_path()))
            .context("Failed to initialize logging with file")?;
        info!("Logging to file: {}", log_file.display());
        None
    } else if args.command.is_none() || matches!(args.command, Some(Command::Release(_))) {
        // Use automatic log file management for main run and release commands
        let base_dir = env::current_dir().context("Failed to get current directory")?;

        let (guard, manager) = logger::init_logging_with_management(log_level, Some(base_dir))
            .context("Failed to initialize logging with management")?;

        // The guard must be kept alive for the application lifetime
        // We store it in a static to prevent it from being dropped
        if let Err(e) = set_global_log_guard(guard) {
            warn!("Failed to set global log guard: {}", e);
        }

        info!("Logging initialized with automatic file management");
        Some(manager)
    } else {
        // Console-only logging for other subcommands
        let _guard = logger::init_logging(log_level, None::<&PathBuf>)
            .context("Failed to initialize console logging")?;
        info!("Console-only logging initialized");
        None
    };

    Ok((log_manager, log_level))
}

/// Load configuration from files and CLI overrides
///
/// Configuration is loaded in this order (later overrides earlier):
/// 1. Global config file (~/.config/ltmatrix/config.toml)
/// 2. Project config file (.ltmatrix/config.toml)
/// 3. CLI arguments (via CliOverrides)
fn load_configuration(args: &Args) -> Result<Config> {
    debug!("Loading configuration");

    // Create CLI overrides from command-line arguments
    let overrides = args.to_overrides();

    // Load config with overrides
    let config = settings::load_config_with_overrides(Some(overrides))
        .context("Failed to load configuration")?;

    debug!("Configuration loaded successfully");
    debug!("Default agent: {:?}", config.default);
    debug!("Log level: {:?}", config.logging.level);
    debug!("Output format: {:?}", config.output.format);

    Ok(config)
}

/// Initialize agent backend based on configuration
///
/// Creates and initializes the appropriate agent backend:
/// - Claude (default)
/// - OpenCode
/// - KimiCode
/// - Codex
fn initialize_agent_backend(config: &Config, args: &Args) -> Result<AgentPool> {
    debug!("Initializing agent backend");

    // Determine which agent to use
    let agent_name = if let Some(ref agent) = args.agent {
        // CLI override takes precedence
        agent.clone()
    } else if let Some(ref default) = config.default {
        // Use config default
        default.clone()
    } else {
        // Use system default
        "claude".to_string()
    };

    debug!("Using agent backend: {}", agent_name);

    // Create agent factory
    let factory = AgentFactory::new();

    // Validate agent is supported
    if !factory.is_supported(&agent_name) {
        return Err(anyhow!(
            "Unsupported agent backend '{}'. Supported: {:?}",
            agent_name,
            factory.supported_backends()
        ));
    }

    // Create agent instance
    let _agent = factory
        .create(&agent_name)
        .with_context(|| format!("Failed to create agent backend '{}'", agent_name))?;

    // Create agent pool with configuration
    let pool = AgentPool::new(config);

    info!("Agent backend '{}' initialized successfully", agent_name);

    Ok(pool)
}

/// Set up signal handlers for graceful shutdown
///
/// Handles:
/// - SIGINT (Ctrl+C)
/// - SIGTERM (termination signal)
fn setup_signal_handlers() {
    // Register shutdown signal handler using signal-hook
    #[cfg(unix)]
    {
        use signal_hook::{consts::SIGTERM, iterator::Signals};

        let mut signals = Signals::new([SIGTERM, signal_hook::consts::SIGINT])
            .expect("Failed to register signal handler");

        std::thread::spawn(move || {
            for sig in signals.forever() {
                info!("Received signal: {}", sig);
                SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
                warn!("Shutdown requested. Cleaning up...");
                break;
            }
        });

        debug!("Signal handlers registered");
    }

    #[cfg(windows)]
    {
        // Basic Windows signal handling placeholder
        // Full Windows signal handling requires additional dependencies
        debug!("Windows signal handling: basic implementation");
    }
}

/// Check if shutdown has been requested
fn is_shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

// =============================================================================
// Command Execution Functions
// =============================================================================

/// Execute the main run command (default behavior)
///
/// This is the primary entry point for running ltmatrix with a goal.
/// It:
/// 1. Validates the goal is provided
/// 2. Configures the execution mode
/// 3. Invokes the pipeline orchestrator
/// 4. Returns appropriate exit code
#[instrument(skip(app_state))]
fn execute_run_command(app_state: &mut AppState) -> Result<i32> {
    let args = &app_state.args;

    // Ensure we have a goal (from CLI arg or file)
    let goal = if let Some(ref file_path) = args.file {
        // Read goal from file
        std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read goal from file: {}", file_path.display()))?
    } else if let Some(ref goal) = args.goal {
        goal.clone()
    } else {
        print_help();
        return Ok(0);
    };

    info!("Starting run command");
    info!("Goal: {}", goal);
    info!("Mode: {:?}", args.get_execution_mode());

    // Determine execution mode
    let mode = args.get_execution_mode().to_model();

    // Check for dry-run mode
    if args.dry_run {
        info!("Dry run mode: pipeline will be planned but not executed");
        println!("\nDry run mode - planning without execution");
        println!("Goal: {}", goal);
        println!("Mode: {:?}", mode);
        println!();
        println!("Remove --dry-run to execute the pipeline");
        return Ok(0);
    }

    // Build orchestrator configuration
    let work_dir = env::current_dir().context("Failed to get current directory")?;

    // Get agent pool from app state
    let agent_pool = app_state
        .agent_pool
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Agent pool not initialized"))?;

    // Create orchestrator config based on execution mode
    let orchestrator_config = match mode {
        ExecutionMode::Fast => OrchestratorConfig::fast_mode()
            .with_work_dir(&work_dir)
            .with_agent_pool(agent_pool)
            .with_progress(app_state.config.output.progress),
        ExecutionMode::Expert => OrchestratorConfig::expert_mode()
            .with_work_dir(&work_dir)
            .with_agent_pool(agent_pool)
            .with_progress(app_state.config.output.progress),
        ExecutionMode::Standard => OrchestratorConfig::default()
            .with_work_dir(&work_dir)
            .with_agent_pool(agent_pool)
            .with_progress(app_state.config.output.progress),
    };

    // Create pipeline orchestrator
    let orchestrator = PipelineOrchestrator::new(orchestrator_config)
        .context("Failed to create pipeline orchestrator")?;

    println!("\n🚀 Starting Pipeline Execution");
    println!("Goal: {}", goal);
    println!("Mode: {}", mode);
    println!();

    // Execute the pipeline
    let result = tokio::runtime::Runtime::new()
        .context("Failed to create async runtime")?
        .block_on(orchestrator.execute_pipeline(&goal, mode))
        .context("Pipeline execution failed")?;

    // Print results
    println!();
    println!("Pipeline Execution Summary:");
    println!("  Total tasks: {}", result.total_tasks);
    println!("  Completed: {}", result.tasks_completed);
    println!("  Failed: {}", result.tasks_failed);
    println!("  Stages completed: {}", result.stages_completed);
    println!("  Success rate: {:.1}%", result.success_rate());
    println!("  Total time: {:.2}s", result.total_time.as_secs_f64());
    println!();

    // Return appropriate exit code
    if result.success {
        Ok(0)
    } else {
        Ok(1)
    }
}

/// Execute the release subcommand
#[instrument(skip(app_state, release_args))]
fn execute_release_command(
    app_state: &mut AppState,
    release_args: &ltmatrix::cli::args::ReleaseArgs,
) -> Result<i32> {
    info!("Executing release command");

    // Delegate to CLI command handler
    if let Err(e) = ltmatrix::cli::command::execute_release(&app_state.args, release_args) {
        error!("Release command failed: {}", e);
        return Ok(1);
    }

    Ok(0)
}

/// Execute the completions subcommand
#[instrument(skip(_app_state, completions_args))]
fn execute_completions_command(
    _app_state: &mut AppState,
    completions_args: &ltmatrix::cli::args::CompletionsArgs,
) -> Result<i32> {
    info!(
        "Executing completions command for shell: {}",
        completions_args.shell
    );

    // Delegate to CLI command handler
    if let Err(e) = ltmatrix::cli::command::execute_completions(completions_args) {
        error!("Completions command failed: {}", e);
        return Ok(1);
    }

    Ok(0)
}

/// Execute the man subcommand
#[instrument(skip(_app_state, man_args))]
fn execute_man_command(
    _app_state: &mut AppState,
    man_args: &ltmatrix::cli::args::ManArgs,
) -> Result<i32> {
    info!("Executing man command");

    // Delegate to CLI command handler
    if let Err(e) = ltmatrix::cli::command::execute_man(man_args) {
        error!("Man command failed: {}", e);
        return Ok(1);
    }

    Ok(0)
}

/// Execute the cleanup subcommand
#[instrument(skip(_app_state, cleanup_args))]
fn execute_cleanup_command(
    _app_state: &mut AppState,
    cleanup_args: &ltmatrix::cli::args::CleanupArgs,
) -> Result<i32> {
    info!("Executing cleanup command");

    // Delegate to CLI command handler
    if let Err(e) = ltmatrix::cli::command::execute_cleanup(cleanup_args) {
        error!("Cleanup command failed: {}", e);
        return Ok(1);
    }

    Ok(0)
}

/// Execute the memory subcommand
#[instrument(skip(_app_state, memory_args))]
fn execute_memory_command(
    _app_state: &mut AppState,
    memory_args: &ltmatrix::cli::args::MemoryArgs,
) -> Result<i32> {
    info!("Executing memory command");

    // Delegate to CLI command handler
    if let Err(e) = ltmatrix::cli::command::execute_memory(memory_args) {
        error!("Memory command failed: {}", e);
        return Ok(1);
    }

    Ok(0)
}

// =============================================================================
// Cleanup Functions
// =============================================================================

/// Cleanup on successful completion
///
/// Performs cleanup operations:
/// - Removes old log files based on retention policy
/// - Flushes any pending log messages
fn cleanup_on_success(log_manager: Option<LogManager>) -> Result<()> {
    debug!("Starting cleanup on success");

    // Clean up old log files if we have a log manager
    if let Some(manager) = log_manager {
        match manager.cleanup_on_success() {
            Ok(removed) => {
                if removed > 0 {
                    info!("Removed {} old log file(s)", removed);
                }
            }
            Err(e) => {
                warn!("Failed to cleanup old log files: {}", e);
            }
        }
    }

    debug!("Cleanup completed");
    Ok(())
}

// =============================================================================
// Error Handling Functions
// =============================================================================

/// Print error with user-friendly formatting
fn print_error(error: &anyhow::Error) {
    eprintln!();
    eprintln!("❌ Error:");

    // Print the error chain
    for (i, cause) in error.chain().enumerate() {
        if i == 0 {
            eprintln!("   {}", cause);
        } else {
            eprintln!("   Caused by: {}", cause);
        }
    }

    eprintln!();

    // Print helpful hints based on error content
    let error_msg = error.to_string().to_lowercase();
    if error_msg.contains("permission") || error_msg.contains("access") {
        eprintln!("Hint: Check file permissions and try running with appropriate access.");
    } else if error_msg.contains("network") || error_msg.contains("connection") {
        eprintln!("Hint: Check your internet connection and try again.");
    } else if error_msg.contains("config") || error_msg.contains("configuration") {
        eprintln!("Hint: Check your configuration file at:");
        eprintln!("  - ~/.config/ltmatrix/config.toml (global)");
        eprintln!("  - .ltmatrix/config.toml (project)");
    } else if error_msg.contains("agent") || error_msg.contains("backend") {
        eprintln!("Hint: Ensure the agent backend is properly configured.");
        eprintln!("Run 'ltmatrix --help' for more information.");
    }

    eprintln!();
    eprintln!("For more help, visit: https://github.com/bigfish/ltmatrix");
    eprintln!();
}

/// Print help information when no goal is provided
fn print_help() {
    println!("ltmatrix - Long-Time Agent Orchestrator");
    println!();
    println!("USAGE:");
    println!("  ltmatrix [OPTIONS] <GOAL>");
    println!("  ltmatrix [SUBCOMMAND]");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help           Print help information");
    println!("  -V, --version        Print version information");
    println!("  --fast               Fast execution mode");
    println!("  --expert             Expert execution mode");
    println!("  --dry-run            Generate plan without execution");
    println!("  --resume             Resume interrupted work");
    println!("  --ask                Ask for clarification before planning");
    println!("  --output <FORMAT>    Output format (text, json)");
    println!("  --log-level <LEVEL>  Log level (trace, debug, info, warn, error)");
    println!("  --log-file <FILE>    Log file path");
    println!();
    println!("SUBCOMMANDS:");
    println!("  release      Create a release build");
    println!("  completions  Generate shell completions");
    println!("  man          Generate man pages");
    println!("  cleanup      Clean up workspace state");
    println!();
    println!("EXAMPLES:");
    println!("  ltmatrix \"build a REST API\"");
    println!("  ltmatrix --fast \"add error handling\"");
    println!("  ltmatrix --expert \"implement authentication\"");
    println!("  ltmatrix --resume");
    println!("  ltmatrix cleanup --remove --force");
    println!("  ltmatrix completions bash");
    println!();
    println!("For more information, visit: https://github.com/bigfish/ltmatrix");
}

// =============================================================================
// Global State Management
// =============================================================================

/// Global log guard storage
///
/// We need to keep the log guard alive for the entire application lifetime.
/// This is a simple way to store it globally.
static GLOBAL_LOG_GUARD: std::sync::OnceLock<LogGuardWrapper> = std::sync::OnceLock::new();

/// Wrapper for the log guard
///
/// # Safety
///
/// `LogGuard` contains a `WorkerGuard` from `tracing-appender` which is thread-safe
/// internally but doesn't implement `Send + Sync`. This wrapper allows storing the
/// guard in a global static (`GLOBAL_LOG_GUARD`).
///
/// **Safety Invariant**: The guard is stored exactly once during application startup
/// via `set_global_log_guard()` and is never moved, mutated, or accessed from multiple
/// threads after initialization. The guard exists solely to keep the log file handle
/// alive for the application's lifetime.
struct LogGuardWrapper {
    _guard: Option<ltmatrix::logging::logger::LogGuard>,
}

// SAFETY: LogGuardWrapper is only stored once in GLOBAL_LOG_GUARD during startup
// and is never accessed concurrently. The inner LogGuard is not accessed after
// being stored - it exists only to keep the file handle alive.
unsafe impl Send for LogGuardWrapper {}
unsafe impl Sync for LogGuardWrapper {}

/// Set the global log guard
fn set_global_log_guard(guard: ltmatrix::logging::logger::LogGuard) -> Result<()> {
    GLOBAL_LOG_GUARD
        .set(LogGuardWrapper {
            _guard: Some(guard),
        })
        .map_err(|_| anyhow!("Global log guard already set"))?;
    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_flag() {
        assert!(!is_shutdown_requested());
        SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
        assert!(is_shutdown_requested());
        SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_print_help_does_not_panic() {
        print_help();
    }

    #[test]
    fn test_print_banner_with_subcommand() {
        let args = Args::try_parse_from(["ltmatrix", "completions", "bash"]).unwrap();
        print_banner(&args);
    }
}
