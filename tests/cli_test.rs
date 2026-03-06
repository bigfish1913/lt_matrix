//! Comprehensive tests for CLI argument parser implementation
//!
//! This test suite verifies:
//! - All CLI arguments and flags parsing
//! - Subcommands (run, release, completions)
//! - Conflict resolution between mutually exclusive flags
//! - Enum value parsing (ExecutionMode, OutputFormat, LogLevel, BlockedStrategy, Shell)
//! - Edge cases and error conditions
//! - Integration with command execution

use clap::Parser;
use ltmatrix::cli::args::{
    Args, BlockedStrategy, Command, ExecutionModeArg, LogLevel, OutputFormat, Shell,
};
use ltmatrix::cli::command::execute_command;
use std::path::PathBuf;

// =============================================================================
// Basic Argument Parsing Tests
// =============================================================================

#[test]
fn test_parse_no_arguments() {
    let args = Args::try_parse_from(["ltmatrix"]).unwrap();
    assert!(args.goal.is_none());
    assert!(!args.fast);
    assert!(!args.expert);
    assert!(!args.dry_run);
    assert!(!args.resume);
    assert!(!args.ask);
    assert!(!args.regenerate_plan);
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Standard);
}

#[test]
fn test_parse_goal_only() {
    let args = Args::try_parse_from(["ltmatrix", "build a REST API"]).unwrap();
    assert_eq!(args.goal, Some("build a REST API".to_string()));
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Standard);
}

#[test]
fn test_parse_goal_with_quotes() {
    let args =
        Args::try_parse_from(["ltmatrix", "add \"error handling\" to all endpoints"]).unwrap();
    assert_eq!(
        args.goal,
        Some("add \"error handling\" to all endpoints".to_string())
    );
}

// =============================================================================
// Execution Mode Tests
// =============================================================================

#[test]
fn test_fast_mode_flag() {
    let args = Args::try_parse_from(["ltmatrix", "--fast", "quick fix"]).unwrap();
    assert!(args.fast);
    assert!(!args.expert);
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Fast);
    assert!(args.goal.is_some());
}

#[test]
fn test_expert_mode_flag() {
    let args = Args::try_parse_from(["ltmatrix", "--expert", "complex feature"]).unwrap();
    assert!(args.expert);
    assert!(!args.fast);
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Expert);
    assert!(args.goal.is_some());
}

#[test]
fn test_mode_standard() {
    let args = Args::try_parse_from(["ltmatrix", "--mode", "standard", "task"]).unwrap();
    assert_eq!(args.mode, Some(ExecutionModeArg::Standard));
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Standard);
}

#[test]
fn test_mode_fast() {
    let args = Args::try_parse_from(["ltmatrix", "--mode", "fast", "task"]).unwrap();
    assert_eq!(args.mode, Some(ExecutionModeArg::Fast));
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Fast);
}

#[test]
fn test_mode_expert() {
    let args = Args::try_parse_from(["ltmatrix", "--mode", "expert", "task"]).unwrap();
    assert_eq!(args.mode, Some(ExecutionModeArg::Expert));
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Expert);
}

#[test]
fn test_fast_and_mode_conflict() {
    let result = Args::try_parse_from(["ltmatrix", "--fast", "--mode", "standard", "task"]);
    assert!(result.is_err(), "Should reject --fast with --mode");
}

#[test]
fn test_expert_and_mode_conflict() {
    let result = Args::try_parse_from(["ltmatrix", "--expert", "--mode", "fast", "task"]);
    assert!(result.is_err(), "Should reject --expert with --mode");
}

#[test]
fn test_fast_and_expert_conflict() {
    let result = Args::try_parse_from(["ltmatrix", "--fast", "--expert", "task"]);
    assert!(result.is_err(), "Should reject --fast with --expert");
}

// =============================================================================
// Agent and Configuration Tests
// =============================================================================

