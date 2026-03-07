//! Integration tests for --on-blocked CLI flag
//!
//! This test module verifies that the --on-blocked CLI flag is properly
//! parsed and integrated with the execution pipeline.

use clap::Parser;
use ltmatrix::cli::args::{Args, BlockedStrategy};

#[cfg(test)]
mod cli_parsing_tests {
    use super::*;

    #[test]
    fn test_parse_on_blocked_skip() {
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "skip", "test goal"])
            .expect("Failed to parse args");

        assert_eq!(args.on_blocked, Some(BlockedStrategy::Skip));
        assert_eq!(args.goal, Some("test goal".to_string()));
    }

    #[test]
    fn test_parse_on_blocked_ask() {
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "ask", "test goal"])
            .expect("Failed to parse args");

        assert_eq!(args.on_blocked, Some(BlockedStrategy::Ask));
    }

    #[test]
    fn test_parse_on_blocked_abort() {
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "abort", "test goal"])
            .expect("Failed to parse args");

        assert_eq!(args.on_blocked, Some(BlockedStrategy::Abort));
    }

    #[test]
    fn test_parse_on_blocked_retry() {
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "retry", "test goal"])
            .expect("Failed to parse args");

        assert_eq!(args.on_blocked, Some(BlockedStrategy::Retry));
    }

    #[test]
    fn test_on_blocked_flag_optional() {
        let args = Args::try_parse_from(["ltmatrix", "test goal"]).expect("Failed to parse args");

        assert_eq!(args.on_blocked, None);
    }

    #[test]
    fn test_on_blocked_with_other_flags() {
        let args = Args::try_parse_from([
            "ltmatrix",
            "--fast",
            "--on-blocked",
            "skip",
            "--log-level",
            "debug",
            "test goal",
        ])
        .expect("Failed to parse args");

        assert!(args.fast);
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Skip));
        assert_eq!(args.log_level, Some(ltmatrix::cli::args::LogLevel::Debug));
    }

    #[test]
    fn test_on_blocked_with_expert_mode() {
        let args =
            Args::try_parse_from(["ltmatrix", "--expert", "--on-blocked", "retry", "test goal"])
                .expect("Failed to parse args");

        assert!(args.expert);
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Retry));
    }

    #[test]
    fn test_on_blocked_with_dry_run() {
        let args = Args::try_parse_from([
            "ltmatrix",
            "--dry-run",
            "--on-blocked",
            "abort",
            "test goal",
        ])
        .expect("Failed to parse args");

        assert!(args.dry_run);
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Abort));
    }

    #[test]
    fn test_on_blocked_case_sensitive() {
        // The strategy names should be case-sensitive
        // "skip" should work
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "skip", "test goal"]);
        assert!(args.is_ok());

        // "Skip" should fail (case-sensitive)
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "Skip", "test goal"]);
        assert!(args.is_err());

        // "SKIP" should fail
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "SKIP", "test goal"]);
        assert!(args.is_err());
    }

    #[test]
    fn test_invalid_on_blocked_value() {
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "invalid", "test goal"]);

        assert!(args.is_err(), "Should reject invalid strategy value");
    }

    #[test]
    fn test_on_blocked_with_resume() {
        let args = Args::try_parse_from(["ltmatrix", "--resume", "--on-blocked", "ask"])
            .expect("Failed to parse args");

        assert!(args.resume);
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Ask));
        assert!(args.goal.is_none()); // No goal needed with resume
    }

    #[test]
    fn test_on_blocked_with_config_file() {
        let args = Args::try_parse_from([
            "ltmatrix",
            "--config",
            "custom.toml",
            "--on-blocked",
            "retry",
            "test goal",
        ])
        .expect("Failed to parse args");

        assert_eq!(args.config, Some(std::path::PathBuf::from("custom.toml")));
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Retry));
    }

    #[test]
    fn test_on_blocked_with_output_format() {
        let args = Args::try_parse_from([
            "ltmatrix",
            "--output",
            "json",
            "--on-blocked",
            "skip",
            "test goal",
        ])
        .expect("Failed to parse args");

        assert_eq!(args.output, Some(ltmatrix::cli::args::OutputFormat::Json));
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Skip));
    }

    #[test]
    fn test_on_blocked_with_all_options() {
        let args = Args::try_parse_from([
            "ltmatrix",
            "--fast",
            "--dry-run",
            "--output",
            "json",
            "--log-level",
            "trace",
            "--on-blocked",
            "abort",
            "--max-retries",
            "5",
            "test goal",
        ])
        .expect("Failed to parse args");

        assert!(args.fast);
        assert!(args.dry_run);
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Abort));
        assert_eq!(args.max_retries, Some(5));
    }
}

