// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Comprehensive unit tests for core components
//!
//! This test module covers:
//! - CLI argument parsing edge cases
//! - Configuration parsing and validation
//! - Agent pool management
//! - Task model operations
//! - Telemetry event handling

use ltmatrix::agent::backend::{
    AgentConfig, AgentError, AgentResponse, AgentSession, ExecutionConfig, MemorySession,
};
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::cli::args::{
    Args, BlockedStrategy, Command, ExecutionModeArg, LogLevel, OutputFormat, Shell,
    CleanupArgs, CompletionsArgs, ManArgs, MemoryArgs, MemoryAction, MemoryClearArgs,
    MemoryStatusArgs, MemorySummarizeArgs, ReleaseArgs,
};
use ltmatrix::models::{ExecutionMode, Task, TaskComplexity, TaskStatus};
use ltmatrix::telemetry::event::{ErrorCategory, TelemetryEvent};
use chrono::{Duration, Utc};
use clap::Parser;
use uuid::Uuid;

// ============================================================================
// CLI Parsing Unit Tests
// ============================================================================

mod cli_parsing_tests {
    use super::*;

    #[test]
    fn test_args_default_values() {
        let args = Args::try_parse_from(["ltmatrix"]).unwrap();
        assert!(args.goal.is_none());
        assert!(!args.fast);
        assert!(!args.expert);
        assert!(!args.dry_run);
        assert!(!args.resume);
        assert!(!args.ask);
        assert!(!args.regenerate_plan);
        assert!(!args.no_color);
        assert!(!args.telemetry);
    }

    #[test]
    fn test_args_with_goal() {
        let args = Args::try_parse_from(["ltmatrix", "build a REST API"]).unwrap();
        assert_eq!(args.goal, Some("build a REST API".to_string()));
    }

    #[test]
    fn test_args_agent_option() {
        let args = Args::try_parse_from(["ltmatrix", "--agent", "claude", "goal"]).unwrap();
        assert_eq!(args.agent, Some("claude".to_string()));
    }

    #[test]
    fn test_args_mode_option() {
        let args = Args::try_parse_from(["ltmatrix", "--mode", "fast", "goal"]).unwrap();
        assert_eq!(args.mode, Some(ExecutionModeArg::Fast));
    }

    #[test]
    fn test_args_fast_mode() {
        let args = Args::try_parse_from(["ltmatrix", "--fast", "goal"]).unwrap();
        assert!(args.fast);
        assert_eq!(args.get_execution_mode(), ExecutionModeArg::Fast);
    }

    #[test]
    fn test_args_expert_mode() {
        let args = Args::try_parse_from(["ltmatrix", "--expert", "goal"]).unwrap();
        assert!(args.expert);
        assert_eq!(args.get_execution_mode(), ExecutionModeArg::Expert);
    }

    #[test]
    fn test_args_mode_precedence() {
        // explicit --mode conflicts with --fast
        let result = Args::try_parse_from(["ltmatrix", "--fast", "--mode", "expert", "goal"]);
        assert!(result.is_err()); // Should error due to conflict
    }

    #[test]
    fn test_args_fast_expert_conflict() {
        let result = Args::try_parse_from(["ltmatrix", "--fast", "--expert", "goal"]);
        assert!(result.is_err()); // Should error due to conflict
    }

    #[test]
    fn test_args_output_formats() {
        let args = Args::try_parse_from(["ltmatrix", "--output", "json", "goal"]).unwrap();
        assert_eq!(args.output, Some(OutputFormat::Json));

        let args = Args::try_parse_from(["ltmatrix", "--output", "text", "goal"]).unwrap();
        assert_eq!(args.output, Some(OutputFormat::Text));

        let args = Args::try_parse_from(["ltmatrix", "--output", "json-compact", "goal"]).unwrap();
        assert_eq!(args.output, Some(OutputFormat::JsonCompact));
    }

    #[test]
    fn test_args_log_levels() {
        for (level_str, expected) in [
            ("trace", LogLevel::Trace),
            ("debug", LogLevel::Debug),
            ("info", LogLevel::Info),
            ("warn", LogLevel::Warn),
            ("error", LogLevel::Error),
        ] {
            let args = Args::try_parse_from(["ltmatrix", "--log-level", level_str, "goal"]).unwrap();
            assert_eq!(args.log_level, Some(expected));
        }
    }

    #[test]
    fn test_args_blocked_strategies() {
        for (strategy_str, expected) in [
            ("skip", BlockedStrategy::Skip),
            ("ask", BlockedStrategy::Ask),
            ("abort", BlockedStrategy::Abort),
            ("retry", BlockedStrategy::Retry),
        ] {
            let args = Args::try_parse_from(["ltmatrix", "--on-blocked", strategy_str, "goal"]).unwrap();
            assert_eq!(args.on_blocked, Some(expected));
        }
    }

    #[test]
    fn test_args_timeout_and_retries() {
        let args = Args::try_parse_from(["ltmatrix", "--timeout", "7200", "--max-retries", "5", "goal"]).unwrap();
        assert_eq!(args.timeout, Some(7200));
        assert_eq!(args.max_retries, Some(5));
    }

    #[test]
    fn test_args_config_path() {
        let args = Args::try_parse_from(["ltmatrix", "--config", "/path/to/config.toml", "goal"]).unwrap();
        assert_eq!(args.config, Some(std::path::PathBuf::from("/path/to/config.toml")));
    }