#[test]
fn test_agent_flag() {
    let args = Args::try_parse_from(["ltmatrix", "--agent", "claude-opus", "task"]).unwrap();
    assert_eq!(args.agent, Some("claude-opus".to_string()));
}

#[test]
fn test_config_short_flag() {
    let args = Args::try_parse_from(["ltmatrix", "-c", "/path/to/config.toml", "task"]).unwrap();
    assert_eq!(args.config, Some(PathBuf::from("/path/to/config.toml")));
}

#[test]
fn test_config_long_flag() {
    let args = Args::try_parse_from(["ltmatrix", "--config", "./custom.toml", "task"]).unwrap();
    assert_eq!(args.config, Some(PathBuf::from("./custom.toml")));
}

// =============================================================================
// Output Format Tests
// =============================================================================

#[test]
fn test_output_text() {
    let args = Args::try_parse_from(["ltmatrix", "--output", "text", "task"]).unwrap();
    assert_eq!(args.output, Some(OutputFormat::Text));
}

#[test]
fn test_output_json() {
    let args = Args::try_parse_from(["ltmatrix", "--output", "json", "task"]).unwrap();
    assert_eq!(args.output, Some(OutputFormat::Json));
}

#[test]
fn test_output_json_compact() {
    let args = Args::try_parse_from(["ltmatrix", "--output", "json-compact", "task"]).unwrap();
    assert_eq!(args.output, Some(OutputFormat::JsonCompact));
}

#[test]
fn test_output_format_display() {
    assert_eq!(OutputFormat::Text.to_string(), "text");
    assert_eq!(OutputFormat::Json.to_string(), "json");
    assert_eq!(OutputFormat::JsonCompact.to_string(), "json-compact");
}

// =============================================================================
// Log Level Tests
// =============================================================================

#[test]
fn test_log_level_trace() {
    let args = Args::try_parse_from(["ltmatrix", "--log-level", "trace", "task"]).unwrap();
    assert_eq!(args.log_level, Some(LogLevel::Trace));
}

#[test]
fn test_log_level_debug() {
    let args = Args::try_parse_from(["ltmatrix", "--log-level", "debug", "task"]).unwrap();
    assert_eq!(args.log_level, Some(LogLevel::Debug));
}

#[test]
fn test_log_level_info() {
    let args = Args::try_parse_from(["ltmatrix", "--log-level", "info", "task"]).unwrap();
    assert_eq!(args.log_level, Some(LogLevel::Info));
}

#[test]
fn test_log_level_warn() {
    let args = Args::try_parse_from(["ltmatrix", "--log-level", "warn", "task"]).unwrap();
    assert_eq!(args.log_level, Some(LogLevel::Warn));
}

#[test]
fn test_log_level_error() {
    let args = Args::try_parse_from(["ltmatrix", "--log-level", "error", "task"]).unwrap();
    assert_eq!(args.log_level, Some(LogLevel::Error));
}

#[test]
fn test_log_level_display() {
    assert_eq!(LogLevel::Trace.to_string(), "trace");
    assert_eq!(LogLevel::Debug.to_string(), "debug");
    assert_eq!(LogLevel::Info.to_string(), "info");
    assert_eq!(LogLevel::Warn.to_string(), "warn");
    assert_eq!(LogLevel::Error.to_string(), "error");
}

#[test]
fn test_log_level_to_tracing_level() {
    use tracing::Level;
    assert_eq!(Level::from(LogLevel::Trace), Level::TRACE);
    assert_eq!(Level::from(LogLevel::Debug), Level::DEBUG);
    assert_eq!(Level::from(LogLevel::Info), Level::INFO);
    assert_eq!(Level::from(LogLevel::Warn), Level::WARN);
    assert_eq!(Level::from(LogLevel::Error), Level::ERROR);
}

#[test]
fn test_log_file_flag() {
    let args =
        Args::try_parse_from(["ltmatrix", "--log-file", "/var/log/ltmatrix.log", "task"]).unwrap();
    assert_eq!(args.log_file, Some(PathBuf::from("/var/log/ltmatrix.log")));
}

