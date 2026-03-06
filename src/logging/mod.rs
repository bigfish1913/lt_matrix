//! Logging system for ltmatrix
//!
//! This module provides structured logging with multiple output formats and levels.
//!
//! # Features
//!
//! - **Multiple Log Levels**: TRACE, DEBUG, INFO, WARN, ERROR
//! - **Dual Output**: Console and file output simultaneously
//! - **Custom Formatters**: Colorized console output, plain text file output
//! - **Log Rotation**: Automatic rotation when log files exceed size limits
//! - **TRACE Level Support**: Special handling for capturing full Claude API calls
//!
//! # Usage
//!
//! ```no_run
//! use ltmatrix::logging::{init_logging, LogLevel};
//!
//! // Initialize with INFO level to console only
//! init_logging(LogLevel::Info, None::<&str>).expect("Failed to init logging");
//!
//! // Initialize with DEBUG level to both console and file
//! init_logging(LogLevel::Debug, Some("app.log")).expect("Failed to init logging");
//!
//! // Initialize with TRACE level for full API call capture
//! init_logging(LogLevel::Trace, Some("trace.log")).expect("Failed to init logging");
//! ```
//!
//! # Log Levels
//!
//! - **TRACE**: Extremely detailed logging, including full Claude API calls and responses
//! - **DEBUG**: Detailed information for debugging, task scheduling details, file changes
//! - **INFO**: General informational messages (default), task start/completion, progress summaries
//! - **WARN**: Warning messages for potentially harmful situations, retries, skipped tasks
//! - **ERROR**: Error messages for error events, failures, errors

pub mod file_manager;
pub mod formatter;
pub mod level;
pub mod logger;

// Test modules (only compiled when testing)
#[cfg(test)]
mod acceptance_tests;
#[cfg(test)]
mod formatter_tests;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod level_tests;
#[cfg(test)]
mod logger_tests;

// Re-export commonly used types and functions
pub use file_manager::{
    LogFileInfo, LogManager, DEFAULT_MAX_LOG_AGE_DAYS, DEFAULT_MAX_LOG_FILES,
    DEFAULT_MAX_LOG_SIZE_BYTES, LOGS_DIR,
};
pub use formatter::{current_timestamp, format_timestamp, TIMESTAMP_FORMAT};
pub use level::LogLevel;
pub use logger::{init_api_trace_logging, init_logging, init_logging_with_management};

/// Initializes the logging system with default settings
///
/// This is a convenience function that initializes logging with INFO level
/// and console output only.
///
/// # Example
///
/// ```no_run
/// use ltmatrix::logging::init_default_logging;
///
/// let _guard = init_default_logging().expect("Failed to initialize logging");
/// ```
pub fn init_default_logging() -> std::io::Result<logger::LogGuard> {
    init_logging(LogLevel::Info, None::<&str>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_default_logging() {
        // Use try_init() pattern to handle cases where dispatcher is already set
        let result = std::panic::catch_unwind(|| {
            let _ = init_default_logging();
        });
        // Should not panic
        assert!(result.is_ok());
    }
}
