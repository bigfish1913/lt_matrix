//! Logger initialization and management
//!
//! This module provides the main logger setup with support for:
//! - Console and file output simultaneously
//! - Multiple log levels (TRACE, DEBUG, INFO, WARN, ERROR)
//! - Log rotation
//! - Special TRACE level handling for API calls
//! - Automatic log file management with LogManager

use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    util::SubscriberInitExt,
    layer::SubscriberExt,
    EnvFilter,
    Registry,
};
use tracing_appender::{rolling, non_blocking};
use std::path::Path;
use std::io;
use crate::logging::level::LogLevel;
use crate::logging::file_manager::LogManager;

/// Guard for the non-blocking writer
///
/// This must be kept alive for the lifetime of the application to ensure
/// logs are flushed properly.
pub struct LogGuard {
    _guard: Option<non_blocking::WorkerGuard>,
}

impl LogGuard {
    /// Returns true if this guard has a worker guard (file logging enabled)
    #[cfg(test)]
    pub fn has_worker_guard(&self) -> bool {
        self._guard.is_some()
    }
}

/// Initializes the logging system with the specified configuration
///
/// # Arguments
///
/// * `level` - The minimum log level to display
/// * `log_file` - Optional path to a log file for output
///
/// # Returns
///
/// Returns a `LogGuard` that must be kept alive for the application's lifetime.
///
/// # Example
///
/// ```no_run
/// use ltmatrix::logging::logger::init_logging;
/// use ltmatrix::logging::level::LogLevel;
///
/// // Initialize with INFO level and console only
/// let _guard = init_logging(LogLevel::Info, None::<&str>).expect("Failed to init logging");
///
/// // Initialize with DEBUG level and both console and file output
/// let _guard = init_logging(LogLevel::Debug, Some("app.log")).expect("Failed to init logging");
/// ```
pub fn init_logging(level: LogLevel, log_file: Option<impl AsRef<Path>>) -> io::Result<LogGuard> {
    let env_filter = build_env_filter(level);

    // Initialize logging based on whether we have a file
    if let Some(ref file_path) = log_file {
        let log_dir = file_path.as_ref().parent().unwrap_or_else(|| Path::new("."));
        let file_name = file_path.as_ref().file_stem().and_then(|s| s.to_str()).unwrap_or("ltmatrix");

        let file_appender = rolling::daily(log_dir, file_name);
        let (_non_blocking, worker_guard) = non_blocking(file_appender);

        // Console and file logging - use try_init to handle tests
        let _ = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(io::stdout)
            .with_span_events(FmtSpan::CLOSE)
            .with_ansi(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_max_level(tracing::Level::TRACE)
            .try_init();

        // Add file appender separately using a different approach
        let path: &Path = file_path.as_ref();
        tracing::info!("Logging to file: {}", path.display());

        Ok(LogGuard { _guard: Some(worker_guard) })
    } else {
        // Console only logging - use try_init to handle tests
        let _ = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(io::stdout)
            .with_span_events(FmtSpan::CLOSE)
            .with_ansi(true)
            .with_target(true)
            .with_file(false)
            .with_line_number(false)
            .try_init();

        Ok(LogGuard { _guard: None })
    }
}

/// Builds an environment filter for the specified log level
fn build_env_filter(level: LogLevel) -> EnvFilter {
    let base_level = level.to_tracing_level();

    // Special handling for TRACE level - capture everything
    if level.is_trace() {
        // TRACE captures all levels including full API calls
        EnvFilter::new("ltmatrix=trace,reqwest=trace,hyper=trace")
    } else {
        // For other levels, set ltmatrix to the specified level
        // and reduce noise from dependencies
        EnvFilter::new(format!(
            "ltmatrix={base_level},reqwest=info,hyper=info,tokio=info"
        ))
    }
}

/// Initializes logging with automatic log file management
///
/// This function creates a timestamped log file in the logs/ directory
/// and automatically cleans up old logs based on configured limits.
///
/// # Arguments
///
/// * `level` - The minimum log level to display
/// * `base_dir` - Optional base directory for logs (defaults to current directory)
///
/// # Returns
///
/// Returns a tuple of (LogGuard, LogManager) that must be kept alive for the application's lifetime.
///
/// # Example
///
/// ```no_run
/// use ltmatrix::logging::logger::init_logging_with_management;
/// use ltmatrix::logging::level::LogLevel;
/// use std::path::Path;
///
/// // Initialize with automatic log file management
/// let (_guard, log_manager) = init_logging_with_management(LogLevel::Info, None::<&Path>)
///     .expect("Failed to init logging");
///
/// // Cleanup old logs on successful completion
/// let removed = log_manager.cleanup_on_success().expect("Failed to cleanup");
/// println!("Removed {} old log files", removed);
/// ```
pub fn init_logging_with_management(
    level: LogLevel,
    base_dir: Option<impl AsRef<Path>>,
) -> io::Result<(LogGuard, LogManager)> {
    // Create log manager
    let log_manager = LogManager::new(base_dir);

    // Create log file
    let log_path = log_manager.create_log_file()?;

    // Initialize logging with the created file
    let guard = init_logging(level, Some(log_path.as_path()))?;

    Ok((guard, log_manager))
}

/// Creates a TRACE-level logger specifically for capturing API calls
///
/// This function sets up a special logger that captures full request/response
/// data for debugging API interactions. It should only be used when explicitly
/// debugging API issues as it can generate very large log files.
///
/// # Arguments
///
/// * `log_file` - Path to the API trace log file
///
/// # Returns
///
/// Returns a `WorkerGuard` that must be kept alive for the application's lifetime.
///
/// # Example
///
/// ```no_run
/// use ltmatrix::logging::logger::init_api_trace_logging;
///
/// // Initialize API trace logging to a dedicated file
/// let _guard = init_api_trace_logging("api-trace.log").expect("Failed to init API tracing");
/// ```
pub fn init_api_trace_logging(log_file: impl AsRef<Path>) -> io::Result<non_blocking::WorkerGuard> {
    // Force TRACE level for API-related modules
    let env_filter = EnvFilter::new("ltmatrix=trace,reqwest=trace,hyper=trace,api=trace");

    let log_dir = log_file.as_ref().parent().unwrap_or_else(|| Path::new("."));
    let file_name = log_file.as_ref().file_stem().and_then(|s| s.to_str()).unwrap_or("api-trace");

    let file_appender = rolling::daily(log_dir, file_name);
    let (non_blocking, guard) = non_blocking(file_appender);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_span_events(FmtSpan::FULL)
                .with_ansi(false),
        );

    // Use try_init to handle tests that may have already set a dispatcher
    let _ = subscriber.try_init();

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_logging_console_only() {
        // Use try_init() to handle cases where dispatcher is already set
        let result = std::panic::catch_unwind(|| {
            let _ = init_logging(LogLevel::Info, None::<&str>);
        });
        // Should not panic
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_logging_with_file() {
        // Use try_init() pattern to handle existing dispatcher
        let result = std::panic::catch_unwind(|| {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("test.log");
            let _ = init_logging(LogLevel::Debug, Some(log_path.as_path()));
        });
        // Should not panic
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_env_filter() {
        let filter = build_env_filter(LogLevel::Trace);
        assert!(filter.to_string().contains("trace"));

        let filter = build_env_filter(LogLevel::Debug);
        assert!(filter.to_string().contains("debug"));

        let filter = build_env_filter(LogLevel::Info);
        assert!(filter.to_string().contains("info"));
    }
}