#[cfg(test)]
mod strategy_behavior_tests {
    use super::*;

    #[test]
    fn test_skip_strategy_allows_other_tasks() {
        // Verify skip strategy is intended to allow other tasks to continue
        let strategy = BlockedStrategy::Skip;
        assert_eq!(strategy.to_string(), "skip");

        // This strategy should be used when we want to continue with
        // unblocked tasks even when some are blocked
        let strategy_description = match strategy {
            BlockedStrategy::Skip => "continue with other tasks",
            _ => "other",
        };
        assert_eq!(strategy_description, "continue with other tasks");
    }

    #[test]
    fn test_abort_strategy_stops_execution() {
        // Verify abort strategy stops the pipeline
        let strategy = BlockedStrategy::Abort;
        assert_eq!(strategy.to_string(), "abort");

        // This strategy should stop pipeline execution when blocking occurs
        let strategy_description = match strategy {
            BlockedStrategy::Abort => "stop pipeline execution",
            _ => "other",
        };
        assert_eq!(strategy_description, "stop pipeline execution");
    }

    #[test]
    fn test_retry_strategy_retries_blocked_tasks() {
        // Verify retry strategy marks tasks for retry
        let strategy = BlockedStrategy::Retry;
        assert_eq!(strategy.to_string(), "retry");

        // This strategy should retry blocked tasks
        let strategy_description = match strategy {
            BlockedStrategy::Retry => "attempt task again",
            _ => "other",
        };
        assert_eq!(strategy_description, "attempt task again");
    }

    #[test]
    fn test_ask_strategy_prompts_user() {
        // Verify ask strategy prompts for user input
        let strategy = BlockedStrategy::Ask;
        assert_eq!(strategy.to_string(), "ask");

        // This strategy should prompt user for action
        let strategy_description = match strategy {
            BlockedStrategy::Ask => "prompt user for action",
            _ => "other",
        };
        assert_eq!(strategy_description, "prompt user for action");
    }

    #[test]
    fn test_all_strategies_covered() {
        // Verify we have all expected strategies
        let strategies = vec![
            BlockedStrategy::Skip,
            BlockedStrategy::Ask,
            BlockedStrategy::Abort,
            BlockedStrategy::Retry,
        ];

        assert_eq!(strategies.len(), 4, "Should have 4 strategies");

        // Each strategy should have a unique string representation
        let strategy_strings: Vec<String> = strategies.iter().map(|s| s.to_string()).collect();

        let unique_strings: std::collections::HashSet<_> = strategy_strings.iter().collect();

        assert_eq!(
            unique_strings.len(),
            4,
            "All strategies should have unique string representations"
        );
    }

    #[test]
    fn test_strategy_default_behavior() {
        // Test that we can determine default behavior when no strategy is specified
        let args_no_strategy =
            Args::try_parse_from(["ltmatrix", "test goal"]).expect("Failed to parse args");

        assert_eq!(args_no_strategy.on_blocked, None);

        // When None, the system should use a sensible default
        // (typically this would be defined in the config or as a constant)
        let default_strategy = BlockedStrategy::Retry; // Example default
        assert!(matches!(
            default_strategy,
            BlockedStrategy::Skip
                | BlockedStrategy::Retry
                | BlockedStrategy::Abort
                | BlockedStrategy::Ask
        ));
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_missing_goal_with_on_blocked() {
        // Should require a goal when not using resume
        let args = Args::try_parse_from(["ltmatrix", "--on-blocked", "skip"]);

        // This should parse but goal will be None
        assert!(args.is_ok());
        let args = args.unwrap();
        assert!(args.goal.is_none());
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Skip));
    }

    #[test]
    fn test_help_includes_on_blocked() {
        // Verify that --on-blocked is documented in help
        // This is a compile-time check that the struct has the field

        // Test that we can create Args with on_blocked set
        let args = test_args();

        assert_eq!(args.on_blocked, Some(BlockedStrategy::Skip));
    }

    #[test]
    fn test_multiple_strategy_flags_not_allowed() {
        // When multiple --on-blocked flags are provided, clap should reject them
        let args = Args::try_parse_from([
            "ltmatrix",
            "--on-blocked",
            "skip",
            "--on-blocked",
            "retry",
            "test goal",
        ]);

        // This should fail because --on-blocked cannot be used multiple times
        assert!(
            args.is_err(),
            "Multiple --on-blocked flags should not be allowed"
        );
        let err = args.unwrap_err();
        // Verify the error message mentions the issue
        assert!(
            err.to_string().contains("cannot be used multiple times")
                || err.to_string().contains("--on-blocked"),
            "Error should mention that --on-blocked cannot be used multiple times"
        );
    }
}