    #[test]
    fn test_args_mcp_config() {
        let args = Args::try_parse_from(["ltmatrix", "--mcp-config", "mcp.toml", "goal"]).unwrap();
        assert_eq!(args.mcp_config, Some(std::path::PathBuf::from("mcp.toml")));
    }

    #[test]
    fn test_args_flags() {
        let args = Args::try_parse_from(["ltmatrix", "--dry-run", "--resume", "--ask", "--no-color", "--telemetry", "goal"]).unwrap();
        assert!(args.dry_run);
        assert!(args.resume);
        assert!(args.ask);
        assert!(args.no_color);
        assert!(args.telemetry);
    }

    #[test]
    fn test_execution_mode_display() {
        assert_eq!(ExecutionModeArg::Fast.to_string(), "fast");
        assert_eq!(ExecutionModeArg::Standard.to_string(), "standard");
        assert_eq!(ExecutionModeArg::Expert.to_string(), "expert");
    }

    #[test]
    fn test_execution_mode_to_model() {
        assert!(matches!(ExecutionModeArg::Fast.to_model(), ExecutionMode::Fast));
        assert!(matches!(ExecutionModeArg::Standard.to_model(), ExecutionMode::Standard));
        assert!(matches!(ExecutionModeArg::Expert.to_model(), ExecutionMode::Expert));
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Text.to_string(), "text");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::JsonCompact.to_string(), "json-compact");
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
    fn test_blocked_strategy_display() {
        assert_eq!(BlockedStrategy::Skip.to_string(), "skip");
        assert_eq!(BlockedStrategy::Ask.to_string(), "ask");
        assert_eq!(BlockedStrategy::Abort.to_string(), "abort");
        assert_eq!(BlockedStrategy::Retry.to_string(), "retry");
    }

    #[test]
    fn test_shell_display() {
        assert_eq!(Shell::Bash.to_string(), "bash");
        assert_eq!(Shell::Zsh.to_string(), "zsh");
        assert_eq!(Shell::Fish.to_string(), "fish");
        assert_eq!(Shell::PowerShell.to_string(), "powershell");
        assert_eq!(Shell::Elvish.to_string(), "elvish");
    }

    #[test]
    fn test_completions_subcommand() {
        let args = Args::try_parse_from(["ltmatrix", "completions", "bash"]).unwrap();
        match args.command {
            Some(Command::Completions(CompletionsArgs { shell, install })) => {
                assert_eq!(shell, Shell::Bash);
                assert!(!install);
            }
            _ => panic!("Expected Completions command"),
        }
    }

    #[test]
    fn test_completions_with_install() {
        let args = Args::try_parse_from(["ltmatrix", "completions", "zsh", "--install"]).unwrap();
        match args.command {
            Some(Command::Completions(CompletionsArgs { shell, install })) => {
                assert_eq!(shell, Shell::Zsh);
                assert!(install);
            }
            _ => panic!("Expected Completions command with install"),
        }
    }

    #[test]
    fn test_man_subcommand() {
        let args = Args::try_parse_from(["ltmatrix", "man"]).unwrap();
        match args.command {
            Some(Command::Man(ManArgs { output })) => {
                assert_eq!(output, std::path::PathBuf::from("./man"));
            }
            _ => panic!("Expected Man command"),
        }
    }

    #[test]
    fn test_man_subcommand_custom_output() {
        let args = Args::try_parse_from(["ltmatrix", "man", "--output", "/usr/share/man"]).unwrap();
        match args.command {
            Some(Command::Man(ManArgs { output })) => {
                assert_eq!(output, std::path::PathBuf::from("/usr/share/man"));
            }
            _ => panic!("Expected Man command with custom output"),
        }
    }

    #[test]
    fn test_cleanup_subcommand_reset_all() {
        let args = Args::try_parse_from(["ltmatrix", "cleanup", "--reset-all"]).unwrap();
        match args.command {
            Some(Command::Cleanup(CleanupArgs { reset_all, reset_failed, remove, force, dry_run })) => {
                assert!(reset_all);
                assert!(!reset_failed);
                assert!(!remove);
                assert!(!force);
                assert!(!dry_run);
            }
            _ => panic!("Expected Cleanup command"),
        }
    }

    #[test]
    fn test_cleanup_subcommand_reset_failed() {
        let args = Args::try_parse_from(["ltmatrix", "cleanup", "--reset-failed", "--force"]).unwrap();
        match args.command {
            Some(Command::Cleanup(CleanupArgs { reset_all, reset_failed, remove, force, dry_run })) => {
                assert!(!reset_all);
                assert!(reset_failed);
                assert!(!remove);
                assert!(force);
                assert!(!dry_run);
            }
            _ => panic!("Expected Cleanup command with reset_failed"),
        }
    }

    #[test]
    fn test_cleanup_subcommand_remove() {
        let args = Args::try_parse_from(["ltmatrix", "cleanup", "--remove", "--dry-run"]).unwrap();
        match args.command {
            Some(Command::Cleanup(CleanupArgs { reset_all, reset_failed, remove, force, dry_run })) => {
                assert!(!reset_all);
                assert!(!reset_failed);
                assert!(remove);
                assert!(!force);
                assert!(dry_run);
            }
            _ => panic!("Expected Cleanup command with remove"),
        }
    }

    #[test]
    fn test_cleanup_conflicts() {
        // reset-all and reset-failed conflict
        let result = Args::try_parse_from(["ltmatrix", "cleanup", "--reset-all", "--reset-failed"]);
        assert!(result.is_err());

        // reset-all and remove conflict
        let result = Args::try_parse_from(["ltmatrix", "cleanup", "--reset-all", "--remove"]);
        assert!(result.is_err());

        // reset-failed and remove conflict
        let result = Args::try_parse_from(["ltmatrix", "cleanup", "--reset-failed", "--remove"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_subcommands() {
        // Summarize
        let args = Args::try_parse_from(["ltmatrix", "memory", "summarize", "--force"]).unwrap();
        match args.command {
            Some(Command::Memory(MemoryArgs { action: MemoryAction::Summarize(args) })) => {
                assert!(args.force);
                assert!(!args.dry_run);
            }
            _ => panic!("Expected Memory Summarize command"),
        }

        // Status
        let args = Args::try_parse_from(["ltmatrix", "memory", "status", "--json"]).unwrap();
        match args.command {
            Some(Command::Memory(MemoryArgs { action: MemoryAction::Status(args) })) => {
                assert!(args.json);
            }
            _ => panic!("Expected Memory Status command"),
        }

        // Clear
        let args = Args::try_parse_from(["ltmatrix", "memory", "clear", "--force"]).unwrap();
        match args.command {
            Some(Command::Memory(MemoryArgs { action: MemoryAction::Clear(args) })) => {
                assert!(args.force);
            }
            _ => panic!("Expected Memory Clear command"),
        }
    }

    #[test]
    fn test_release_subcommand() {
        let args = Args::try_parse_from(["ltmatrix", "release", "--archive", "--all-targets"]).unwrap();
        match args.command {
            Some(Command::Release(ReleaseArgs { target, output, archive, all_targets })) => {
                assert!(target.is_none());
                assert_eq!(output, std::path::PathBuf::from("./dist"));
                assert!(archive);
                assert!(all_targets);
            }
            _ => panic!("Expected Release command"),
        }
    }

    #[test]
    fn test_release_subcommand_with_target() {
        let args = Args::try_parse_from(["ltmatrix", "release", "--target", "x86_64-unknown-linux-musl"]).unwrap();
        match args.command {
            Some(Command::Release(ReleaseArgs { target, output, archive, all_targets })) => {
                assert_eq!(target, Some("x86_64-unknown-linux-musl".to_string()));
                assert!(!archive);
                assert!(!all_targets);
            }
            _ => panic!("Expected Release command with target"),
        }
    }

    #[test]
    fn test_is_run_command() {
        let args = Args::try_parse_from(["ltmatrix", "goal"]).unwrap();
        assert!(args.is_run_command());

        let args = Args::try_parse_from(["ltmatrix", "completions", "bash"]).unwrap();
        assert!(!args.is_run_command());
    }
}

// ============================================================================
// Agent Config Unit Tests
// ============================================================================

mod agent_config_tests {
    use super::*;

    #[test]
    fn test_agent_config_default_values() {
        let config = AgentConfig::default();
        assert_eq!(config.name, "claude");
        assert_eq!(config.model, "claude-sonnet-4-6");
        assert_eq!(config.command, "claude");
        assert_eq!(config.timeout_secs, 3600);
        assert_eq!(config.max_retries, 3);
        assert!(config.enable_session);
    }

    #[test]
    fn test_agent_config_builder_all_fields() {
        let config = AgentConfig::builder()
            .name("opencode")
            .model("gpt-4-turbo")
            .command("opencode")
            .timeout_secs(7200)
            .max_retries(5)
            .enable_session(false)
            .build();

        assert_eq!(config.name, "opencode");
        assert_eq!(config.model, "gpt-4-turbo");
        assert_eq!(config.command, "opencode");
        assert_eq!(config.timeout_secs, 7200);
        assert_eq!(config.max_retries, 5);
        assert!(!config.enable_session);
    }

    #[test]
    fn test_agent_config_validate_success() {
        let config = AgentConfig::builder()
            .name("test")
            .model("test-model")
            .command("test-cmd")
            .timeout_secs(100)
            .build();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_agent_config_validate_empty_name() {
        let config = AgentConfig::builder()
            .name("")
            .model("test-model")
            .command("test-cmd")
            .timeout_secs(100)
            .build();

        let result = config.validate();
        assert!(result.is_err());
        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, "name");
        } else {
            panic!("Expected ConfigValidation error for name");
        }
    }

    #[test]
    fn test_agent_config_validate_whitespace_name() {
        let config = AgentConfig::builder()
            .name("   ")
            .model("test-model")
            .command("test-cmd")
            .timeout_secs(100)
            .build();

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_agent_config_validate_empty_model() {
        let config = AgentConfig::builder()
            .name("test")
            .model("")
            .command("test-cmd")
            .timeout_secs(100)
            .build();

        let result = config.validate();
        assert!(result.is_err());
        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, "model");
        } else {
            panic!("Expected ConfigValidation error for model");
        }
    }

    #[test]
    fn test_agent_config_validate_empty_command() {
        let config = AgentConfig::builder()
            .name("test")
            .model("test-model")
            .command("")
            .timeout_secs(100)
            .build();

        let result = config.validate();
        assert!(result.is_err());
        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, "command");
        } else {
            panic!("Expected ConfigValidation error for command");
        }
    }

    #[test]
    fn test_agent_config_validate_zero_timeout() {
        let config = AgentConfig::builder()
            .name("test")
            .model("test-model")
            .command("test-cmd")
            .timeout_secs(0)
            .build();

        let result = config.validate();
        assert!(result.is_err());
        if let Err(AgentError::ConfigValidation { field, .. }) = result {
            assert_eq!(field, "timeout_secs");
        } else {
            panic!("Expected ConfigValidation error for timeout_secs");
        }
    }

    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-6");
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout, 3600);
        assert!(config.enable_session);
        assert!(config.env_vars.is_empty());
    }

    #[test]
    fn test_execution_config_custom() {
        let config = ExecutionConfig {
            model: "claude-opus-4-6".to_string(),
            max_retries: 10,
            timeout: 14400,
            enable_session: false,
            env_vars: vec![
                ("API_KEY".to_string(), "secret".to_string()),
                ("DEBUG".to_string(), "1".to_string()),
            ],
        };

        assert_eq!(config.model, "claude-opus-4-6");
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.timeout, 14400);
        assert!(!config.enable_session);
        assert_eq!(config.env_vars.len(), 2);
    }

    #[test]
    fn test_agent_response_default() {
        let response = AgentResponse::default();
        assert!(response.output.is_empty());
        assert!(response.structured_data.is_none());
        assert!(!response.is_complete);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_agent_response_with_all_fields() {
        let response = AgentResponse {
            output: "Task completed successfully".to_string(),
            structured_data: Some(serde_json::json!({
                "files_changed": 5,
                "tests_run": 10,
                "tests_passed": 10
            })),
            is_complete: true,
            error: None,
        };

        assert_eq!(response.output, "Task completed successfully");
        assert!(response.structured_data.is_some());
        assert!(response.is_complete);
        assert!(response.error.is_none());

        let data = response.structured_data.unwrap();
        assert_eq!(data["files_changed"], 5);
        assert_eq!(data["tests_run"], 10);
    }

    #[test]
    fn test_agent_response_with_error() {
        let response = AgentResponse {
            output: String::new(),
            structured_data: None,
            is_complete: false,
            error: Some("Execution failed: timeout".to_string()),
        };

        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap(), "Execution failed: timeout");
    }
}