// =============================================================================
// Execution Parameters Tests
// =============================================================================

#[test]
fn test_max_retries() {
    let args = Args::try_parse_from(["ltmatrix", "--max-retries", "5", "task"]).unwrap();
    assert_eq!(args.max_retries, Some(5));
}

#[test]
fn test_timeout_seconds() {
    let args = Args::try_parse_from(["ltmatrix", "--timeout", "7200", "task"]).unwrap();
    assert_eq!(args.timeout, Some(7200));
}

#[test]
fn test_dry_run_flag() {
    let args = Args::try_parse_from(["ltmatrix", "--dry-run", "task"]).unwrap();
    assert!(args.dry_run);
}

#[test]
fn test_resume_flag() {
    let args = Args::try_parse_from(["ltmatrix", "--resume"]).unwrap();
    assert!(args.resume);
}

#[test]
fn test_ask_flag() {
    let args = Args::try_parse_from(["ltmatrix", "--ask", "task"]).unwrap();
    assert!(args.ask);
}

#[test]
fn test_regenerate_plan_flag() {
    let args = Args::try_parse_from(["ltmatrix", "--regenerate-plan", "task"]).unwrap();
    assert!(args.regenerate_plan);
}

// =============================================================================
// Blocked Strategy Tests
// =============================================================================

#[test]
fn test_on_blocked_skip() {
    let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "skip", "task"]).unwrap();
    assert_eq!(args.on_blocked, Some(BlockedStrategy::Skip));
}

#[test]
fn test_on_blocked_ask() {
    let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "ask", "task"]).unwrap();
    assert_eq!(args.on_blocked, Some(BlockedStrategy::Ask));
}

#[test]
fn test_on_blocked_abort() {
    let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "abort", "task"]).unwrap();
    assert_eq!(args.on_blocked, Some(BlockedStrategy::Abort));
}

#[test]
fn test_on_blocked_retry() {
    let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "retry", "task"]).unwrap();
    assert_eq!(args.on_blocked, Some(BlockedStrategy::Retry));
}

#[test]
fn test_blocked_strategy_display() {
    assert_eq!(BlockedStrategy::Skip.to_string(), "skip");
    assert_eq!(BlockedStrategy::Ask.to_string(), "ask");
    assert_eq!(BlockedStrategy::Abort.to_string(), "abort");
    assert_eq!(BlockedStrategy::Retry.to_string(), "retry");
}

// =============================================================================
// MCP Configuration Tests
// =============================================================================

#[test]
fn test_mcp_config_flag() {
    let args =
        Args::try_parse_from(["ltmatrix", "--mcp-config", "/path/to/mcp.json", "task"]).unwrap();
    assert_eq!(args.mcp_config, Some(PathBuf::from("/path/to/mcp.json")));
}

// =============================================================================
// Subcommand Tests
// =============================================================================

#[test]
fn test_default_run_command() {
    let args = Args::try_parse_from(["ltmatrix", "task"]).unwrap();
    assert!(args.is_run_command());
    assert!(args.command.is_none());
}

#[test]
fn test_release_subcommand() {
    let args = Args::try_parse_from(["ltmatrix", "release"]).unwrap();
    assert!(!args.is_run_command());
    assert!(matches!(args.command, Some(Command::Release(_))));
}

#[test]
fn test_release_with_target() {
    let args = Args::try_parse_from([
        "ltmatrix",
        "release",
        "--target",
        "x86_64-unknown-linux-musl",
    ])
    .unwrap();
    if let Some(Command::Release(release_args)) = args.command {
        assert_eq!(
            release_args.target,
            Some("x86_64-unknown-linux-musl".to_string())
        );
    } else {
        panic!("Expected Release command");
    }
}

#[test]
fn test_release_with_output() {
    let args = Args::try_parse_from(["ltmatrix", "release", "--output", "./build"]).unwrap();
    if let Some(Command::Release(release_args)) = args.command {
        assert_eq!(release_args.output, PathBuf::from("./build"));
    } else {
        panic!("Expected Release command");
    }
}

