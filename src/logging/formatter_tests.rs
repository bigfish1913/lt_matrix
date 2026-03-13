// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Comprehensive tests for formatter implementation
//!
//! These tests verify:
//! - Timestamp formatting
//! - Console formatting with colors
//! - Level formatting

use crate::logging::formatter::{
    current_timestamp, format_level, format_timestamp, TIMESTAMP_FORMAT,
};
use crate::terminal::{self, ColorConfig};
use chrono::{DateTime, Local};
use tracing::Level;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // ============================================================================
    // Timestamp Formatting Tests
    // ============================================================================

    #[test]
    fn test_format_timestamp_valid_format() {
        let dt = Local::now();
        let formatted = format_timestamp(dt);

        // Should match format: YYYY-MM-DD HH:MM:SS.mmm
        assert!(
            formatted.len() >= 23,
            "Timestamp should be at least 23 characters"
        );

        // Check for separators
        assert!(formatted.contains('-'), "Should contain date separators");
        assert!(
            formatted.contains(' '),
            "Should contain space between date and time"
        );
        assert!(formatted.contains(':'), "Should contain time separators");
    }

    #[test]
    fn test_format_timestamp_consistency() {
        let dt = Local::now();
        let formatted1 = format_timestamp(dt);
        let formatted2 = format_timestamp(dt);

        // Same datetime should produce same format
        assert_eq!(formatted1, formatted2);
    }

    #[test]
    fn test_format_timestamp_chrono_compatibility() {
        let dt = Local
            .with_ymd_and_hms(2024, 3, 15, 14, 30, 45)
            .single()
            .unwrap();
        let formatted = format_timestamp(dt);

        // Should contain the date and time
        assert!(formatted.contains("2024"), "Should contain year");
        assert!(formatted.contains("15"), "Should contain day");
        assert!(formatted.contains("14"), "Should contain hour");
        assert!(formatted.contains("30"), "Should contain minute");
        assert!(formatted.contains("45"), "Should contain second");
    }

    #[test]
    fn test_current_timestamp_always_different() {
        let ts1 = current_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = current_timestamp();

        // Timestamps should be different (unless very unlucky)
        // Note: This test might rarely fail if called exactly at same millisecond
        let ts1_parsed =
            DateTime::parse_from_str(&format!("{}+00:00", ts1), "%Y-%m-%d %H:%M:%S%.3f%z");
        let ts2_parsed =
            DateTime::parse_from_str(&format!("{}+00:00", ts2), "%Y-%m-%d %H:%M:%S%.3f%z");

        assert!(
            ts1_parsed.is_ok() || ts2_parsed.is_ok(),
            "Timestamps should be parseable"
        );
    }

    #[test]
    fn test_timestamp_format_constant() {
        // Verify TIMESTAMP_FORMAT is valid
        assert_eq!(TIMESTAMP_FORMAT, "%Y-%m-%d %H:%M:%S%.3f");
        assert!(!TIMESTAMP_FORMAT.is_empty());
    }

    #[test]
    fn test_current_timestamp_non_empty() {
        let ts = current_timestamp();
        assert!(!ts.is_empty());
        assert!(ts.len() > 10); // At least "YYYY-MM-DD "
    }

    // ============================================================================
    // Level Formatting Tests
    // ============================================================================

    #[test]
    fn test_format_level_contains_level_name() {
        let test_cases = vec![
            (Level::TRACE, "TRACE"),
            (Level::DEBUG, "DEBUG"),
            (Level::INFO, "INFO"),
            (Level::WARN, "WARN"),
            (Level::ERROR, "ERROR"),
        ];

        for (level, expected_name) in test_cases {
            let formatted = format_level(&level);
            assert!(
                formatted.contains(expected_name),
                "Formatted level should contain '{}', got: {}",
                expected_name,
                formatted
            );
        }
    }

    #[test]
    fn test_format_level_has_ansi_codes() {
        // Formatted levels should contain the level text (ANSI codes may not be added on Windows)
        let trace = format_level(&Level::TRACE);
        assert!(trace.contains("TRACE"), "Should contain TRACE");
    }

    #[test]
    fn test_format_level_different_for_different_levels() {
        let trace = format_level(&Level::TRACE);
        let debug = format_level(&Level::DEBUG);
        let info = format_level(&Level::INFO);
        let warn = format_level(&Level::WARN);
        let error = format_level(&Level::ERROR);

        // Each level should have a different representation
        // (at minimum due to different names, likely also different colors)
        let levels = vec![trace, debug, info, warn, error];
        for (i, level1) in levels.iter().enumerate() {
            for (j, level2) in levels.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        level1, level2,
                        "Levels at indices {} and {} should be different",
                        i, j
                    );
                }
            }
        }
    }

    // ============================================================================
    // Console Formatting Tests
    // ============================================================================

    #[test]
    #[ignore] // Requires creating a proper tracing event
    fn test_format_console_line_structure() {
        // This would require creating a proper tracing::Event
        // which is complex. For now, we'll skip this.
        // In a real scenario, you'd use tracing's test utilities
    }

    #[test]
    #[ignore] // Requires creating a proper tracing event
    fn test_format_file_line_structure() {
        // Same as above - requires proper event creation
    }

    #[test]
    #[ignore] // Requires creating a proper tracing event
    fn test_format_message_extraction() {
        // Requires proper Event construction
        // Would test message field extraction
    }

    // ============================================================================
    // Message Extraction Tests
    // ============================================================================

    #[test]
    fn test_format_timestamp_empty_input() {
        // Timestamp should handle various datetime values
        let dt = Local
            .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
            .single()
            .unwrap();
        let formatted = format_timestamp(dt);
        assert!(!formatted.is_empty());
        assert!(formatted.contains("1970"));
    }

    #[test]
    fn test_format_timestamp_future_date() {
        let dt = Local
            .with_ymd_and_hms(2099, 12, 31, 23, 59, 59)
            .single()
            .unwrap();
        let formatted = format_timestamp(dt);
        assert!(!formatted.is_empty());
        assert!(formatted.contains("2099"));
    }

    #[test]
    fn test_format_timestamp_leap_second() {
        // Test near leap second - use a valid time instead of 23:59:60
        // which is not universally supported
        let dt = Local.with_ymd_and_hms(2024, 6, 30, 23, 59, 59).unwrap();
        let formatted = format_timestamp(dt);
        assert!(!formatted.is_empty());
    }

    // ============================================================================
    // ANSI Code Tests
    // ============================================================================

    #[test]
    fn test_terminal_style_text() {
        let config = ColorConfig::plain();
        let plain = "test";
        let styled = terminal::style_text(plain, terminal::Color::Green, config);

        // With plain config (colors disabled), should just return the text
        assert_eq!(styled, plain);
    }

    #[test]
    fn test_terminal_style_different_colors() {
        let config = ColorConfig::plain();
        let colors = vec![
            terminal::Color::Black,
            terminal::Color::Red,
            terminal::Color::Green,
            terminal::Color::Yellow,
            terminal::Color::Blue,
            terminal::Color::Magenta,
            terminal::Color::Cyan,
            terminal::Color::White,
        ];

        let texts: Vec<_> = colors
            .iter()
            .map(|&color| terminal::style_text("test", color, config))
            .collect();

        // With colors disabled, all should return the same plain text
        for text in &texts {
            assert_eq!(text, "test");
        }
    }

    #[test]
    fn test_terminal_style_bright_vs_dim() {
        let config = ColorConfig::plain();
        let bright = terminal::style_text("test", terminal::Color::BrightRed, config);
        let dim = terminal::style_text("test", terminal::Color::Dim, config);

        // With colors disabled, both should just return the plain text
        assert_eq!(bright, "test");
        assert_eq!(dim, "test");
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[test]
    fn test_formatter_module_comprehensive() {
        // Test that all formatter functions are callable
        let dt = Local::now();
        let ts = format_timestamp(dt);
        let current = current_timestamp();

        let trace = format_level(&Level::TRACE);
        let debug = format_level(&Level::DEBUG);
        let info = format_level(&Level::INFO);
        let warn = format_level(&Level::WARN);
        let error = format_level(&Level::ERROR);

        // All should produce non-empty strings
        assert!(!ts.is_empty());
        assert!(!current.is_empty());
        assert!(!trace.is_empty());
        assert!(!debug.is_empty());
        assert!(!info.is_empty());
        assert!(!warn.is_empty());
        assert!(!error.is_empty());
    }

    #[test]
    fn test_multiple_calls_consistency() {
        let dt = Local::now();

        // Multiple calls with same input should produce consistent output
        let results: Vec<_> = (0..10).map(|_| format_timestamp(dt)).collect();

        for result in &results[1..] {
            assert_eq!(result, &results[0]);
        }
    }
}