// ============================================================================
// Agent Error Unit Tests
// ============================================================================

mod agent_error_tests {
    use super::*;

    #[test]
    fn test_agent_error_command_not_found() {
        let error = AgentError::CommandNotFound {
            command: "claude".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("claude"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_agent_error_execution_failed() {
        let error = AgentError::ExecutionFailed {
            command: "opencode".to_string(),
            message: "Exit code 1".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("opencode"));
        assert!(msg.contains("failed"));
        assert!(msg.contains("Exit code 1"));
    }

    #[test]
    fn test_agent_error_timeout() {
        let error = AgentError::Timeout {
            command: "kimi-code".to_string(),
            timeout_secs: 3600,
        };
        let msg = error.to_string();
        assert!(msg.contains("kimi-code"));
        assert!(msg.contains("timed out"));
        assert!(msg.contains("3600"));
    }

    #[test]
    fn test_agent_error_invalid_response() {
        let error = AgentError::InvalidResponse {
            reason: "JSON parse error at line 1".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("Invalid"));
        assert!(msg.contains("JSON parse error"));
    }

    #[test]
    fn test_agent_error_config_validation() {
        let error = AgentError::ConfigValidation {
            field: "timeout_secs".to_string(),
            message: "must be greater than 0".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("timeout_secs"));
        assert!(msg.contains("must be greater than 0"));
    }

    #[test]
    fn test_agent_error_session_not_found() {
        let error = AgentError::SessionNotFound {
            session_id: "abc-123-def".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("abc-123-def"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_agent_error_is_std_error() {
        let error = AgentError::CommandNotFound {
            command: "test".to_string(),
        };
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &error;
    }
}

// ============================================================================
// Memory Session Unit Tests
// ============================================================================

mod memory_session_tests {
    use super::*;

    #[test]
    fn test_memory_session_default() {
        let session = MemorySession::default();
        assert!(!session.session_id.is_empty());
        assert_eq!(session.agent_name, "claude");
        assert_eq!(session.model, "claude-sonnet-4-6");
        assert_eq!(session.reuse_count, 0);
    }

    #[test]
    fn test_memory_session_custom() {
        let session = MemorySession {
            session_id: "custom-id".to_string(),
            agent_name: "opencode".to_string(),
            model: "gpt-4".to_string(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            reuse_count: 5,
        };

        assert_eq!(session.session_id, "custom-id");
        assert_eq!(session.agent_name, "opencode");
        assert_eq!(session.model, "gpt-4");
        assert_eq!(session.reuse_count, 5);
    }

    #[test]
    fn test_memory_session_mark_accessed() {
        let mut session = MemorySession::default();
        let initial_last_accessed = session.last_accessed;

        session.mark_accessed();
        assert_eq!(session.reuse_count, 1);
        assert!(session.last_accessed >= initial_last_accessed);

        session.mark_accessed();
        assert_eq!(session.reuse_count, 2);
    }

    #[test]
    fn test_memory_session_not_stale_initially() {
        let session = MemorySession::default();
        assert!(!session.is_stale());
    }

    #[test]
    fn test_memory_session_is_stale_after_one_hour() {
        let session = MemorySession {
            session_id: "test".to_string(),
            agent_name: "claude".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            created_at: Utc::now() - Duration::hours(2),
            last_accessed: Utc::now() - Duration::hours(2),
            reuse_count: 0,
        };
        assert!(session.is_stale());
    }

    #[test]
    fn test_memory_session_not_stale_at_boundary() {
        let session = MemorySession {
            session_id: "test".to_string(),
            agent_name: "claude".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            created_at: Utc::now() - Duration::hours(2),
            last_accessed: Utc::now() - Duration::seconds(3599),
            reuse_count: 0,
        };
        assert!(!session.is_stale());
    }

    #[test]
    fn test_memory_session_trait_implementation() {
        let mut session = MemorySession::default();

        // Test all trait methods
        let id = session.session_id();
        assert!(!id.is_empty());

        let agent = session.agent_name();
        assert_eq!(agent, "claude");

        let model = session.model();
        assert_eq!(model, "claude-sonnet-4-6");

        let _created = session.created_at();
        let _accessed = session.last_accessed();
        let _count = session.reuse_count();

        session.mark_accessed();
        assert_eq!(session.reuse_count(), 1);

        let _stale = session.is_stale();
    }
}

// ============================================================================
// Session Pool Unit Tests
// ============================================================================

mod session_pool_tests {
    use super::*;

    #[test]
    fn test_pool_new_is_empty() {
        let pool = SessionPool::new();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_pool_default_is_empty() {
        let pool = SessionPool::default();
        assert!(pool.is_empty());
    }

    #[test]
    fn test_pool_register_and_get() {
        let mut pool = SessionPool::new();
        let session = MemorySession::default();
        let id = session.session_id.clone();

        pool.register(session);

        assert!(!pool.is_empty());
        assert_eq!(pool.len(), 1);

        let fetched = pool.get(&id);
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().session_id, id);
    }

    #[test]
    fn test_pool_register_replaces_existing() {
        let mut pool = SessionPool::new();

        let session1 = MemorySession {
            session_id: "same-id".to_string(),
            reuse_count: 1,
            ..Default::default()
        };
        pool.register(session1);

        let session2 = MemorySession {
            session_id: "same-id".to_string(),
            reuse_count: 5,
            ..Default::default()
        };
        pool.register(session2);

        assert_eq!(pool.len(), 1);
        let fetched = pool.get("same-id").unwrap();
        assert_eq!(fetched.reuse_count, 5);
    }

    #[test]
    fn test_pool_remove() {
        let mut pool = SessionPool::new();
        let session = MemorySession::default();
        let id = session.session_id.clone();

        pool.register(session);
        assert_eq!(pool.len(), 1);

        let removed = pool.remove(&id);
        assert!(removed.is_some());
        assert!(pool.is_empty());

        let removed_again = pool.remove(&id);
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_pool_get_nonexistent() {
        let pool = SessionPool::new();
        assert!(pool.get("nonexistent-id").is_none());
    }

    #[test]
    fn test_pool_cleanup_stale() {
        let mut pool = SessionPool::new();

        // Fresh session
        pool.register(MemorySession::default());

        // Stale session
        let mut stale = MemorySession::default();
        stale.last_accessed = Utc::now() - Duration::hours(2);
        pool.register(stale);

        assert_eq!(pool.len(), 2);

        let cleaned = pool.cleanup_stale();
        assert_eq!(cleaned, 1);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_pool_cleanup_no_stale() {
        let mut pool = SessionPool::new();
        pool.register(MemorySession::default());
        pool.register(MemorySession::default());

        let cleaned = pool.cleanup_stale();
        assert_eq!(cleaned, 0);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_pool_get_or_create_creates_new() {
        let mut pool = SessionPool::new();

        let session = pool.get_or_create("claude", "claude-sonnet-4-6");
        let agent_name = session.agent_name.clone();
        let model = session.model.clone();
        drop(session); // Release borrow

        assert_eq!(pool.len(), 1);
        assert_eq!(agent_name, "claude");
        assert_eq!(model, "claude-sonnet-4-6");
    }

    #[test]
    fn test_pool_get_or_create_reuses_existing() {
        let mut pool = SessionPool::new();

        let session1 = pool.get_or_create("claude", "claude-sonnet-4-6");
        let id1 = session1.session_id.clone();
        drop(session1); // Release borrow

        let session2 = pool.get_or_create("claude", "claude-sonnet-4-6");
        let id2 = session2.session_id.clone();
        drop(session2); // Release borrow

        assert_eq!(id1, id2);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_pool_get_or_create_different_agents() {
        let mut pool = SessionPool::new();

        let session1 = pool.get_or_create("claude", "claude-sonnet-4-6");
        let id1 = session1.session_id.clone();
        drop(session1); // Release borrow

        // Get second session with different agent
        let session2 = pool.get_or_create("opencode", "gpt-4");
        let id2 = session2.session_id.clone();
        drop(session2); // Release borrow

        assert_ne!(id1, id2);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_pool_get_or_create_does_not_reuse_stale() {
        let mut pool = SessionPool::new();

        // Create first session
        let session1 = pool.get_or_create("claude", "claude-sonnet-4-6");
        let id1 = session1.session_id.clone();
        drop(session1); // Release borrow

        // Make it stale by registering a stale session with same agent/model
        let stale = MemorySession {
            session_id: id1.clone(),
            agent_name: "claude".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            last_accessed: Utc::now() - Duration::hours(2),
            ..Default::default()
        };
        pool.register(stale);

        // Get or create should create new session since the existing one is stale
        let session2 = pool.get_or_create("claude", "claude-sonnet-4-6");
        let id2 = session2.session_id.clone();
        drop(session2); // Release borrow

        assert_ne!(id2, id1);
        // Pool should have 2 sessions now (stale + new)
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_pool_list_by_agent() {
        let mut pool = SessionPool::new();

        pool.register(MemorySession {
            agent_name: "claude".to_string(),
            ..Default::default()
        });
        pool.register(MemorySession {
            agent_name: "claude".to_string(),
            ..Default::default()
        });
        pool.register(MemorySession {
            agent_name: "opencode".to_string(),
            ..Default::default()
        });

        let claude_sessions = pool.list_by_agent("claude");
        assert_eq!(claude_sessions.len(), 2);

        let opencode_sessions = pool.list_by_agent("opencode");
        assert_eq!(opencode_sessions.len(), 1);

        let unknown_sessions = pool.list_by_agent("unknown");
        assert!(unknown_sessions.is_empty());
    }

    #[test]
    fn test_pool_mark_accessed() {
        let mut pool = SessionPool::new();
        let session = MemorySession::default();
        let id = session.session_id.clone();
        pool.register(session);

        let initial_count = pool.get(&id).unwrap().reuse_count;

        let result = pool.mark_accessed(&id);
        assert!(result);
        assert_eq!(pool.get(&id).unwrap().reuse_count, initial_count + 1);

        let result = pool.mark_accessed("nonexistent");
        assert!(!result);
    }

    #[test]
    fn test_pool_iter() {
        let mut pool = SessionPool::new();

        pool.register(MemorySession {
            agent_name: "claude".to_string(),
            ..Default::default()
        });
        pool.register(MemorySession {
            agent_name: "opencode".to_string(),
            ..Default::default()
        });

        let agents: Vec<_> = pool.iter().map(|s| s.agent_name.as_str()).collect();
        assert_eq!(agents.len(), 2);
        assert!(agents.contains(&"claude"));
        assert!(agents.contains(&"opencode"));
    }

    #[test]
    fn test_pool_get_for_retry() {
        let mut pool = SessionPool::new();
        let session = pool.get_or_create("claude", "claude-sonnet-4-6");
        let id = session.session_id.clone();
        let initial_count = session.reuse_count;

        let result = pool.get_for_retry(&id);
        assert!(result.is_some());

        let retry_session = result.unwrap();
        assert_eq!(retry_session.session_id, id);
        assert_eq!(retry_session.reuse_count, initial_count + 1);
    }

    #[test]
    fn test_pool_get_for_retry_stale_returns_none() {
        let mut pool = SessionPool::new();

        // Create a stale session
        let mut stale = MemorySession {
            agent_name: "claude".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            last_accessed: Utc::now() - Duration::hours(2),
            ..Default::default()
        };
        let id = stale.session_id.clone();
        pool.register(stale);

        let result = pool.get_for_retry(&id);
        assert!(result.is_none());
    }

    #[test]
    fn test_pool_get_for_retry_nonexistent_returns_none() {
        let mut pool = SessionPool::new();
        let result = pool.get_for_retry("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_pool_has_warmup() {
        let pool = SessionPool::new();
        assert!(!pool.has_warmup());
    }

    #[test]
    fn test_pool_is_warmed_up() {
        let pool = SessionPool::new();
        assert!(!pool.is_warmed_up("claude", "claude-sonnet-4-6"));
    }
}

// ============================================================================
// Task Model Unit Tests
// ============================================================================

mod task_model_tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let task = Task::new("task-1", "Implement feature", "Description of task");

        assert_eq!(task.id, "task-1");
        assert_eq!(task.title, "Implement feature");
        assert_eq!(task.description, "Description of task");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.complexity, TaskComplexity::Moderate);
        assert_eq!(task.retry_count, 0);
    }

    #[test]
    fn test_task_status_is_terminal() {
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::InProgress.is_terminal());
        assert!(!TaskStatus::Blocked.is_terminal());
    }

    #[test]
    fn test_task_complexity_variants() {
        // Test all complexity variants exist
        let _simple = TaskComplexity::Simple;
        let _moderate = TaskComplexity::Moderate;
        let _complex = TaskComplexity::Complex;
    }

    #[test]
    fn test_task_has_session() {
        let mut task = Task::new("task-1", "Test", "Description");
        assert!(!task.has_session());

        task.set_session_id("session-123");
        assert!(task.has_session());

        task.clear_session_id();
        assert!(!task.has_session());
    }

    #[test]
    fn test_task_prepare_retry() {
        let mut task = Task::new("task-1", "Test", "Description");
        task.set_session_id("session-123");
        task.status = TaskStatus::Failed;

        task.prepare_retry();

        assert_eq!(task.retry_count, 1);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.has_session()); // Session preserved
    }

    #[test]
    fn test_task_session_operations() {
        let mut task = Task::new("task-1", "Test", "Description");

        // Set session ID
        task.set_session_id("test-session-id");
        assert_eq!(task.get_session_id(), Some("test-session-id"));

        // Clear session ID
        task.clear_session_id();
        assert!(task.get_session_id().is_none());

        // Set parent session ID
        task.set_parent_session_id("parent-session-id");
        assert!(task.get_parent_session_id().is_some());

        // Clear parent session ID
        task.clear_parent_session_id();
        assert!(task.get_parent_session_id().is_none());
    }

    #[test]
    fn test_task_depends_on() {
        let mut task = Task::new("task-1", "Test", "Description");

        task.depends_on.push("task-0".to_string());
        task.depends_on.push("task-0b".to_string());

        assert_eq!(task.depends_on.len(), 2);
        assert!(task.depends_on.contains(&"task-0".to_string()));
    }

    #[test]
    fn test_task_serialization() {
        let task = Task::new("task-1", "Test Task", "Description");

        let json = serde_json::to_string(&task).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(task.id, parsed.id);
        assert_eq!(task.title, parsed.title);
        assert_eq!(task.description, parsed.description);
    }
}

// ============================================================================
// Telemetry Event Unit Tests
// ============================================================================

mod telemetry_event_tests {
    use super::*;

    #[test]
    fn test_telemetry_event_session_start() {
        let session_id = Uuid::new_v4();
        let event = TelemetryEvent::SessionStart {
            session_id,
            version: "0.1.0".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            timestamp: Utc::now(),
        };

        assert_eq!(event.session_id(), &session_id);
    }

    #[test]
    fn test_telemetry_event_pipeline_complete() {
        let session_id = Uuid::new_v4();
        let event = TelemetryEvent::PipelineComplete {
            session_id,
            execution_mode: ExecutionMode::Standard,
            agent_backend: "claude".to_string(),
            total_tasks: 10,
            completed_tasks: 8,
            failed_tasks: 2,
            duration_secs: 3600,
            timestamp: Utc::now(),
        };

        assert_eq!(event.session_id(), &session_id);
    }

    #[test]
    fn test_telemetry_event_error() {
        let session_id = Uuid::new_v4();
        let event = TelemetryEvent::Error {
            session_id,
            error_category: ErrorCategory::AgentTimeout,
            timestamp: Utc::now(),
        };

        assert_eq!(event.session_id(), &session_id);
    }

    #[test]
    fn test_telemetry_event_serialization() {
        let session_id = Uuid::new_v4();
        let event = TelemetryEvent::SessionStart {
            session_id,
            version: "0.1.0".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("SessionStart") || json.contains("session_start"));
        assert!(json.contains("linux"));
        assert!(json.contains("x86_64"));
    }

    #[test]
    fn test_telemetry_event_roundtrip() {
        let session_id = Uuid::new_v4();
        let event = TelemetryEvent::PipelineComplete {
            session_id,
            execution_mode: ExecutionMode::Expert,
            agent_backend: "claude".to_string(),
            total_tasks: 5,
            completed_tasks: 5,
            failed_tasks: 0,
            duration_secs: 1800,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: TelemetryEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.session_id(), parsed.session_id());
    }

    #[test]
    fn test_error_category_all_variants() {
        let categories = [
            ErrorCategory::AgentTimeout,
            ErrorCategory::AgentExecutionFailed,
            ErrorCategory::TestFailure,
            ErrorCategory::VerificationFailed,
            ErrorCategory::GitOperationFailed,
            ErrorCategory::ConfigurationError,
            ErrorCategory::DependencyValidationFailed,
            ErrorCategory::PipelineExecutionFailed,
            ErrorCategory::Other,
        ];

        for category in categories {
            let json = serde_json::to_string(&category).unwrap();
            let parsed: ErrorCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(category, parsed);
        }
    }

    #[test]
    fn test_error_category_from_message() {
        // Test all category detection patterns
        assert_eq!(
            ErrorCategory::from_error_message("timeout occurred"),
            ErrorCategory::AgentTimeout
        );
        assert_eq!(
            ErrorCategory::from_error_message("TIMED OUT"),
            ErrorCategory::AgentTimeout
        );
        assert_eq!(
            ErrorCategory::from_error_message("test failed"),
            ErrorCategory::TestFailure
        );
        assert_eq!(
            ErrorCategory::from_error_message("verification failed"),
            ErrorCategory::VerificationFailed
        );
        assert_eq!(
            ErrorCategory::from_error_message("git commit failed"),
            ErrorCategory::GitOperationFailed
        );
        assert_eq!(
            ErrorCategory::from_error_message("configuration error"),
            ErrorCategory::ConfigurationError
        );
        assert_eq!(
            ErrorCategory::from_error_message("dependency not found"),
            ErrorCategory::DependencyValidationFailed
        );
        assert_eq!(
            ErrorCategory::from_error_message("pipeline stage failed"),
            ErrorCategory::PipelineExecutionFailed
        );
        assert_eq!(
            ErrorCategory::from_error_message("agent returned error"),
            ErrorCategory::AgentExecutionFailed
        );
        assert_eq!(
            ErrorCategory::from_error_message("unknown error"),
            ErrorCategory::Other
        );
        assert_eq!(
            ErrorCategory::from_error_message(""),
            ErrorCategory::Other
        );
    }
}

// ============================================================================
// Execution Mode Unit Tests
// ============================================================================

mod execution_mode_tests {
    use super::*;

    #[test]
    fn test_execution_mode_variants() {
        assert_eq!(ExecutionMode::Fast.to_string(), "fast");
        assert_eq!(ExecutionMode::Standard.to_string(), "standard");
        assert_eq!(ExecutionMode::Expert.to_string(), "expert");
    }

    #[test]
    fn test_execution_mode_serialization() {
        for mode in [ExecutionMode::Fast, ExecutionMode::Standard, ExecutionMode::Expert] {
            let json = serde_json::to_string(&mode).unwrap();
            let parsed: ExecutionMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, parsed);
        }
    }

    #[test]
    fn test_execution_mode_clone() {
        let mode = ExecutionMode::Expert;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);
    }
}

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_agent_config_with_unicode() {
        let config = AgentConfig::builder()
            .name("claude-中文-日本語")
            .model("claude-sonnet-4-6")
            .command("claude")
            .timeout_secs(3600)
            .build();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_agent_config_with_special_characters() {
        let config = AgentConfig::builder()
            .name("agent-with-dashes_and_underscores")
            .model("model@v2.0")
            .command("./bin/agent")
            .timeout_secs(3600)
            .build();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_large_agent_response() {
        let large_output = "x".repeat(1_000_000); // 1MB
        let response = AgentResponse {
            output: large_output.clone(),
            structured_data: None,
            is_complete: true,
            error: None,
        };

        assert_eq!(response.output.len(), 1_000_000);
    }

    #[test]
    fn test_complex_structured_data() {
        let structured_data = serde_json::json!({
            "files": [
                {"path": "/src/main.rs", "changes": 50},
                {"path": "/src/lib.rs", "changes": 30}
            ],
            "tests": {
                "total": 100,
                "passed": 98,
                "failed": 2
            },
            "metrics": {
                "coverage": 85.5,
                "complexity": "low"
            }
        });

        let response = AgentResponse {
            output: "Task completed".to_string(),
            structured_data: Some(structured_data.clone()),
            is_complete: true,
            error: None,
        };

        assert!(response.structured_data.is_some());
        let data = response.structured_data.unwrap();
        assert_eq!(data["files"][0]["path"], "/src/main.rs");
        assert_eq!(data["tests"]["total"], 100);
        assert_eq!(data["metrics"]["coverage"], 85.5);
    }

    #[test]
    fn test_pool_with_many_sessions() {
        let mut pool = SessionPool::new();

        // Create 100 sessions
        for i in 0..100 {
            let session = MemorySession {
                session_id: format!("session-{}", i),
                agent_name: format!("agent-{}", i % 5),
                model: format!("model-{}", i % 3),
                ..Default::default()
            };
            pool.register(session);
        }

        assert_eq!(pool.len(), 100);

        // List by agent
        for agent_idx in 0..5 {
            let sessions = pool.list_by_agent(&format!("agent-{}", agent_idx));
            assert_eq!(sessions.len(), 20);
        }
    }

    #[test]
    fn test_pool_rapid_create_operations() {
        let mut pool = SessionPool::new();
        let mut ids = Vec::new();

        // Rapid create same agent (should reuse)
        for _ in 0..50 {
            let session = pool.get_or_create("claude", "claude-sonnet-4-6");
            ids.push(session.session_id.clone());
        }

        // All should reuse the same session - check by counting unique IDs
        let unique_ids: std::collections::HashSet<_> = ids.iter().cloned().collect();
        assert_eq!(unique_ids.len(), 1);

        // Create different agent sessions
        for i in 0..50 {
            pool.get_or_create(&format!("agent-{}", i), "model");
        }

        // Verify total count
        let all_sessions: Vec<_> = pool.iter().collect();
        assert_eq!(all_sessions.len(), 51);
    }

    #[test]
    fn test_cli_args_with_special_goal() {
        // Test goal with special characters
        let args = Args::try_parse_from(["ltmatrix", "build a \"REST API\" with tests"]).unwrap();
        assert_eq!(args.goal, Some("build a \"REST API\" with tests".to_string()));
    }

    #[test]
    fn test_cli_args_multiple_flags() {
        let args = Args::try_parse_from([
            "ltmatrix",
            "--fast",
            "--dry-run",
            "--no-color",
            "--telemetry",
            "--timeout", "60",
            "--max-retries", "1",
            "goal",
        ]).unwrap();

        assert!(args.fast);
        assert!(args.dry_run);
        assert!(args.no_color);
        assert!(args.telemetry);
        assert_eq!(args.timeout, Some(60));
        assert_eq!(args.max_retries, Some(1));
    }

    #[test]
    fn test_task_with_subtasks() {
        let mut task = Task::new("task-1", "Parent Task", "Description");

        let subtask1 = Task::new("task-1-1", "Subtask 1", "Description");
        let subtask2 = Task::new("task-1-2", "Subtask 2", "Description");

        task.subtasks.push(subtask1);
        task.subtasks.push(subtask2);

        assert_eq!(task.subtasks.len(), 2);
        assert_eq!(task.subtasks[0].id, "task-1-1");
        assert_eq!(task.subtasks[1].id, "task-1-2");
    }
}