#[test]
fn test_release_with_archive() {
    let args = Args::try_parse_from(["ltmatrix", "release", "--archive"]).unwrap();
    if let Some(Command::Release(release_args)) = args.command {
        assert!(release_args.archive);
    } else {
        panic!("Expected Release command");
    }
}

#[test]
fn test_release_with_all_targets() {
    let args = Args::try_parse_from(["ltmatrix", "release", "--all-targets"]).unwrap();
    if let Some(Command::Release(release_args)) = args.command {
        assert!(release_args.all_targets);
    } else {
        panic!("Expected Release command");
    }
}

#[test]
fn test_release_default_output() {
    let args = Args::try_parse_from(["ltmatrix", "release"]).unwrap();
    if let Some(Command::Release(release_args)) = args.command {
        assert_eq!(release_args.output, PathBuf::from("./dist"));
    } else {
        panic!("Expected Release command");
    }
}

#[test]
fn test_completions_subcommand_bash() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "bash"]).unwrap();
    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::Bash);
    } else {
        panic!("Expected Completions command");
    }
}

#[test]
fn test_completions_subcommand_zsh() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "zsh"]).unwrap();
    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::Zsh);
    } else {
        panic!("Expected Completions command");
    }
}

#[test]
fn test_completions_subcommand_fish() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "fish"]).unwrap();
    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::Fish);
    } else {
        panic!("Expected Completions command");
    }
}

#[test]
fn test_completions_subcommand_powershell() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "powershell"]).unwrap();
    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::PowerShell);
    } else {
        panic!("Expected Completions command");
    }
}

#[test]
fn test_completions_subcommand_elvish() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "elvish"]).unwrap();
    if let Some(Command::Completions(completions_args)) = args.command {
        assert_eq!(completions_args.shell, Shell::Elvish);
    } else {
        panic!("Expected Completions command");
    }
}

#[test]
fn test_shell_display() {
    assert_eq!(Shell::Bash.to_string(), "bash");
    assert_eq!(Shell::Zsh.to_string(), "zsh");
    assert_eq!(Shell::Fish.to_string(), "fish");
    assert_eq!(Shell::PowerShell.to_string(), "powershell");
    assert_eq!(Shell::Elvish.to_string(), "elvish");
}

// =============================================================================
// Complex Multi-Flag Tests
// =============================================================================

#[test]
fn test_multiple_flags_combination() {
    let args = Args::try_parse_from([
        "ltmatrix",
        "--fast",
        "--output",
        "json",
        "--log-level",
        "debug",
        "--max-retries",
        "3",
        "--timeout",
        "1800",
        "--dry-run",
        "complex task",
    ])
    .unwrap();

    assert!(args.fast);
    assert_eq!(args.output, Some(OutputFormat::Json));
    assert_eq!(args.log_level, Some(LogLevel::Debug));
    assert_eq!(args.max_retries, Some(3));
    assert_eq!(args.timeout, Some(1800));
    assert!(args.dry_run);
    assert_eq!(args.goal, Some("complex task".to_string()));
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Fast);
}

#[test]
fn test_expert_mode_with_all_options() {
    let args = Args::try_parse_from([
        "ltmatrix",
        "--expert",
        "--agent",
        "claude-opus",
        "--config",
        "/custom/config.toml",
        "--output",
        "json-compact",
        "--log-level",
        "trace",
        "--log-file",
        "/var/log/ltmatrix.log",
        "--max-retries",
        "5",
        "--timeout",
        "7200",
        "--on-blocked",
        "ask",
        "--mcp-config",
        "/custom/mcp.json",
        "--regenerate-plan",
        "expert task",
    ])
    .unwrap();

    assert!(args.expert);
    assert_eq!(args.agent, Some("claude-opus".to_string()));
    assert_eq!(args.config, Some(PathBuf::from("/custom/config.toml")));
    assert_eq!(args.output, Some(OutputFormat::JsonCompact));
    assert_eq!(args.log_level, Some(LogLevel::Trace));
    assert_eq!(args.log_file, Some(PathBuf::from("/var/log/ltmatrix.log")));
    assert_eq!(args.max_retries, Some(5));
    assert_eq!(args.timeout, Some(7200));
    assert_eq!(args.on_blocked, Some(BlockedStrategy::Ask));
    assert_eq!(args.mcp_config, Some(PathBuf::from("/custom/mcp.json")));
    assert!(args.regenerate_plan);
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Expert);
}

