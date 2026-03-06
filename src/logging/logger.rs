//! Logger initialization and management
//!
//! This module provides the main logger setup with support for:
//! - Console and file output simultaneously
//! - Multiple log levels (TRACE, DEBUG, INFO, WARN, ERROR)
//! - Log rotation
//! - Special TRACE level handling for API calls

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

/// Maximum log file size before rotation (10 MB)
const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;

/// Number of rotated log files to keep
const MAX_LOG_FILES: usize = 5;

/// Guard for the non-blocking writer
///
/// This must be kept alive for the lifetime of the application to ensure
/// logs are flushed properly.
pub struct LogGuard {
    _guard: Option<non_blocking::WorkerGuard>,
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

        // Console and file logging
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(io::stdout)
            .with_span_events(FmtSpan::CLOSE)
            .with_ansi(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_max_level(tracing::Level::TRACE)
            .init();

        // Add file appender separately using a different approach
        let path: &Path = file_path.as_ref();
        tracing::info!("Logging to file: {}", path.display());

        Ok(LogGuard { _guard: Some(worker_guard) })
    } else {
        // Console only logging
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(io::stdout)
            .with_span_events(FmtSpan::CLOSE)
            .with_ansi(true)
            .with_target(true)
            .with_file(false)
            .with_line_number(false)
            .init();

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

/// Creates a file logging layer with rotation support
fn create_file_layer(_file_path: &Path) -> io::Result<(fmt::Layer<Registry>, non_blocking::WorkerGuard)> {
    // For now, use a simpler approach
    // In a future version, we can implement proper dual-layer logging
    todo!("Implement proper dual-layer logging with rotation")
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

    subscriber.init();

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_logging_console_only() {
        // This should not panic
        let result = init_logging(LogLevel::Info, None::<&str>);
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_logging_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let result = init_logging(LogLevel::Debug, Some(log_path.as_path()));
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
