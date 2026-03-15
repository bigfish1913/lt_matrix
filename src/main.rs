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
use ltmatrix::workspace::WorkspaceState;

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
    // Use a robust handler that won't panic again if stderr is closed
    let _original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Try to print to stderr, but ignore errors (pipe might be closed on Windows)
        let _ = std::io::stderr().write_all(b"Application panic: ");
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            let _ = std::io::stderr().write_all(s.as_bytes());
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            let _ = std::io::stderr().write_all(s.as_bytes());
        }
        if let Some(location) = panic_info.location() {
            let _ = std::io::stderr().write_all(format!("\n at {}:{}", location.file(), location.line()).as_bytes());
        }
        let _ = std::io::stderr().write_all(b"\n");
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
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

    let mut write_out = |msg: &str| {
        let _ = std::io::stdout().write_all(msg.as_bytes());
    };

    write_out("ltmatrix - Long-Time Agent Orchestrator\n");
    write_out(&format!("Version: {}\n", env!("CARGO_PKG_VERSION")));

    if args.is_run_command() && args.goal.is_some() {
        write_out("\n");
    }

    let _ = std::io::stdout().flush();
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
    // For main pipeline (no command) and release command, disable console to not interfere with progress display
    let is_main_pipeline = args.command.is_none() || matches!(args.command, Some(Command::Release(_)));

    let log_manager = if args.log_file.is_some() {
        // User specified a specific log file
        let log_file = args.log_file.as_ref().unwrap();
        let _guard = logger::init_logging(log_level, Some(log_file.as_path()), !is_main_pipeline)
            .context("Failed to initialize logging with file")?;
        None
    } else if is_main_pipeline {
        // Use automatic log file management for main run and release commands
        // Disable console output to not interfere with progress display
        let base_dir = env::current_dir().context("Failed to get current directory")?;

        let (guard, manager) = logger::init_logging_with_management(log_level, Some(base_dir), false)
            .context("Failed to initialize logging with management")?;

        // The guard must be kept alive for the application lifetime
        // We store it in a static to prevent it from being dropped
        if let Err(e) = set_global_log_guard(guard) {
            // Can't use warn! here since logging might not be fully initialized
            eprintln!("Failed to set global log guard: {}", e);
        }

        Some(manager)
    } else {
        // Console-only logging for other subcommands
        let _guard = logger::init_logging(log_level, None::<&PathBuf>, true)
            .context("Failed to initialize console logging")?;
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
// Helper Functions
// =============================================================================

/// Safely write to stdout, ignoring pipe errors (common on Windows when pipe is closed)
fn safe_print(msg: &str) {
    let _ = std::io::stdout().write_all(msg.as_bytes());
}

/// Safely write a line to stdout
fn safe_println(msg: &str) {
    safe_print(msg);
    safe_print("\n");
}

/// Safely flush stdout
fn safe_flush() {
    let _ = std::io::stdout().flush();
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

    // Build goal from CLI args and/or file
    let goal = match (&args.goal, &args.file) {
        (Some(cli_goal), Some(file_path)) => {
            // Both provided: CLI goal as main, file as reference
            let file_content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read goal from file: {}", file_path.display()))?;

            info!("Goal from CLI: {}", cli_goal);
            info!("Reference file: {} ({} chars)", file_path.display(), file_content.len());

            // Combine: CLI goal as main instruction, file content as reference (not displayed)
            format!(
                "{}\n\n---\n参考文档内容:\n{}\n---",
                cli_goal, file_content
            )
        }
        (None, Some(file_path)) => {
            // File only
            let file_content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read goal from file: {}", file_path.display()))?;

            info!("Goal from file: {} ({} chars)", file_path.display(), file_content.len());
            file_content
        }
        (Some(cli_goal), None) => {
            // CLI only
            info!("Goal from CLI: {}", cli_goal);
            cli_goal.clone()
        }
        (None, None) => {
            print_help();
            return Ok(0);
        }
    };

    info!("Starting run command");
    info!("Mode: {:?}", args.get_execution_mode());

    // Determine execution mode
    let mode = args.get_execution_mode().to_model();

    // Build orchestrator configuration
    let work_dir = env::current_dir().context("Failed to get current directory")?;

    // Auto-detect existing tasks and ask to resume (unless --resume is explicitly set)
    let should_resume = if !args.resume && WorkspaceState::exists(&work_dir) {
        if let Ok(state) = WorkspaceState::load(work_dir.clone()) {
            let recovery = state.get_recovery_summary();
            if recovery.can_resume {
                let completed = state.tasks.iter().filter(|t| t.status == ltmatrix::models::TaskStatus::Completed).count();
                let total = state.tasks.len();
                let pending = recovery.total_incomplete;

                safe_println("");
                safe_println(&format!("\x1B[33m发现已有 {} 个任务 ({} 已完成, {} 待处理)\x1B[0m", total, completed, pending));
                safe_print("继续上次执行? [Y/n] ");
                safe_flush();

                // Read user input
                let mut input = String::new();
                match std::io::stdin().read_line(&mut input) {
                    Ok(_) => {
                        let answer = input.trim().to_lowercase();
                        if answer.is_empty() || answer == "y" || answer == "yes" {
                            true
                        } else {
                            safe_println("\x1B[33m清除现有任务，重新生成...\x1B[0m");
                            // Clear existing tasks
                            let manifest_path = work_dir.join(".ltmatrix").join("tasks-manifest.json");
                            if manifest_path.exists() {
                                std::fs::remove_file(&manifest_path).ok();
                            }
                            false
                        }
                    }
                    Err(_) => false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        args.resume
    };

    // Handle resume mode
    if should_resume {
        return execute_resume_from_run(app_state, work_dir, mode);
    }

    // Check for dry-run mode
    if args.dry_run {
        info!("Dry run mode: pipeline will be planned but not executed");
        safe_println("\nDry run mode - planning without execution");
        safe_println(&format!("Mode: {:?}", mode));
        safe_println("Remove --dry-run to execute the pipeline");
        safe_flush();
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
    // Use model from config file if specified
    let pipeline_config = &app_state.config.pipeline;
    let generation_model = &pipeline_config.generation_model;
    let assessment_model = &pipeline_config.assessment_model;

    debug!("Pipeline config from file: generation_model={}, assessment_model={}", generation_model, assessment_model);
    info!("Using models: generation={}, assessment={}", generation_model, assessment_model);

    let orchestrator_config = match mode {
        ExecutionMode::Fast => OrchestratorConfig::fast_mode()
            .with_work_dir(&work_dir)
            .with_agent_pool(agent_pool)
            .with_progress(app_state.config.output.progress)
            .with_pipeline_config(generation_model, assessment_model),
        ExecutionMode::Expert => OrchestratorConfig::expert_mode()
            .with_work_dir(&work_dir)
            .with_agent_pool(agent_pool)
            .with_progress(app_state.config.output.progress)
            .with_pipeline_config(generation_model, assessment_model),
        ExecutionMode::Standard => OrchestratorConfig::default()
            .with_work_dir(&work_dir)
            .with_agent_pool(agent_pool)
            .with_progress(app_state.config.output.progress)
            .with_pipeline_config(generation_model, assessment_model),
    };

    // Create pipeline orchestrator
    let orchestrator = PipelineOrchestrator::new(orchestrator_config)
        .context("Failed to create pipeline orchestrator")?;

    safe_println("\nStarting Pipeline Execution");
    safe_println(&format!("Mode: {}", mode));
    safe_flush();

    // Execute the pipeline
    let result = tokio::runtime::Runtime::new()
        .context("Failed to create async runtime")?
        .block_on(orchestrator.execute_pipeline(&goal, mode))
        .context("Pipeline execution failed")?;

    // Print results
    safe_println("");
    safe_println("Pipeline Execution Summary:");
    safe_println(&format!("  Total tasks: {}", result.total_tasks));
    safe_println(&format!("  Completed: {}", result.tasks_completed));
    safe_println(&format!("  Failed: {}", result.tasks_failed));
    safe_println(&format!("  Stages completed: {}", result.stages_completed));
    safe_println(&format!("  Success rate: {:.1}%", result.success_rate()));
    safe_println(&format!("  Total time: {:.2}s", result.total_time.as_secs_f64()));
    safe_println("");
    safe_flush();

    // Return appropriate exit code
    if result.success {
        Ok(0)
    } else {
        Ok(1)
    }
}

/// Execute resume from within run command (auto-detected or user requested)
#[instrument(skip(app_state))]
fn execute_resume_from_run(app_state: &mut AppState, work_dir: PathBuf, mode: ExecutionMode) -> Result<i32> {
    info!("Resuming pipeline from previous run");

    // Load workspace state with transformation
    let state = WorkspaceState::load_with_transform(work_dir.clone())
        .context("Failed to load workspace state")?;

    let recovery = state.get_recovery_summary();
    if !recovery.can_resume {
        safe_println("No incomplete tasks found, nothing to resume");
        return Ok(0);
    }

    // Display workspace status
    let summary = state.status_summary();
    safe_println("\n\x1B[36m恢复执行\x1B[0m");
    safe_println(&format!("  总任务: {}", summary.total()));
    safe_println(&format!("  已完成: {}", summary.completed));
    safe_println(&format!("  待处理: {}", summary.pending));
    safe_println(&format!("  失败: {} (将重试)", summary.failed));
    safe_println(&format!("  进度: {:.1}%", summary.completion_percentage()));
    safe_println("");
    safe_flush();

    // Get agent pool
    let agent_pool = app_state
        .agent_pool
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Agent pool not initialized"))?;

    // Create orchestrator config
    let pipeline_config = &app_state.config.pipeline;
    let generation_model = &pipeline_config.generation_model;
    let assessment_model = &pipeline_config.assessment_model;

    let orchestrator_config = match mode {
        ExecutionMode::Fast => OrchestratorConfig::fast_mode()
            .with_work_dir(&work_dir)
            .with_agent_pool(agent_pool)
            .with_progress(app_state.config.output.progress)
            .with_pipeline_config(generation_model, assessment_model),
        ExecutionMode::Expert => OrchestratorConfig::expert_mode()
            .with_work_dir(&work_dir)
            .with_agent_pool(agent_pool)
            .with_progress(app_state.config.output.progress)
            .with_pipeline_config(generation_model, assessment_model),
        ExecutionMode::Standard => OrchestratorConfig::default()
            .with_work_dir(&work_dir)
            .with_agent_pool(agent_pool)
            .with_progress(app_state.config.output.progress)
            .with_pipeline_config(generation_model, assessment_model),
    };

    // Create orchestrator
    let orchestrator = PipelineOrchestrator::new(orchestrator_config)
        .context("Failed to create pipeline orchestrator")?;

    // Resume pipeline
    let result = tokio::runtime::Runtime::new()
        .context("Failed to create async runtime")?
        .block_on(orchestrator.resume_pipeline())
        .context("Pipeline resume failed")?;

    // Print results
    safe_println("");
    safe_println("Pipeline Execution Summary:");
    safe_println(&format!("  Total tasks: {}", result.total_tasks));
    safe_println(&format!("  Completed: {}", result.tasks_completed));
    safe_println(&format!("  Failed: {}", result.tasks_failed));
    safe_println(&format!("  Stages completed: {}", result.stages_completed));
    safe_println(&format!("  Success rate: {:.1}%", result.success_rate()));
    safe_println(&format!("  Total time: {:.2}s", result.total_time.as_secs_f64()));
    safe_println("");
    safe_flush();

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
    // Use a helper to safely write to stderr, ignoring pipe errors on Windows
    let mut write_err = |msg: &str| {
        let _ = std::io::stderr().write_all(msg.as_bytes());
    };

    write_err("\n");
    write_err("Error:\n");

    // Print the error chain
    for (i, cause) in error.chain().enumerate() {
        if i == 0 {
            write_err("   ");
            write_err(&cause.to_string());
            write_err("\n");
        } else {
            write_err("   Caused by: ");
            write_err(&cause.to_string());
            write_err("\n");
        }
    }

    write_err("\n");

    // Print helpful hints based on error content
    let error_msg = error.to_string().to_lowercase();
    if error_msg.contains("permission") || error_msg.contains("access") {
        write_err("Hint: Check file permissions and try running with appropriate access.\n");
    } else if error_msg.contains("network") || error_msg.contains("connection") {
        write_err("Hint: Check your internet connection and try again.\n");
    } else if error_msg.contains("config") || error_msg.contains("configuration") {
        write_err("Hint: Check your configuration file at:\n");
        write_err("  - ~/.config/ltmatrix/config.toml (global)\n");
        write_err("  - .ltmatrix/config.toml (project)\n");
    } else if error_msg.contains("agent") || error_msg.contains("backend") {
        write_err("Hint: Ensure the agent backend is properly configured.\n");
        write_err("Run 'ltmatrix --help' for more information.\n");
    }

    write_err("\n");
    write_err("For more help, visit: https://github.com/bigfish/ltmatrix\n");
    write_err("\n");

    // Flush stderr, ignoring any errors
    let _ = std::io::stderr().flush();
}

/// Print help information when no goal is provided
fn print_help() {
    // Use a helper to safely write to stdout, ignoring pipe errors
    let mut write_out = |msg: &str| {
        let _ = std::io::stdout().write_all(msg.as_bytes());
    };

    write_out("ltmatrix - Long-Time Agent Orchestrator\n");
    write_out("\n");
    write_out("USAGE:\n");
    write_out("  ltmatrix [OPTIONS] <GOAL>\n");
    write_out("  ltmatrix [SUBCOMMAND]\n");
    write_out("\n");
    write_out("OPTIONS:\n");
    write_out("  -h, --help           Print help information\n");
    write_out("  -V, --version        Print version information\n");
    write_out("  --fast               Fast execution mode\n");
    write_out("  --expert             Expert execution mode\n");
    write_out("  --dry-run            Generate plan without execution\n");
    write_out("  --resume             Resume interrupted work\n");
    write_out("  --ask                Ask for clarification before planning\n");
    write_out("  --output <FORMAT>    Output format (text, json)\n");
    write_out("  --log-level <LEVEL>  Log level (trace, debug, info, warn, error)\n");
    write_out("  --log-file <FILE>    Log file path\n");
    write_out("\n");
    write_out("SUBCOMMANDS:\n");
    write_out("  release      Create a release build\n");
    write_out("  completions  Generate shell completions\n");
    write_out("  man          Generate man pages\n");
    write_out("  cleanup      Clean up workspace state\n");
    write_out("\n");
    write_out("EXAMPLES:\n");
    write_out("  ltmatrix \"build a REST API\"\n");
    write_out("  ltmatrix --fast \"add error handling\"\n");
    write_out("  ltmatrix --expert \"implement authentication\"\n");
    write_out("  ltmatrix --resume\n");
    write_out("  ltmatrix cleanup --remove --force\n");
    write_out("  ltmatrix completions bash\n");
    write_out("\n");
    write_out("For more information, visit: https://github.com/bigfish/ltmatrix\n");

    // Flush stdout, ignoring any errors
    let _ = std::io::stdout().flush();
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