// =============================================================================
// Edge Cases and Error Conditions
// =============================================================================

#[test]
fn test_invalid_output_format() {
    let result = Args::try_parse_from(["ltmatrix", "--output", "invalid", "task"]);
    assert!(result.is_err(), "Should reject invalid output format");
}

#[test]
fn test_invalid_log_level() {
    let result = Args::try_parse_from(["ltmatrix", "--log-level", "verbose", "task"]);
    assert!(result.is_err(), "Should reject invalid log level");
}

#[test]
fn test_invalid_blocked_strategy() {
    let result = Args::try_parse_from(["ltmatrix", "--on-blocked", "ignore", "task"]);
    assert!(result.is_err(), "Should reject invalid blocked strategy");
}

#[test]
fn test_invalid_shell() {
    let result = Args::try_parse_from(["ltmatrix", "completions", "invalid-shell"]);
    assert!(result.is_err(), "Should reject invalid shell type");
}

#[test]
fn test_negative_max_retries_rejected() {
    let result = Args::try_parse_from(["ltmatrix", "--max-retries", "-1", "task"]);
    // clap should reject negative values for u32
    assert!(result.is_err());
}

#[test]
fn test_zero_timeout_accepted() {
    let args = Args::try_parse_from(["ltmatrix", "--timeout", "0", "task"]).unwrap();
    assert_eq!(args.timeout, Some(0));
}

#[test]
fn test_empty_goal_string() {
    let args = Args::try_parse_from(["ltmatrix", ""]).unwrap();
    assert_eq!(args.goal, Some("".to_string()));
}

#[test]
fn test_goal_with_special_characters() {
    let goal = "fix regex: s/([a-z]+)/\\U$1/g";
    let args = Args::try_parse_from(["ltmatrix", goal]).unwrap();
    assert_eq!(args.goal, Some(goal.to_string()));
}

#[test]
fn test_goal_with_unicode() {
    let goal = "add emoji support 🎉 for internationalization";
    let args = Args::try_parse_from(["ltmatrix", goal]).unwrap();
    assert_eq!(args.goal, Some(goal.to_string()));
}

#[test]
fn test_execution_mode_display() {
    assert_eq!(ExecutionModeArg::Fast.to_string(), "fast");
    assert_eq!(ExecutionModeArg::Standard.to_string(), "standard");
    assert_eq!(ExecutionModeArg::Expert.to_string(), "expert");
}

#[test]
fn test_execution_mode_conversion_to_model() {
    use ltmatrix::models::ExecutionMode;
    assert_eq!(ExecutionModeArg::Fast.to_model(), ExecutionMode::Fast);
    assert_eq!(
        ExecutionModeArg::Standard.to_model(),
        ExecutionMode::Standard
    );
    assert_eq!(ExecutionModeArg::Expert.to_model(), ExecutionMode::Expert);
}

// =============================================================================
// Command Execution Tests
// =============================================================================

#[test]
fn test_execute_run_command_success() {
    let args = Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();
    let result = execute_command(args);
    assert!(result.is_ok(), "Should execute run command successfully");
}

#[test]
fn test_execute_run_without_goal() {
    let args = Args::try_parse_from(["ltmatrix"]).unwrap();
    let result = execute_command(args);
    assert!(result.is_ok(), "Should execute without goal (shows help)");
}

