// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Progress tracking and display
//!
//! This module provides real-time progress tracking, progress bars, and ETA estimation
//! with full terminal color support.

pub mod bar;
pub mod eta;
pub mod manager;
pub mod reporter;
pub mod tracker;

// Re-export commonly used types and functions
pub use bar::{
    colorize_percentage, create_custom_progress_bar, create_progress_bar, create_spinner,
    BarColorConfig,
};
pub use eta::{EtaCalculator, HistoricalData, MetricsCollector, format_eta};
pub use manager::{ProgressBarType, ProgressManager, ProgressManagerConfig};
pub use reporter::{
    flush, report_progress_summary, report_status, report_task_blocked, report_task_complete,
    report_task_error, report_task_retry, report_task_start, ReporterColorConfig,
};
pub use tracker::{ProgressTracker, TaskStats, TrackerColorConfig};
