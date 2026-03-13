// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Comprehensive tests for LogLevel implementation
//!
//! These tests verify:
//! - All log levels (TRACE, DEBUG, INFO, WARN, ERROR)
//! - Level ordering and comparison
//! - String parsing and display
//! - Conversion to tracing::Level
//! - Default values

use crate::logging::level::LogLevel;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Level Ordering and Comparison Tests
    // ============================================================================

    #[test]
    fn test_level_complete_ordering() {
        // Verify complete ordering chain
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Trace < LogLevel::Info);
        assert!(LogLevel::Trace < LogLevel::Warn);
        assert!(LogLevel::Trace < LogLevel::Error);

        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Debug < LogLevel::Warn);
        assert!(LogLevel::Debug < LogLevel::Error);

        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Info < LogLevel::Error);

        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_level_equality() {
        assert_eq!(LogLevel::Trace, LogLevel::Trace);
        assert_eq!(LogLevel::Debug, LogLevel::Debug);
        assert_eq!(LogLevel::Info, LogLevel::Info);
        assert_eq!(LogLevel::Warn, LogLevel::Warn);
        assert_eq!(LogLevel::Error, LogLevel::Error);
    }

    #[test]
    fn test_level_partial_ord() {
        // Test partial ordering
        assert!(LogLevel::Trace <= LogLevel::Debug);
        assert!(LogLevel::Debug <= LogLevel::Info);
        assert!(LogLevel::Info <= LogLevel::Warn);
        assert!(LogLevel::Warn <= LogLevel::Error);

        // Test equality in partial ordering
        assert!(LogLevel::Info <= LogLevel::Info);
        assert!(LogLevel::Info >= LogLevel::Info);
    }

    // ============================================================================
    // String Parsing Tests
    // ============================================================================

    #[test]
    fn test_from_str_case_insensitive() {
        // Test all valid case variations
        let test_cases = vec![
            ("trace", LogLevel::Trace),
            ("TRACE", LogLevel::Trace),
            ("Trace", LogLevel::Trace),
            ("tRaCe", LogLevel::Trace),
            ("debug", LogLevel::Debug),
            ("DEBUG", LogLevel::Debug),
            ("Debug", LogLevel::Debug),
            ("info", LogLevel::Info),
            ("INFO", LogLevel::Info),
            ("Info", LogLevel::Info),
            ("warn", LogLevel::Warn),
            ("WARN", LogLevel::Warn),
            ("Warn", LogLevel::Warn),
            ("error", LogLevel::Error),
            ("ERROR", LogLevel::Error),
            ("Error", LogLevel::Error),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                LogLevel::from_str(input).unwrap(),
                expected,
                "Failed to parse: {}",
                input
            );
        }
    }

    #[test]
    fn test_from_str_warning_alias() {
        // Test that "warning" maps to WARN
        assert_eq!(LogLevel::from_str("warning").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("WARNING").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("Warning").unwrap(), LogLevel::Warn);
    }

    #[test]
    fn test_from_str_invalid_inputs() {
        let invalid_inputs = vec![
            "invalid",
            "infoo",
            "tracee",
            "",
            " ",
            "infodebug",
            "TRACEEXTRA",
            "level1",
            "1",
            "trace debug",
        ];

        for input in invalid_inputs {
            let result = LogLevel::from_str(input);
            assert!(
                result.is_err(),
                "Expected error for invalid input: '{}'",
                input
            );

            if let Err(e) = result {
                assert!(
                    e.contains("Invalid log level"),
                    "Error message should mention invalid log level"
                );
                assert!(e.contains(input), "Error should contain the invalid input");
            }
        }
    }

    #[test]
    fn test_from_str_error_message_format() {
        let result = LogLevel::from_str("invalid_level");
        match result {
            Ok(_) => panic!("Expected error for invalid log level"),
            Err(e) => {
                // Verify error message contains helpful information
                assert!(e.contains("invalid_level"));
                assert!(e.contains("trace") || e.contains("debug") || e.contains("info"));
            }
        }
    }

    // ============================================================================
    // Display and String Representation Tests
    // ============================================================================

    #[test]
    fn test_display_uppercase() {
        // All log levels should display in uppercase
        assert_eq!(format!("{}", LogLevel::Trace), "TRACE");
        assert_eq!(format!("{}", LogLevel::Debug), "DEBUG");
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Warn), "WARN");
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
    }

    #[test]
    fn test_as_str_static() {
        // Verify as_str returns static strings with correct values
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");

        // Verify it's a static string (has 'static lifetime)
        let _s: &'static str = LogLevel::Info.as_str();
    }

    #[test]
    fn test_display_matches_as_str() {
        // Display and as_str should return the same values
        for level in [
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ] {
            assert_eq!(format!("{}", level), level.as_str());
        }
    }

    // ============================================================================
    // Tracing Level Conversion Tests
    // ============================================================================

    #[test]
    fn test_to_tracing_level_all_levels() {
        use tracing::Level;

        assert_eq!(LogLevel::Trace.to_tracing_level(), Level::TRACE);
        assert_eq!(LogLevel::Debug.to_tracing_level(), Level::DEBUG);
        assert_eq!(LogLevel::Info.to_tracing_level(), Level::INFO);
        assert_eq!(LogLevel::Warn.to_tracing_level(), Level::WARN);
        assert_eq!(LogLevel::Error.to_tracing_level(), Level::ERROR);
    }

    #[test]
    fn test_from_tracing_level_roundtrip() {
        use tracing::Level;

        let test_cases = vec![
            (Level::TRACE, LogLevel::Trace),
            (Level::DEBUG, LogLevel::Debug),
            (Level::INFO, LogLevel::Info),
            (Level::WARN, LogLevel::Warn),
            (Level::ERROR, LogLevel::Error),
        ];

        for (tracing_level, expected) in test_cases {
            let log_level = LogLevel::from(tracing_level);
            assert_eq!(log_level, expected);
            // Test roundtrip conversion
            assert_eq!(log_level.to_tracing_level(), tracing_level);
        }
    }

    // ============================================================================
    // Special Method Tests
    // ============================================================================

    #[test]
    fn test_is_trace() {
        assert!(LogLevel::Trace.is_trace());
        assert!(!LogLevel::Debug.is_trace());
        assert!(!LogLevel::Info.is_trace());
        assert!(!LogLevel::Warn.is_trace());
        assert!(!LogLevel::Error.is_trace());
    }

    #[test]
    fn test_default_level() {
        assert_eq!(LogLevel::default(), LogLevel::Info);
    }

    #[test]
    fn test_const_methods() {
        // Verify const methods can be used in const contexts
        const DEFAULT_LEVEL: LogLevel = LogLevel::default();
        const IS_TRACE: bool = LogLevel::Trace.is_trace();
        const TRACE_LEVEL: tracing::Level = LogLevel::Trace.to_tracing_level();

        assert_eq!(DEFAULT_LEVEL, LogLevel::Info);
        assert!(IS_TRACE);
        assert_eq!(TRACE_LEVEL, tracing::Level::TRACE);
    }

    // ============================================================================
    // Clone and Copy Tests
    // ============================================================================

    #[test]
    fn test_level_is_copy() {
        // LogLevel should be Copy (can be copied without moving)
        let level = LogLevel::Info;
        let copied = level; // This should move if not Copy
        let _still_valid = level; // This should fail if level was moved

        assert_eq!(copied, LogLevel::Info);
        assert_eq!(_still_valid, LogLevel::Info);
    }

    #[test]
    fn test_level_is_clone() {
        // LogLevel should be Clone
        let level = LogLevel::Debug;
        let cloned = level.clone();

        assert_eq!(level, cloned);
    }

    // ============================================================================
    // Debug Representation Tests
    // ============================================================================

    #[test]
    fn test_debug_formatting() {
        // Debug output should contain the level name
        let trace_debug = format!("{:?}", LogLevel::Trace);
        let info_debug = format!("{:?}", LogLevel::Info);

        assert!(trace_debug.contains("Trace"));
        assert!(info_debug.contains("Info"));
    }

    // ============================================================================
    // Edge Cases and Comprehensive Tests
    // ============================================================================

    #[test]
    fn test_all_level_values_unique() {
        // Ensure all levels have unique values
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
                    assert_ne!(
                        level1, level2,
                        "Levels at indices {} and {} are equal",
                        i, j
                    );
                }
            }
        }
    }

    #[test]
    fn test_level_iterating() {
        // Test that we can use levels in collections and iteration
        let levels = vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ];

        let mut count = 0;
        for level in &levels {
            match level {
                LogLevel::Trace => count += 1,
                LogLevel::Debug => count += 1,
                LogLevel::Info => count += 1,
                LogLevel::Warn => count += 1,
                LogLevel::Error => count += 1,
            }
        }

        assert_eq!(count, 5);
    }

    #[test]
    fn test_level_comparison_operators() {
        use std::cmp::Ordering;

        assert_eq!(LogLevel::Trace.cmp(&LogLevel::Debug), Ordering::Less);
        assert_eq!(LogLevel::Debug.cmp(&LogLevel::Trace), Ordering::Greater);
        assert_eq!(LogLevel::Info.cmp(&LogLevel::Info), Ordering::Equal);
    }

    #[test]
    fn test_level_max_min() {
        use std::cmp::{max, min};

        assert_eq!(min(LogLevel::Debug, LogLevel::Error), LogLevel::Debug);
        assert_eq!(max(LogLevel::Debug, LogLevel::Error), LogLevel::Error);
        assert_eq!(min(LogLevel::Trace, LogLevel::Info), LogLevel::Trace);
        assert_eq!(max(LogLevel::Trace, LogLevel::Info), LogLevel::Info);
    }
}