#[test]
fn test_execute_release_command() {
    let args = Args::try_parse_from(["ltmatrix", "release"]).unwrap();
    let result = execute_command(args);
    assert!(
        result.is_ok(),
        "Should execute release command successfully"
    );
}

#[test]
fn test_execute_release_with_options() {
    let args = Args::try_parse_from([
        "ltmatrix",
        "release",
        "--target",
        "x86_64-pc-windows-msvc",
        "--output",
        "./dist",
        "--archive",
    ])
    .unwrap();
    let result = execute_command(args);
    assert!(
        result.is_ok(),
        "Should execute release with options successfully"
    );
}

#[test]
fn test_execute_completions_command() {
    let args = Args::try_parse_from(["ltmatrix", "completions", "bash"]).unwrap();
    let result = execute_command(args);
    assert!(
        result.is_ok(),
        "Should execute completions command successfully"
    );
}

#[test]
fn test_execute_completions_all_shells() {
    let shells = vec!["bash", "zsh", "fish", "powershell", "elvish"];

    for shell in shells {
        let args = Args::try_parse_from(["ltmatrix", "completions", shell]).unwrap();
        let result = execute_command(args);
        assert!(
            result.is_ok(),
            "Should execute completions for {} successfully",
            shell
        );
    }
}

// =============================================================================
// Path Handling Tests
// =============================================================================

#[test]
fn test_config_path_relative() {
    let args = Args::try_parse_from(["ltmatrix", "--config", "config.toml", "task"]).unwrap();
    assert_eq!(args.config, Some(PathBuf::from("config.toml")));
}

#[test]
fn test_config_path_absolute() {
    let args = Args::try_parse_from(["ltmatrix", "--config", "/etc/ltmatrix/config.toml", "task"])
        .unwrap();
    assert_eq!(
        args.config,
        Some(PathBuf::from("/etc/ltmatrix/config.toml"))
    );
}

#[test]
fn test_config_path_with_dots() {
    let args = Args::try_parse_from(["ltmatrix", "--config", "../config.toml", "task"]).unwrap();
    assert_eq!(args.config, Some(PathBuf::from("../config.toml")));
}

#[test]
fn test_log_file_path_with_tilde() {
    // Note: This will be literal "~" - expansion is handled by the shell
    let args =
        Args::try_parse_from(["ltmatrix", "--log-file", "~/logs/ltmatrix.log", "task"]).unwrap();
    assert_eq!(args.log_file, Some(PathBuf::from("~/logs/ltmatrix.log")));
}

#[test]
fn test_release_output_path_with_parent_dots() {
    let args = Args::try_parse_from(["ltmatrix", "release", "--output", "../dist"]).unwrap();
    if let Some(Command::Release(release_args)) = args.command {
        assert_eq!(release_args.output, PathBuf::from("../dist"));
    } else {
        panic!("Expected Release command");
    }
}

// =============================================================================
// Flag Interaction Tests
// =============================================================================

#[test]
fn test_resume_without_goal() {
    let args = Args::try_parse_from(["ltmatrix", "--resume"]).unwrap();
    assert!(args.resume);
    assert!(args.goal.is_none());
}

#[test]
fn test_dry_run_with_goal() {
    let args = Args::try_parse_from(["ltmatrix", "--dry-run", "plan only"]).unwrap();
    assert!(args.dry_run);
    assert_eq!(args.goal, Some("plan only".to_string()));
}

#[test]
fn test_ask_with_expert_mode() {
    let args = Args::try_parse_from(["ltmatrix", "--expert", "--ask", "task"]).unwrap();
    assert!(args.ask);
    assert!(args.expert);
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Expert);
}

#[test]
fn test_regenerate_plan_with_fast_mode() {
    let args = Args::try_parse_from(["ltmatrix", "--fast", "--regenerate-plan", "task"]).unwrap();
    assert!(args.regenerate_plan);
    assert!(args.fast);
    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Fast);
}

// =============================================================================
// Integration-Style Tests
// =============================================================================

