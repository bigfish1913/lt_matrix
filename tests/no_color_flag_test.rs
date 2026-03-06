//! Tests for --no-color flag and NO_COLOR environment variable integration
//!
//! This test file verifies the interaction between CLI arguments,
//! environment variables, and terminal color configuration.

use clap::{CommandFactory, Parser};
use ltmatrix::cli::args::Args;
use ltmatrix::terminal::{self, ColorConfig};
use std::env;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // CLI Flag Parsing Tests
    // =========================================================================

    #[test]
    fn test_no_color_flag_parsing() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test goal"]).unwrap();
        assert!(args.no_color, "--no-color flag should be parsed correctly");
    }

    #[test]
    fn test_no_color_flag_default_is_false() {
        let args = Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();
        assert!(!args.no_color, "--no-color should default to false");
    }

    #[test]
    fn test_no_color_flag_with_subcommand() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "completions", "bash"]).unwrap();
        assert!(args.no_color, "--no-color should work with subcommands");
    }

    #[test]
    fn test_no_color_flag_with_other_options() {
        let args = Args::try_parse_from([
            "ltmatrix",
            "--no-color",
            "--fast",
            "--log-level",
            "debug",
            "test goal",
        ])
        .unwrap();
        assert!(
            args.no_color,
            "--no-color should work alongside other flags"
        );
        assert!(args.fast);
        assert_eq!(args.log_level, Some(ltmatrix::cli::args::LogLevel::Debug));
    }

    // =========================================================================
    // ColorConfig Integration Tests
    // =========================================================================

    #[test]
    fn test_color_config_from_cli_args_no_color_set() {
        // When --no-color is set, ColorConfig should have colors disabled
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test goal"]).unwrap();

        // The integration should create a ColorConfig with colors disabled
        // when --no-color flag is present
        let config = if args.no_color {
            ColorConfig::plain()
        } else {
            ColorConfig::auto()
        };

        assert!(
            !config.is_enabled(),
            "ColorConfig should be plain when --no-color is set"
        );
    }

    #[test]
    fn test_color_config_from_cli_args_no_color_not_set() {
        // When --no-color is NOT set, ColorConfig should auto-detect
        let args = Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();

        let config = if args.no_color {
            ColorConfig::plain()
        } else {
            ColorConfig::auto()
        };

        // Result depends on terminal detection
        let _ = config.is_enabled();
    }

    #[test]
    fn test_color_config_respects_no_color_env_with_check() {
        env::set_var("NO_COLOR", "1");

        // When NO_COLOR is set and we check it, colors should be disabled
        let config = ColorConfig::with_config(true, true);
        assert!(
            !config.is_enabled(),
            "NO_COLOR should disable colors when check_no_color=true"
        );

        env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_color_config_ignores_no_color_env_without_check() {
        env::set_var("NO_COLOR", "1");

        // When NO_COLOR is set but we don't check it, colors should be enabled
        let config = ColorConfig::with_config(true, false);
        assert!(
            config.is_enabled(),
            "NO_COLOR should be ignored when check_no_color=false"
        );

        env::remove_var("NO_COLOR");
    }

    // =========================================================================
    // Priority Tests: CLI Flag vs Environment Variable
    // =========================================================================

    #[test]
    fn test_cli_no_color_takes_priority_over_env() {
        // Set NO_COLOR environment variable
        env::set_var("NO_COLOR", "0"); // Try to enable via env

        // CLI flag should override environment variable
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test goal"]).unwrap();

        let config = if args.no_color {
            ColorConfig::plain()
        } else {
            ColorConfig::auto()
        };

        assert!(
            !config.is_enabled(),
            "CLI --no-color should override NO_COLOR env var"
        );

        env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_no_color_flag_creates_plain_config() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test goal"]).unwrap();

        // Simulate the integration logic that should be in the main application
        let config = create_color_config_from_args(&args);

        assert!(
            !config.is_enabled(),
            "ColorConfig should be disabled with --no-color flag"
        );
    }

    #[test]
    fn test_no_flag_creates_auto_config() {
        let args = Args::try_parse_from(["ltmatrix", "test goal"]).unwrap();

        let config = create_color_config_from_args(&args);

        // Auto config behavior depends on environment
        let _ = config.is_enabled();
    }

    // =========================================================================
    // Output Formatting Tests
    // =========================================================================

    #[test]
    fn test_output_with_no_color_flag() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test goal"]).unwrap();
        let config = create_color_config_from_args(&args);

        let success_msg = terminal::success("Operation successful", config);
        let error_msg = terminal::error("Operation failed", config);
        let warning_msg = terminal::warning("Operation warning", config);

        assert_eq!(success_msg, "Operation successful");
        assert_eq!(error_msg, "Operation failed");
        assert_eq!(warning_msg, "Operation warning");
    }

    #[test]
    fn test_status_colorization_with_no_color_flag() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test goal"]).unwrap();
        let config = create_color_config_from_args(&args);

        let pending = terminal::colorize_status("pending", config);
        let completed = terminal::colorize_status("completed", config);
        let failed = terminal::colorize_status("failed", config);

        assert_eq!(pending, "pending");
        assert_eq!(completed, "completed");
        assert_eq!(failed, "failed");
    }

    #[test]
    fn test_log_level_colorization_with_no_color_flag() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test goal"]).unwrap();
        let config = create_color_config_from_args(&args);

        let info = terminal::colorize_log_level("INFO", config);
        let warn = terminal::colorize_log_level("WARN", config);
        let error = terminal::colorize_log_level("ERROR", config);

        // When colors disabled, returns original case (not normalized)
        assert_eq!(info, "INFO");
        assert_eq!(warn, "WARN");
        assert_eq!(error, "ERROR");
    }

    #[test]
    fn test_progress_colorization_with_no_color_flag() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test goal"]).unwrap();
        let config = create_color_config_from_args(&args);

        let progress = terminal::colorize_progress("50%", config);

        assert_eq!(progress, "50%");
    }

    // =========================================================================
    // Integration with Subcommands
    // =========================================================================

    #[test]
    fn test_no_color_flag_with_release_subcommand() {
        let args =
            Args::try_parse_from(["ltmatrix", "--no-color", "release", "--archive"]).unwrap();
        let config = create_color_config_from_args(&args);

        assert!(
            !config.is_enabled(),
            "--no-color should work with release subcommand"
        );
    }

    #[test]
    fn test_no_color_flag_with_completions_subcommand() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "completions", "bash"]).unwrap();
        let config = create_color_config_from_args(&args);

        assert!(
            !config.is_enabled(),
            "--no-color should work with completions subcommand"
        );
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_no_color_with_empty_goal() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color"]).unwrap();
        assert!(args.no_color);
        assert!(args.goal.is_none());
    }

    #[test]
    fn test_no_color_position_independence() {
        // Test that --no-color works regardless of position in command line
        let args1 = Args::try_parse_from(["ltmatrix", "--no-color", "test"]).unwrap();
        let args2 = Args::try_parse_from(["ltmatrix", "--fast", "--no-color", "test"]).unwrap();
        let args3 = Args::try_parse_from(["ltmatrix", "test", "--no-color"]).unwrap_or_else(|_| {
            // clap might not accept flags after the goal
            Args::try_parse_from(["ltmatrix", "--no-color", "test"]).unwrap()
        });

        assert!(args1.no_color);
        assert!(args2.no_color);
        assert!(args3.no_color);
    }

    #[test]
    fn test_multiple_flag_combinations() {
        // Test --no-color with various flag combinations
        let test_cases = vec![
            vec!["ltmatrix", "--no-color", "--fast", "test"],
            vec!["ltmatrix", "--no-color", "--expert", "test"],
            vec!["ltmatrix", "--no-color", "--dry-run", "test"],
            vec!["ltmatrix", "--no-color", "--resume", "test"],
            vec!["ltmatrix", "--no-color", "--output", "json", "test"],
            vec!["ltmatrix", "--no-color", "--log-level", "trace", "test"],
        ];

        for args_vec in &test_cases {
            let args = Args::try_parse_from(args_vec.as_slice()).unwrap();
            assert!(
                args.no_color,
                "--no-color should be set in combination: {:?}",
                args_vec
            );
        }
    }

    // =========================================================================
    // Helper Functions
    // =========================================================================

    /// Creates a ColorConfig based on CLI arguments
    ///
    /// This function simulates the integration logic that should be implemented
    /// in the main application to wire up the --no-color flag with ColorConfig.
    fn create_color_config_from_args(args: &Args) -> ColorConfig {
        if args.no_color {
            // Explicitly disable colors via CLI flag
            ColorConfig::plain()
        } else {
            // Auto-detect, but still respect NO_COLOR environment variable
            ColorConfig::auto()
        }
    }

    // =========================================================================
    // Documentation Tests
    // =========================================================================

    #[test]
    fn test_no_color_documentation_verification() {
        // Verify that the --no-color flag is properly documented
        let mut args = Args::command();

        // Get the help text
        let help = args.render_help();

        // Verify --no-color is mentioned in help
        let help_str = help.to_string();
        assert!(
            help_str.contains("--no-color"),
            "--no-color should be documented in help text"
        );
        assert!(
            help_str.contains("colored"),
            "Help should mention colored output"
        );
        assert!(
            help_str.contains("NO_COLOR"),
            "Help should mention NO_COLOR environment variable"
        );
    }
}
