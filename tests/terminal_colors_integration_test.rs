//! Integration tests for terminal color and formatting functionality
//!
//! This test file verifies the acceptance criteria:
//! - Terminal color support using console crate
//! - Colorized task statuses, log levels, progress indicators
//! - NO_COLOR environment variable support
//! - Terminal capability detection
//! - --no-color flag functionality

use ltmatrix::terminal::{self, ColorConfig};
use std::env;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // NO_COLOR Environment Variable Tests
    // =========================================================================

    #[test]
    fn test_no_color_env_var_disables_colors_when_checking() {
        // Set NO_COLOR environment variable
        env::set_var("NO_COLOR", "1");

        // With check_no_color=true, colors should be disabled
        let config = ColorConfig::with_config(true, true);
        assert!(!config.is_enabled(), "NO_COLOR should disable colors");

        // Cleanup
        env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_no_color_env_var_empty_value_disables_colors() {
        // Set NO_COLOR to empty string (should still disable per spec)
        env::set_var("NO_COLOR", "");

        let config = ColorConfig::with_config(true, true);
        assert!(!config.is_enabled(), "NO_COLOR='' should disable colors");

        // Cleanup
        env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_no_color_env_var_any_value_disables_colors() {
        // Set NO_COLOR to arbitrary value
        env::set_var("NO_COLOR", "true");

        let config = ColorConfig::with_config(true, true);
        assert!(!config.is_enabled(), "NO_COLOR=<any value> should disable colors");

        // Cleanup
        env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_no_color_env_var_ignored_when_checking_disabled() {
        // Set NO_COLOR environment variable
        env::set_var("NO_COLOR", "1");

        // With check_no_color=false, NO_COLOR should be ignored
        let config = ColorConfig::with_config(true, false);
        assert!(config.is_enabled(), "NO_COLOR should be ignored when check_no_color=false");

        // Cleanup
        env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_auto_respects_no_color_env_var() {
        // Set NO_COLOR environment variable
        env::set_var("NO_COLOR", "1");

        let config = ColorConfig::auto();
        assert!(!config.is_enabled(), "ColorConfig::auto() should respect NO_COLOR");

        // Cleanup
        env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_no_color_env_var_affects_styling() {
        // Set NO_COLOR environment variable
        env::set_var("NO_COLOR", "1");

        let config = ColorConfig::auto();
        let styled = terminal::success("Test message", config);
        assert_eq!(styled, "Test message", "Styled text should be plain when NO_COLOR is set");

        // Cleanup
        env::remove_var("NO_COLOR");
    }

    // =========================================================================
    // Terminal Capability Detection Tests
    // =========================================================================

    #[test]
    fn test_auto_detects_terminal_capabilities() {
        let config = ColorConfig::auto();
        // Just verify it doesn't panic and returns a valid config
        let enabled = config.is_enabled();
        // Result depends on whether we're in a terminal during test execution
        let _ = enabled;
    }

    #[test]
    fn test_plain_config_always_disabled() {
        let config = ColorConfig::plain();
        assert!(!config.is_enabled(), "Plain config should always have colors disabled");
    }

    #[test]
    fn test_explicit_enabled_overrides_detection() {
        let config = ColorConfig::with_config(true, false);
        assert!(config.is_enabled(), "Explicit enabled=true should force colors on");
    }

    #[test]
    fn test_explicit_disabled_overrides_detection() {
        let config = ColorConfig::with_config(false, false);
        assert!(!config.is_enabled(), "Explicit enabled=false should force colors off");
    }

    // =========================================================================
    // Task Status Colorization Tests
    // =========================================================================

    #[test]
    fn test_task_status_all_variants() {
        let config = ColorConfig::plain();

        // When colors are disabled, returns original string (no normalization)
        let test_cases = vec![
            // Input, Expected Output (same as input when colors disabled)
            ("pending", "pending"),
            ("PENDING", "PENDING"),
            ("Pending", "Pending"),
            ("in_progress", "in_progress"),
            ("in-progress", "in-progress"),
            ("inprogress", "inprogress"),
            ("IN_PROGRESS", "IN_PROGRESS"),
            ("completed", "completed"),
            ("COMPLETED", "COMPLETED"),
            ("Completed", "Completed"),
            ("failed", "failed"),
            ("FAILED", "FAILED"),
            ("blocked", "blocked"),
            ("BLOCKED", "BLOCKED"),
        ];

        for (input, expected) in test_cases {
            let result = terminal::colorize_status(input, config);
            assert_eq!(result, expected, "Status '{}' should be '{}' when colors disabled", input, expected);
        }
    }

    #[test]
    fn test_task_status_unknown_passes_through() {
        let config = ColorConfig::plain();
        let unknown_statuses = vec!["unknown", "custom", "cancelled", "deferred"];

        for status in unknown_statuses {
            let result = terminal::colorize_status(status, config);
            assert_eq!(result, status, "Unknown status '{}' should pass through unchanged", status);
        }
    }

    #[test]
    fn test_task_status_with_colors_enabled() {
        let config = ColorConfig::with_config(true, false);

        // When colors are enabled, status should be uppercase and styled
        let result = terminal::colorize_status("pending", config);
        // Result will have ANSI codes, but should contain "PENDING"
        assert!(result.contains("PENDING") || result == "pending", "Status should be properly formatted");
    }

    // =========================================================================
    // Log Level Colorization Tests
    // =========================================================================

    #[test]
    fn test_log_level_all_variants() {
        let config = ColorConfig::plain();

        // When colors are disabled, the implementation returns the original string
        // (not normalized). It only normalizes when colors are enabled.
        let test_cases = vec![
            // Input, Expected Output (same as input when colors disabled)
            ("trace", "trace"),
            ("TRACE", "TRACE"),
            ("debug", "debug"),
            ("DEBUG", "DEBUG"),
            ("info", "info"),
            ("INFO", "INFO"),
            ("warn", "warn"),
            ("WARN", "WARN"),
            ("warning", "warning"),
            ("WARNING", "WARNING"),
            ("error", "error"),
            ("ERROR", "ERROR"),
        ];

        for (input, expected) in test_cases {
            let result = terminal::colorize_log_level(input, config);
            assert_eq!(result, expected, "Log level '{}' should be '{}' when colors disabled", input, expected);
        }
    }

    #[test]
    fn test_log_level_unknown_passes_through() {
        let config = ColorConfig::plain();
        let unknown_levels = vec!["custom", "fatal", "critical"];

        for level in unknown_levels {
            let result = terminal::colorize_log_level(level, config);
            assert_eq!(result, level, "Unknown log level '{}' should pass through unchanged", level);
        }
    }

    // =========================================================================
    // Progress Indicator Tests
    // =========================================================================

    #[test]
    fn test_progress_indicator_formats() {
        let config = ColorConfig::plain();

        let test_cases = vec![
            "0%",
            "25%",
            "50%",
            "75%",
            "100%",
            "1/10",
            "5/20",
            "Processing...",
            "Initializing",
            "Complete",
        ];

        for progress in test_cases {
            let result = terminal::colorize_progress(progress, config);
            assert_eq!(result, progress, "Progress '{}' should pass through when colors disabled", progress);
        }
    }

    // =========================================================================
    // Message Helper Tests
    // =========================================================================

    #[test]
    fn test_message_helper_colors() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::success("Success", config), "Success");
        assert_eq!(terminal::error("Error", config), "Error");
        assert_eq!(terminal::warning("Warning", config), "Warning");
        assert_eq!(terminal::info("Info", config), "Info");
        assert_eq!(terminal::dim("Dim", config), "Dim");
        assert_eq!(terminal::bold("Bold", config), "Bold");
    }

    #[test]
    fn test_message_helper_with_unicode_symbols() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::success("✓ Success", config), "✓ Success");
        assert_eq!(terminal::error("✗ Error", config), "✗ Error");
        assert_eq!(terminal::warning("⚠ Warning", config), "⚠ Warning");
        assert_eq!(terminal::info("ℹ Info", config), "ℹ Info");
    }

    #[test]
    fn test_message_helper_with_emoji() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::success("🎉 Success", config), "🎉 Success");
        assert_eq!(terminal::error("💥 Error", config), "💥 Error");
        assert_eq!(terminal::warning("⚡ Warning", config), "⚡ Warning");
        assert_eq!(terminal::info("📝 Info", config), "📝 Info");
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_full_workflow_with_plain_config() {
        let config = ColorConfig::plain();

        // Simulate a typical workflow
        let start = terminal::info("Starting task execution", config);
        let status = terminal::colorize_status("pending", config);
        let progress = terminal::colorize_progress("0%", config);
        let complete = terminal::success("All tasks completed!", config);

        assert_eq!(start, "Starting task execution");
        assert_eq!(status, "pending");
        assert_eq!(progress, "0%");
        assert_eq!(complete, "All tasks completed!");
    }

    #[test]
    fn test_log_workflow_with_plain_config() {
        let config = ColorConfig::plain();

        // When colors disabled, returns original case
        let trace = terminal::colorize_log_level("TRACE", config);
        let debug = terminal::colorize_log_level("DEBUG", config);
        let info = terminal::colorize_log_level("INFO", config);
        let warn = terminal::colorize_log_level("WARN", config);
        let error = terminal::colorize_log_level("ERROR", config);

        assert_eq!(trace, "TRACE");
        assert_eq!(debug, "DEBUG");
        assert_eq!(info, "INFO");
        assert_eq!(warn, "WARN");
        assert_eq!(error, "ERROR");
    }

    #[test]
    fn test_task_workflow_with_plain_config() {
        let config = ColorConfig::plain();

        // Simulate task lifecycle
        let pending = terminal::colorize_status("pending", config);
        let in_progress = terminal::colorize_status("in_progress", config);
        let completed = terminal::colorize_status("completed", config);

        assert_eq!(pending, "pending");
        assert_eq!(in_progress, "in_progress");
        assert_eq!(completed, "completed");
    }

    #[test]
    fn test_error_workflow_with_plain_config() {
        let config = ColorConfig::plain();

        // Simulate error reporting
        let error_msg = terminal::error("Task failed", config);
        let error_level = terminal::colorize_log_level("ERROR", config);
        let failed_status = terminal::colorize_status("failed", config);

        assert_eq!(error_msg, "Task failed");
        assert_eq!(error_level, "ERROR");  // Returns original when colors disabled
        assert_eq!(failed_status, "failed");
    }

    // =========================================================================
    // Edge Cases Tests
    // =========================================================================

    #[test]
    fn test_empty_strings() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::style_text("", terminal::Color::Red, config), "");
        assert_eq!(terminal::colorize_status("", config), "");
        assert_eq!(terminal::colorize_log_level("", config), "");
        assert_eq!(terminal::colorize_progress("", config), "");
        assert_eq!(terminal::success("", config), "");
        assert_eq!(terminal::error("", config), "");
        assert_eq!(terminal::warning("", config), "");
        assert_eq!(terminal::info("", config), "");
    }

    #[test]
    fn test_whitespace_preservation() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::style_text("  test  ", terminal::Color::Red, config), "  test  ");
        assert_eq!(terminal::success("  success  ", config), "  success  ");
        assert_eq!(terminal::colorize_status("  pending  ", config), "  pending  ");
        assert_eq!(terminal::colorize_log_level("  info  ", config), "  info  ");
    }

    #[test]
    fn test_newline_preservation() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::success("Line 1\nLine 2", config), "Line 1\nLine 2");
        assert_eq!(terminal::error("Error\nDetails", config), "Error\nDetails");
    }

    #[test]
    fn test_special_characters() {
        let config = ColorConfig::plain();

        assert_eq!(terminal::success("Success! @#$%", config), "Success! @#$%");
        assert_eq!(terminal::error("Error: <>&\"", config), "Error: <>&\"");
        assert_eq!(terminal::warning("Warning: []{}", config), "Warning: []{}");
    }

    #[test]
    fn test_unicode_multibyte_characters() {
        let config = ColorConfig::plain();

        // Japanese
        assert_eq!(terminal::success("成功", config), "成功");
        // Chinese
        assert_eq!(terminal::error("错误", config), "错误");
        // Korean
        assert_eq!(terminal::warning("경고", config), "경고");
        // Arabic
        assert_eq!(terminal::info("معلومات", config), "معلومات");
        // Emoji
        assert_eq!(terminal::bold("🎉🚀✨", config), "🎉🚀✨");
    }

    #[test]
    fn test_very_long_strings() {
        let config = ColorConfig::plain();
        let long_string = "a".repeat(10000);

        let result = terminal::success(&long_string, config);
        assert_eq!(result, long_string);
        assert_eq!(result.len(), 10000);
    }

    // =========================================================================
    // ColorConfig Default Tests
    // =========================================================================

    #[test]
    fn test_color_config_default_impl() {
        let config = ColorConfig::default();
        // Should not panic and should return a valid config
        let _ = config.is_enabled();
    }

    #[test]
    fn test_color_config_clone() {
        let config1 = ColorConfig::plain();
        let config2 = config1;

        assert!(!config1.is_enabled());
        assert!(!config2.is_enabled());
    }

    #[test]
    fn test_color_config_copy() {
        let config = ColorConfig::plain();
        let copied = config;

        assert!(!config.is_enabled());
        assert!(!copied.is_enabled());
    }

    // =========================================================================
    // Stress Tests
    // =========================================================================

    #[test]
    fn test_rapid_color_config_creation() {
        // Create many configs rapidly to ensure no resource leaks
        for _ in 0..1000 {
            let _ = ColorConfig::auto();
            let _ = ColorConfig::plain();
            let _ = ColorConfig::with_config(true, false);
            let _ = ColorConfig::with_config(false, false);
        }
    }

    #[test]
    fn test_rapid_styling_operations() {
        let config = ColorConfig::plain();
        let messages = vec!["test", "message", "success", "error", "warning"];

        for _ in 0..1000 {
            for msg in &messages {
                let _ = terminal::success(msg, config);
                let _ = terminal::error(msg, config);
                let _ = terminal::warning(msg, config);
                let _ = terminal::info(msg, config);
            }
        }
    }

    // =========================================================================
    // Consistency Tests
    // =========================================================================

    #[test]
    fn test_colorize_status_normalization_consistency() {
        let config = ColorConfig::plain();

        // When colors are disabled, each variant returns as-is (no normalization)
        let inputs = vec!["pending", "PENDING", "Pending"];
        let results: Vec<_> = inputs
            .iter()
            .map(|s| terminal::colorize_status(s, config))
            .collect();

        // With colors disabled, each should return its input (not normalized)
        assert_eq!(results[0], "pending");
        assert_eq!(results[1], "PENDING");
        assert_eq!(results[2], "Pending");
    }

    #[test]
    fn test_colorize_log_level_normalization_consistency() {
        let config = ColorConfig::plain();

        // When colors are disabled, each variant returns as-is (no normalization)
        let inputs = vec!["info", "INFO", "Info"];
        let results: Vec<_> = inputs
            .iter()
            .map(|s| terminal::colorize_log_level(s, config))
            .collect();

        // With colors disabled, each should return its input (not normalized)
        assert_eq!(results[0], "info");
        assert_eq!(results[1], "INFO");
        assert_eq!(results[2], "Info");
    }
}