#[test]
fn test_realistic_fast_development_workflow() {
    let args = Args::try_parse_from([
        "ltmatrix",
        "--fast",
        "--output",
        "json",
        "--log-level",
        "warn",
        "add user authentication",
    ])
    .unwrap();

    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Fast);
    assert_eq!(args.output, Some(OutputFormat::Json));
    assert_eq!(args.log_level, Some(LogLevel::Warn));
    assert_eq!(args.goal, Some("add user authentication".to_string()));
}

#[test]
fn test_realistic_expert_development_workflow() {
    let args = Args::try_parse_from([
        "ltmatrix",
        "--expert",
        "--agent",
        "claude-opus",
        "--log-level",
        "debug",
        "--max-retries",
        "5",
        "--timeout",
        "7200",
        "--on-blocked",
        "ask",
        "implement distributed caching system",
    ])
    .unwrap();

    assert_eq!(args.get_execution_mode(), ExecutionModeArg::Expert);
    assert_eq!(args.agent, Some("claude-opus".to_string()));
    assert_eq!(args.log_level, Some(LogLevel::Debug));
    assert_eq!(args.max_retries, Some(5));
    assert_eq!(args.timeout, Some(7200));
    assert_eq!(args.on_blocked, Some(BlockedStrategy::Ask));
    assert_eq!(
        args.goal,
        Some("implement distributed caching system".to_string())
    );
}

#[test]
fn test_realistic_release_workflow() {
    let args = Args::try_parse_from([
        "ltmatrix",
        "release",
        "--target",
        "x86_64-unknown-linux-gnu",
        "--output",
        "./release",
        "--archive",
    ])
    .unwrap();

    if let Some(Command::Release(release_args)) = args.command {
        assert_eq!(
            release_args.target,
            Some("x86_64-unknown-linux-gnu".to_string())
        );
        assert_eq!(release_args.output, PathBuf::from("./release"));
        assert!(release_args.archive);
    } else {
        panic!("Expected Release command");
    }
}

#[test]
fn test_realistic_debugging_workflow() {
    let args = Args::try_parse_from([
        "ltmatrix",
        "--log-level",
        "trace",
        "--log-file",
        "debug.log",
        "--on-blocked",
        "ask",
        "--regenerate-plan",
        "fix memory leak in worker pool",
    ])
    .unwrap();

    assert_eq!(args.log_level, Some(LogLevel::Trace));
    assert_eq!(args.log_file, Some(PathBuf::from("debug.log")));
    assert_eq!(args.on_blocked, Some(BlockedStrategy::Ask));
    assert!(args.regenerate_plan);
    assert_eq!(
        args.goal,
        Some("fix memory leak in worker pool".to_string())
    );
}

#[test]
fn test_resuming_interrupted_work() {
    let args = Args::try_parse_from([
        "ltmatrix",
        "--resume",
        "--log-level",
        "info",
        "--on-blocked",
        "retry",
    ])
    .unwrap();

    assert!(args.resume);
    assert_eq!(args.log_level, Some(LogLevel::Info));
    assert_eq!(args.on_blocked, Some(BlockedStrategy::Retry));
    assert!(args.goal.is_none());
}

// =============================================================================
// Help and Version Tests
// =============================================================================

#[test]
fn test_version_flag() {
    let result = Args::try_parse_from(["ltmatrix", "--version"]);
    // Version flag causes clap to exit, so this will fail
    assert!(result.is_err());
}

#[test]
fn test_help_flag() {
    let result = Args::try_parse_from(["ltmatrix", "--help"]);
    // Help flag causes clap to exit, so this will fail
    assert!(result.is_err());
}

#[test]
fn test_short_help_flag() {
    let result = Args::try_parse_from(["ltmatrix", "-h"]);
    // Help flag causes clap to exit, so this will fail
    assert!(result.is_err());
}

// =============================================================================
// Validation Tests
// =============================================================================

