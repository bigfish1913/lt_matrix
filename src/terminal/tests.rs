//! Tests for terminal color and formatting functionality

#[cfg(test)]
mod tests {
    use super::super::*;

    // =========================================================================
    // ColorConfig Tests
    // =========================================================================

    #[test]
    fn test_color_config_auto() {
        let config = ColorConfig::auto();
        // Just ensure it doesn't panic and returns a value
        let enabled = config.is_enabled();
        // Result depends on whether we're in a terminal
        let _ = enabled;
    }

    #[test]
    fn test_color_config_plain() {
        let config = ColorConfig::plain();
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_color_config_with_config() {
        let config = ColorConfig::with_config(true, false);
        assert!(config.is_enabled());

        let config = ColorConfig::with_config(false, false);
        assert!(!config.is_enabled());

        // NO_COLOR env var should disable colors even when enabled=true
        let config = ColorConfig::with_config(true, true);
        // Result depends on NO_COLOR env var
        let _ = config.is_enabled();
    }

    #[test]
    fn test_color_config_default() {
        let config = ColorConfig::default();
        // Just ensure it doesn't panic
        let _ = config.is_enabled();
    }

    // =========================================================================
    // Color Styling Tests
    // =========================================================================

    #[test]
    fn test_style_text_with_colors_disabled() {
        let config = ColorConfig::plain();
        assert_eq!(style_text("test", Color::Red, config), "test");
        assert_eq!(style_text("hello", Color::Green, config), "hello");
        assert_eq!(style_text("world", Color::Blue, config), "world");
    }

    #[test]
    fn test_style_text_all_colors() {
        let config = ColorConfig::plain();

        let colors = vec![
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
            Color::White,
            Color::Black,
            Color::BrightRed,
            Color::BrightGreen,
            Color::BrightYellow,
            Color::BrightBlue,
            Color::BrightMagenta,
            Color::BrightCyan,
            Color::BrightWhite,
            Color::Dim,
            Color::Bold,
        ];

        for color in colors {
            let result = style_text("test", color, config);
            assert_eq!(result, "test");
        }
    }

    // =========================================================================
    // Status Colorization Tests
    // =========================================================================

    #[test]
    fn test_colorize_status_all_statuses() {
        let config = ColorConfig::plain();

        // Test all status values
        assert_eq!(colorize_status("pending", config), "pending");
        assert_eq!(colorize_status("PENDING", config), "pending");
        assert_eq!(colorize_status("Pending", config), "pending");

        assert_eq!(colorize_status("in_progress", config), "in_progress");
        assert_eq!(colorize_status("in-progress", config), "in_progress");
        assert_eq!(colorize_status("inprogress", config), "in_progress");

        assert_eq!(colorize_status("completed", config), "completed");
        assert_eq!(colorize_status("failed", config), "failed");
        assert_eq!(colorize_status("blocked", config), "blocked");

        // Unknown status should pass through
        assert_eq!(colorize_status("unknown", config), "unknown");
    }

    #[test]
    fn test_colorize_status_case_insensitive() {
        let config = ColorConfig::plain();

        assert_eq!(colorize_status("PENDING", config), "pending");
        assert_eq!(colorize_status("Pending", config), "pending");
        assert_eq!(colorize_status("pEnDiNg", config), "pending");
    }

    // =========================================================================
    // Log Level Colorization Tests
    // =========================================================================

    #[test]
    fn test_colorize_log_level_all_levels() {
        let config = ColorConfig::plain();

        assert_eq!(colorize_log_level("trace", config), "trace");
        assert_eq!(colorize_log_level("TRACE", config), "trace");

        assert_eq!(colorize_log_level("debug", config), "debug");
        assert_eq!(colorize_log_level("DEBUG", config), "debug");

        assert_eq!(colorize_log_level("info", config), "info");
        assert_eq!(colorize_log_level("INFO", config), "info");

        assert_eq!(colorize_log_level("warn", config), "warn");
        assert_eq!(colorize_log_level("WARN", config), "warn");
        assert_eq!(colorize_log_level("warning", config), "warn");

        assert_eq!(colorize_log_level("error", config), "error");
        assert_eq!(colorize_log_level("ERROR", config), "error");
    }

    #[test]
    fn test_colorize_log_level_case_insensitive() {
        let config = ColorConfig::plain();

        assert_eq!(colorize_log_level("INFO", config), "info");
        assert_eq!(colorize_log_level("Info", config), "info");
        assert_eq!(colorize_log_level("iNfO", config), "info");
    }

    // =========================================================================
    // Progress Colorization Tests
    // =========================================================================

    #[test]
    fn test_colorize_progress() {
        let config = ColorConfig::plain();
        assert_eq!(colorize_progress("50%", config), "50%");
        assert_eq!(colorize_progress("3/10", config), "3/10");
        assert_eq!(colorize_progress("Processing...", config), "Processing...");
    }

    // =========================================================================
    // Message Helper Tests
    // =========================================================================

    #[test]
    fn test_message_helpers() {
        let config = ColorConfig::plain();

        assert_eq!(success("Success!", config), "Success!");
        assert_eq!(error("Error!", config), "Error!");
        assert_eq!(warning("Warning!", config), "Warning!");
        assert_eq!(info("Info!", config), "Info!");
        assert_eq!(dim("Dim", config), "Dim");
        assert_eq!(bold("Bold", config), "Bold");
    }

    #[test]
    fn test_message_helpers_with_special_characters() {
        let config = ColorConfig::plain();

        assert_eq!(success("✓ Success!", config), "✓ Success!");
        assert_eq!(error("✗ Error!", config), "✗ Error!");
        assert_eq!(warning("⚠ Warning!", config), "⚠ Warning!");
        assert_eq!(info("ℹ Info!", config), "ℹ Info!");
    }

    // =========================================================================
    // Empty String Tests
    // =========================================================================

    #[test]
    fn test_empty_strings() {
        let config = ColorConfig::plain();

        assert_eq!(style_text("", Color::Red, config), "");
        assert_eq!(colorize_status("", config), "");
        assert_eq!(colorize_log_level("", config), "");
        assert_eq!(colorize_progress("", config), "");
        assert_eq!(success("", config), "");
        assert_eq!(error("", config), "");
        assert_eq!(warning("", config), "");
        assert_eq!(info("", config), "");
        assert_eq!(dim("", config), "");
        assert_eq!(bold("", config), "");
    }

    // =========================================================================
    // Unicode Tests
    // =========================================================================

    #[test]
    fn test_unicode_strings() {
        let config = ColorConfig::plain();

        assert_eq!(style_text("こんにちは", Color::Red, config), "こんにちは");
        assert_eq!(success("成功", config), "成功");
        assert_eq!(error("错误", config), "错误");
        assert_eq!(colorize_status("待機", config), "待機");
        assert_eq!(colorize_log_level("情報", config), "情報");
    }

    // =========================================================================
    // Whitespace Tests
    // =========================================================================

    #[test]
    fn test_whitespace_preservation() {
        let config = ColorConfig::plain();

        assert_eq!(style_text("  test  ", Color::Red, config), "  test  ");
        assert_eq!(success("  success  ", config), "  success  ");
        assert_eq!(colorize_status("  pending  ", config), "  pending  ");
        assert_eq!(colorize_log_level("  info  ", config), "  info  ");
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_full_workflow() {
        let config = ColorConfig::plain();

        // Simulate a workflow with different message types
        let start_msg = info("Starting workflow", config);
        let pending_msg = colorize_status("pending", config);
        let progress_msg = colorize_progress("0%", config);
        let complete_msg = success("Workflow complete!", config);

        assert_eq!(start_msg, "Starting workflow");
        assert_eq!(pending_msg, "pending");
        assert_eq!(progress_msg, "0%");
        assert_eq!(complete_msg, "Workflow complete!");
    }
}
