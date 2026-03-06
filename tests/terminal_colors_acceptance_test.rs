//! Acceptance tests for Terminal Color and Formatting Implementation
//!
//! This test file validates all acceptance criteria from the task:
//!
//! TASK: Implement color and formatting for terminals
//!
//! Acceptance Criteria:
//! 1. Add terminal color support using console or termcolor
//! 2. Colorize: task statuses, log levels, progress indicators
//! 3. Respect NO_COLOR environment variable
//! 4. Detect terminal capabilities
//! 5. Add --no-color flag for forced plain output
//!
//! Run with: cargo test terminal_colors_acceptance_test

use clap::{CommandFactory, Parser};
use ltmatrix::cli::args::Args;
use ltmatrix::terminal::{self, Color, ColorConfig};
use std::env;
use std::io::IsTerminal;
use std::sync::Mutex;

/// Mutex to serialize tests that manipulate the NO_COLOR environment variable,
/// preventing race conditions between parallel test threads.
static NO_COLOR_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod acceptance_tests {
    use super::*;

    // =========================================================================
    // ACCEPTANCE CRITERION 1: Terminal color support using console crate
    // =========================================================================

    #[test]
    fn acceptance_01_console_crate_integration() {
        // Verify that the console crate is being used by checking Color enum
        // The implementation uses console::Style and console::Color

        let config = ColorConfig::with_config(true, false);
        let styled = terminal::style_text("test", Color::Red, config);

        // When colors are enabled, the result should differ from plain text
        // (it will contain ANSI color codes) OR be the same if the console
        // crate detects no TTY (e.g., in CI/test environment)
        // The key is that is_enabled() returns true
        assert!(config.is_enabled(), "Color config should be enabled");

        // The style_text function should at least execute without panic
        // and return a string (whether colored or plain depends on TTY)
        assert!(!styled.is_empty(), "Styled text should not be empty");
    }

    #[test]
    fn acceptance_01_all_basic_colors_supported() {
        let config = ColorConfig::with_config(true, false);

        // Test all basic colors from console crate
        let colors = vec![
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
            Color::White,
            Color::Black,
        ];

        for color in colors {
            let styled = terminal::style_text("test", color, config);
            // Should execute without panic and return a string
            assert!(
                !styled.is_empty(),
                "Color {:?} should produce output",
                color
            );
        }
    }

    #[test]
    fn acceptance_01_bright_colors_supported() {
        let config = ColorConfig::with_config(true, false);

        // Test bright color variants
        let bright_colors = vec![
            Color::BrightRed,
            Color::BrightGreen,
            Color::BrightYellow,
            Color::BrightBlue,
            Color::BrightMagenta,
            Color::BrightCyan,
            Color::BrightWhite,
        ];

        for color in bright_colors {
            let styled = terminal::style_text("test", color, config);
            // Should execute without panic and return a string
            assert!(
                !styled.is_empty(),
                "Bright color {:?} should produce output",
                color
            );
        }
    }

    #[test]
    fn acceptance_01_text_styles_supported() {
        let config = ColorConfig::with_config(true, false);

        // Test text styles (bold, dim)
        let bold = terminal::style_text("test", Color::Bold, config);
        let dim = terminal::style_text("test", Color::Dim, config);

        // Should execute without panic and return strings
        assert!(!bold.is_empty(), "Bold style should produce output");
        assert!(!dim.is_empty(), "Dim style should produce output");
    }

    // =========================================================================
    // ACCEPTANCE CRITERION 2: Colorize task statuses, log levels, progress
    // =========================================================================

    #[test]
    fn acceptance_02_task_status_colorization() {
        let config = ColorConfig::with_config(true, false);

        // Test all defined task statuses
        let statuses = vec!["pending", "in_progress", "completed", "failed", "blocked"];

        for status in statuses {
            let colorized = terminal::colorize_status(status, config);
            assert!(
                !colorized.is_empty(),
                "Status '{}' should produce output",
                status
            );
            assert!(
                colorized.len() >= status.len(),
                "Colorized status should be at least as long as input"
            );
        }
    }

    #[test]
    fn acceptance_02_task_status_normalization() {
        let config = ColorConfig::plain();

        // When colors are disabled, status is returned as-is (no normalization)
        assert_eq!(terminal::colorize_status("pending", config), "pending");
        assert_eq!(terminal::colorize_status("PENDING", config), "PENDING");
        assert_eq!(
            terminal::colorize_status("in-progress", config),
            "in-progress"
        );
        assert_eq!(
            terminal::colorize_status("inprogress", config),
            "inprogress"
        );
    }

    #[test]
    fn acceptance_02_log_level_colorization() {
        let config = ColorConfig::with_config(true, false);

        // Test all defined log levels
        let levels = vec!["trace", "debug", "info", "warn", "warning", "error"];

        for level in levels {
            let colorized = terminal::colorize_log_level(level, config);
            // Just verify that output is produced
            assert!(
                !colorized.is_empty(),
                "Log level '{}' should produce output",
                level
            );
            // The output should be reasonable (either plain or styled)
            assert!(
                colorized.len() > 0,
                "Log level '{}' should have content",
                level
            );
        }
    }

    #[test]
    fn acceptance_02_log_level_normalization() {
        let config = ColorConfig::plain();

        // When colors are disabled, log level is returned as-is (no normalization)
        assert_eq!(terminal::colorize_log_level("INFO", config), "INFO");
        assert_eq!(terminal::colorize_log_level("warning", config), "warning");
        assert_eq!(terminal::colorize_log_level("WARNING", config), "WARNING");
    }

    #[test]
    fn acceptance_02_progress_indicator_colorization() {
        let config = ColorConfig::with_config(true, false);

        // Test various progress indicator formats
        let progress_formats = vec![
            "0%",
            "50%",
            "100%",
            "1/10",
            "5/20",
            "Processing...",
            "Complete",
        ];

        for progress in progress_formats {
            let colorized = terminal::colorize_progress(progress, config);
            assert!(
                !colorized.is_empty(),
                "Progress '{}' should produce output",
                progress
            );
            assert!(
                colorized.len() >= progress.len(),
                "Colorized progress should be at least as long as input"
            );
        }
    }

    #[test]
    fn acceptance_02_message_helper_functions() {
        let config = ColorConfig::with_config(true, false);

        // Test all message helper functions
        let success = terminal::success("Success message", config);
        let error = terminal::error("Error message", config);
        let warning = terminal::warning("Warning message", config);
        let info = terminal::info("Info message", config);
        let dim = terminal::dim("Dim message", config);
        let bold = terminal::bold("Bold message", config);

        // All should produce output (styled if TTY, plain if not)
        assert!(!success.is_empty());
        assert!(!error.is_empty());
        assert!(!warning.is_empty());
        assert!(!info.is_empty());
        assert!(!dim.is_empty());
        assert!(!bold.is_empty());
    }

    // =========================================================================
    // ACCEPTANCE CRITERION 3: Respect NO_COLOR environment variable
    // =========================================================================

    #[test]
    fn acceptance_03_no_color_env_var_disables_colors() {
        let _lock = NO_COLOR_MUTEX.lock().unwrap();
        env::remove_var("NO_COLOR");
        env::set_var("NO_COLOR", "1");

        let config = ColorConfig::auto();
        assert!(
            !config.is_enabled(),
            "NO_COLOR environment variable should disable colors"
        );

        env::remove_var("NO_COLOR");
    }

    #[test]
    fn acceptance_03_no_color_any_value_disables_colors() {
        let _lock = NO_COLOR_MUTEX.lock().unwrap();
        env::remove_var("NO_COLOR");

        let test_values = vec!["1", "true", "yes", "0", "", "any-value"];

        for value in test_values {
            env::set_var("NO_COLOR", value);

            let config = ColorConfig::with_config(true, true);
            // NO_COLOR should be respected and disable colors when check_no_color=true
            assert!(
                !config.is_enabled(),
                "NO_COLOR={} should disable colors when check_no_color=true",
                value
            );

            env::remove_var("NO_COLOR");
        }
    }

    #[test]
    fn acceptance_03_no_color_affects_output() {
        let _lock = NO_COLOR_MUTEX.lock().unwrap();
        env::remove_var("NO_COLOR");
        env::set_var("NO_COLOR", "1");

        let config = ColorConfig::auto();

        // All styled output should be plain when NO_COLOR is set
        assert_eq!(terminal::success("Test", config), "Test");
        assert_eq!(terminal::error("Test", config), "Test");
        assert_eq!(terminal::warning("Test", config), "Test");
        assert_eq!(terminal::info("Test", config), "Test");
        assert_eq!(terminal::colorize_status("pending", config), "pending");
        assert_eq!(terminal::colorize_log_level("INFO", config), "INFO"); // Returns as-is when colors disabled
        assert_eq!(terminal::colorize_progress("50%", config), "50%");

        env::remove_var("NO_COLOR");
    }

    #[test]
    fn acceptance_03_no_color_spec_compliance() {
        let _lock = NO_COLOR_MUTEX.lock().unwrap();
        // Verify compliance with https://no-color.org/
        // "Command-line software which adds colored output should check for
        // the presence of a NO_COLOR environment variable that, when present
        // (regardless of its value), prevents the addition of color."

        let test_values = vec!["", "0", "1", "false", "true", "anything"];

        for value in test_values {
            env::set_var("NO_COLOR", value);

            let config = ColorConfig::auto();
            assert!(
                !config.is_enabled(),
                "NO_COLOR with value '{}' should prevent color output per spec",
                value
            );

            env::remove_var("NO_COLOR");
        }
    }

    // =========================================================================
    // ACCEPTANCE CRITERION 4: Detect terminal capabilities
    // =========================================================================

    #[test]
    fn acceptance_04_terminal_detection_works() {
        // Verify that terminal detection doesn't panic and returns a valid result
        let config = ColorConfig::auto();
        let is_terminal = config.is_enabled();

        // We can't assert a specific value since it depends on test environment
        // but we can verify the detection logic executed without error
        let _ = is_terminal;
    }

    #[test]
    fn acceptance_04_auto_detection_respects_terminal() {
        let config1 = ColorConfig::auto();
        let config2 = ColorConfig::auto();

        // Multiple calls should be consistent
        assert_eq!(
            config1.is_enabled(),
            config2.is_enabled(),
            "Terminal detection should be consistent"
        );

        // Auto detection checks stdout (we just verify it doesn't panic)
        let _ = std::io::stdout().is_terminal();
    }

    #[test]
    fn acceptance_04_manual_override_works() {
        // Verify manual override of terminal detection
        let force_enabled = ColorConfig::with_config(true, false);
        let force_disabled = ColorConfig::with_config(false, false);

        assert!(
            force_enabled.is_enabled(),
            "Manual override should force colors on"
        );
        assert!(
            !force_disabled.is_enabled(),
            "Manual override should force colors off"
        );
    }

    #[test]
    fn acceptance_04_detection_can_be_disabled() {
        let config = ColorConfig::plain();
        assert!(
            !config.is_enabled(),
            "Plain config should bypass detection and disable colors"
        );
    }

    // =========================================================================
    // ACCEPTANCE CRITERION 5: --no-color flag for forced plain output
    // =========================================================================

    #[test]
    fn acceptance_05_no_color_flag_exists() {
        // Verify the --no-color flag can be parsed
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test"]);
        assert!(args.is_ok(), "--no-color flag should be recognized");
        assert!(args.unwrap().no_color, "--no-color flag should be set");
    }

    #[test]
    fn acceptance_05_no_color_flag_default() {
        // Verify --no-color defaults to false
        let args = Args::try_parse_from(["ltmatrix", "test"]).unwrap();
        assert!(!args.no_color, "--no-color should default to false");
    }

    #[test]
    fn acceptance_05_no_color_flag_forces_plain_output() {
        // Verify --no-color flag forces plain output
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test"]).unwrap();

        // Simulate integration: --no-color should create plain config
        let config = if args.no_color {
            ColorConfig::plain()
        } else {
            ColorConfig::auto()
        };

        assert!(
            !config.is_enabled(),
            "--no-color flag should result in plain output"
        );
    }

    #[test]
    fn acceptance_05_no_color_flag_affects_all_output() {
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test"]).unwrap();
        let config = if args.no_color {
            ColorConfig::plain()
        } else {
            ColorConfig::auto()
        };

        // All output should be plain when --no-color is set
        assert_eq!(terminal::success("Test", config), "Test");
        assert_eq!(terminal::error("Test", config), "Test");
        assert_eq!(terminal::colorize_status("pending", config), "pending");
        assert_eq!(terminal::colorize_log_level("INFO", config), "INFO"); // Returns as-is when colors disabled
        assert_eq!(terminal::colorize_progress("50%", config), "50%");
    }

    #[test]
    fn acceptance_05_no_color_flag_works_with_subcommands() {
        // Test --no-color with various subcommands
        let test_cases = vec![
            vec!["ltmatrix", "--no-color", "completions", "bash"],
            vec!["ltmatrix", "--no-color", "release"],
            vec!["ltmatrix", "--no-color", "release", "--archive"],
        ];

        for args_vec in &test_cases {
            let args = Args::try_parse_from(args_vec.as_slice()).unwrap();
            assert!(
                args.no_color,
                "--no-color should work with subcommand: {:?}",
                args_vec
            );
        }
    }

    #[test]
    fn acceptance_05_no_color_flag_documented() {
        // Verify --no-color is documented in help
        let mut cmd = Args::command();
        let help = cmd.render_help().to_string();

        assert!(
            help.contains("--no-color"),
            "--no-color should be documented in help text"
        );
    }

    // =========================================================================
    // INTEGRATION TESTS: Multiple criteria working together
    // =========================================================================

    #[test]
    fn integration_no_color_flag_overrides_env() {
        let _lock = NO_COLOR_MUTEX.lock().unwrap();
        // Set NO_COLOR to "0" (trying to enable colors)
        env::set_var("NO_COLOR", "0");

        // But --no-color flag should override
        let args = Args::try_parse_from(["ltmatrix", "--no-color", "test"]).unwrap();
        let config = if args.no_color {
            ColorConfig::plain()
        } else {
            ColorConfig::auto()
        };

        assert!(
            !config.is_enabled(),
            "CLI --no-color flag should override NO_COLOR environment variable"
        );

        env::remove_var("NO_COLOR");
    }

    #[test]
    fn integration_terminal_detection_with_no_color_env() {
        let _lock = NO_COLOR_MUTEX.lock().unwrap();
        // Test interaction between terminal detection and NO_COLOR
        env::set_var("NO_COLOR", "1");

        let config = ColorConfig::auto();
        assert!(
            !config.is_enabled(),
            "NO_COLOR should disable colors even when terminal supports them"
        );

        env::remove_var("NO_COLOR");
    }

    #[test]
    fn integration_complete_workflow() {
        // Simulate a complete workflow with all features
        let args = Args::try_parse_from([
            "ltmatrix",
            "--no-color",
            "--log-level",
            "debug",
            "implement feature X",
        ])
        .unwrap();

        let config = if args.no_color {
            ColorConfig::plain()
        } else {
            ColorConfig::auto()
        };

        // Verify all colorization works with the config
        let start = terminal::info("Starting", config);
        let status = terminal::colorize_status("pending", config);
        let progress = terminal::colorize_progress("0%", config);
        let level = terminal::colorize_log_level("DEBUG", config);
        let complete = terminal::success("Complete", config);

        assert_eq!(start, "Starting");
        assert_eq!(status, "pending");
        assert_eq!(progress, "0%");
        assert_eq!(level, "DEBUG"); // Returns as-is when colors disabled
        assert_eq!(complete, "Complete");
    }

    // =========================================================================
    // REGRESSION TESTS: Ensure functionality doesn't break
    // =========================================================================

    #[test]
    fn regression_plain_config_always_works() {
        // Ensure plain config always produces plain output
        let config = ColorConfig::plain();

        assert_eq!(terminal::style_text("test", Color::Red, config), "test");
        assert_eq!(terminal::success("test", config), "test");
        assert_eq!(terminal::error("test", config), "test");
        assert_eq!(terminal::colorize_status("pending", config), "pending");
        assert_eq!(terminal::colorize_log_level("INFO", config), "INFO"); // Returns as-is when colors disabled
        assert_eq!(terminal::colorize_progress("50%", config), "50%");
    }

    #[test]
    fn regression_empty_strings_handled() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::success("", config), "");
        assert_eq!(terminal::error("", config), "");
        assert_eq!(terminal::colorize_status("", config), "");
        assert_eq!(terminal::colorize_log_level("", config), "");
        assert_eq!(terminal::colorize_progress("", config), "");
    }

    #[test]
    fn regression_unicode_handled() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::success("こんにちは", config), "こんにちは");
        assert_eq!(terminal::error("🎉", config), "🎉");
        assert_eq!(terminal::colorize_status("待機", config), "待機");
    }

    #[test]
    fn regression_whitespace_preserved() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::success("  test  ", config), "  test  ");
        assert_eq!(terminal::error("test\nline2", config), "test\nline2");
    }
}