#[test]
fn test_execution_mode_priority() {
    // Mode flag should take priority
    let args1 = Args::try_parse_from(["ltmatrix", "--mode", "fast", "task"]).unwrap();
    assert_eq!(args1.get_execution_mode(), ExecutionModeArg::Fast);

    // --fast should work when --mode is not specified
    let args2 = Args::try_parse_from(["ltmatrix", "--fast", "task"]).unwrap();
    assert_eq!(args2.get_execution_mode(), ExecutionModeArg::Fast);

    // --expert should work when --mode is not specified
    let args3 = Args::try_parse_from(["ltmatrix", "--expert", "task"]).unwrap();
    assert_eq!(args3.get_execution_mode(), ExecutionModeArg::Expert);

    // Standard when no mode specified
    let args4 = Args::try_parse_from(["ltmatrix", "task"]).unwrap();
    assert_eq!(args4.get_execution_mode(), ExecutionModeArg::Standard);
}

#[test]
fn test_subcommand_excludes_goal() {
    // Release subcommand should not accept goal
    let args = Args::try_parse_from(["ltmatrix", "release"]).unwrap();
    assert!(args.goal.is_none());
    assert!(matches!(args.command, Some(Command::Release(_))));

    // Completions subcommand should not accept goal
    let args = Args::try_parse_from(["ltmatrix", "completions", "bash"]).unwrap();
    assert!(args.goal.is_none());
    assert!(matches!(args.command, Some(Command::Completions(_))));
}

#[test]
fn test_all_execution_modes_are_distinct() {
    let modes = vec![
        ExecutionModeArg::Fast,
        ExecutionModeArg::Standard,
        ExecutionModeArg::Expert,
    ];

    for (i, mode1) in modes.iter().enumerate() {
        for (j, mode2) in modes.iter().enumerate() {
            if i != j {
                assert_ne!(mode1, mode2, "Modes should be distinct");
            }
        }
    }
}

#[test]
fn test_all_output_formats_are_distinct() {
    let formats = vec![
        OutputFormat::Text,
        OutputFormat::Json,
        OutputFormat::JsonCompact,
    ];

    for (i, format1) in formats.iter().enumerate() {
        for (j, format2) in formats.iter().enumerate() {
            if i != j {
                assert_ne!(format1, format2, "Formats should be distinct");
            }
        }
    }
}

#[test]
fn test_all_log_levels_are_distinct() {
    let levels = vec![
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];

    for (i, level1) in levels.iter().enumerate() {
        for (j, level2) in levels.iter().enumerate() {
            if i != j {
                assert_ne!(level1, level2, "Log levels should be distinct");
            }
        }
    }
}

#[test]
fn test_all_blocked_strategies_are_distinct() {
    let strategies = vec![
        BlockedStrategy::Skip,
        BlockedStrategy::Ask,
        BlockedStrategy::Abort,
        BlockedStrategy::Retry,
    ];

    for (i, strategy1) in strategies.iter().enumerate() {
        for (j, strategy2) in strategies.iter().enumerate() {
            if i != j {
                assert_ne!(strategy1, strategy2, "Strategies should be distinct");
            }
        }
    }
}

#[test]
fn test_all_shell_types_are_distinct() {
    let shells = vec![
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Elvish,
    ];

    for (i, shell1) in shells.iter().enumerate() {
        for (j, shell2) in shells.iter().enumerate() {
            if i != j {
                assert_ne!(shell1, shell2, "Shell types should be distinct");
            }
        }
    }
}

// =============================================================================
// Argument Value Validation
// =============================================================================

#[test]
fn test_max_retries_upper_bound() {
    // Test that large values are accepted
    let args = Args::try_parse_from(["ltmatrix", "--max-retries", "999999", "task"]).unwrap();
    assert_eq!(args.max_retries, Some(999999));
}

#[test]
fn test_timeout_upper_bound() {
    // Test that large values are accepted (u64::MAX would be too large for practical use)
    let args = Args::try_parse_from(["ltmatrix", "--timeout", "86400", "task"]).unwrap();
    assert_eq!(args.timeout, Some(86400)); // 24 hours
}