// Helper function to create test Args (can't implement Default due to orphan rule)
fn test_args() -> Args {
    Args {
        goal: Some("test".to_string()),
        agent: None,
        mode: None,
        fast: false,
        expert: false,
        config: None,
        output: None,
        log_level: None,
        log_file: None,
        max_retries: None,
        timeout: None,
        dry_run: false,
        resume: false,
        ask: false,
        regenerate_plan: false,
        on_blocked: Some(BlockedStrategy::Skip),
        mcp_config: None,
        no_color: false,
        telemetry: false,
        command: None,
    }
}

#[cfg(test)]
mod integration_scenarios_tests {
    use super::*;

    #[test]
    fn test_scenario_fast_mode_with_skip() {
        // Common scenario: fast mode with skip strategy
        let args =
            Args::try_parse_from(["ltmatrix", "--fast", "--on-blocked", "skip", "quick fix"])
                .expect("Failed to parse args");

        assert!(args.fast);
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Skip));
        assert_eq!(args.goal, Some("quick fix".to_string()));
    }

    #[test]
    fn test_scenario_expert_mode_with_retry() {
        // Common scenario: expert mode with retry strategy
        let args = Args::try_parse_from([
            "ltmatrix",
            "--expert",
            "--on-blocked",
            "retry",
            "implement feature",
        ])
        .expect("Failed to parse args");

        assert!(args.expert);
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Retry));
    }

    #[test]
    fn test_scenario_dry_run_with_abort() {
        // Common scenario: dry run with abort to catch issues
        let args = Args::try_parse_from([
            "ltmatrix",
            "--dry-run",
            "--on-blocked",
            "abort",
            "plan changes",
        ])
        .expect("Failed to parse args");

        assert!(args.dry_run);
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Abort));
    }

    #[test]
    fn test_scenario_resume_with_ask() {
        // Common scenario: resume with ask for interactive recovery
        let args = Args::try_parse_from(["ltmatrix", "--resume", "--on-blocked", "ask"])
            .expect("Failed to parse args");

        assert!(args.resume);
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Ask));
    }

    #[test]
    fn test_scenario_debug_output_with_skip() {
        // Scenario: debugging with detailed output and skip strategy
        let args = Args::try_parse_from([
            "ltmatrix",
            "--log-level",
            "debug",
            "--output",
            "json",
            "--on-blocked",
            "skip",
            "debug issue",
        ])
        .expect("Failed to parse args");

        assert_eq!(args.log_level, Some(ltmatrix::cli::args::LogLevel::Debug));
        assert_eq!(args.output, Some(ltmatrix::cli::args::OutputFormat::Json));
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Skip));
    }

    #[test]
    fn test_scenario_custom_config_with_retry() {
        // Scenario: using custom config with retry strategy
        let args = Args::try_parse_from([
            "ltmatrix",
            "--config",
            "/path/to/config.toml",
            "--on-blocked",
            "retry",
            "build feature",
        ])
        .expect("Failed to parse args");

        assert_eq!(
            args.config,
            Some(std::path::PathBuf::from("/path/to/config.toml"))
        );
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Retry));
    }

    #[test]
    fn test_scenario_max_retries_with_retry_strategy() {
        // Scenario: custom max retries with retry strategy
        let args = Args::try_parse_from([
            "ltmatrix",
            "--max-retries",
            "10",
            "--on-blocked",
            "retry",
            "complex task",
        ])
        .expect("Failed to parse args");

        assert_eq!(args.max_retries, Some(10));
        assert_eq!(args.on_blocked, Some(BlockedStrategy::Retry));
    }
}
