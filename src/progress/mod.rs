//! Progress tracking and display
//!
//! This module provides real-time progress tracking, progress bars, and ETA estimation
//! with full terminal color support.

pub mod tracker;
pub mod bar;
pub mod reporter;

// Re-export commonly used types and functions
pub use tracker::{ProgressTracker, TaskStats, TrackerColorConfig};
pub use bar::{create_progress_bar, create_custom_progress_bar, create_spinner, BarColorConfig, colorize_percentage};
pub use reporter::{
    report_task_start, report_task_complete, report_task_error, report_task_retry,
    report_task_blocked, report_progress_summary, report_status, flush, ReporterColorConfig,
};
