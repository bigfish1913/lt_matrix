// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Progress reporting with color support
//!
//! This module provides progress reporting functionality with colorized output.

use crate::terminal::{self, ColorConfig};
use std::io::{self, Write};

/// Color configuration for progress reporting
#[derive(Debug, Clone, Copy)]
pub struct ReporterColorConfig {
    pub inner: ColorConfig,
}

impl ReporterColorConfig {
    /// Creates a new ReporterColorConfig that auto-detects terminal capabilities
    #[must_use]
    pub fn auto() -> Self {
        ReporterColorConfig {
            inner: ColorConfig::auto(),
        }
    }

    /// Creates a ReporterColorConfig with colors disabled
    #[must_use]
    pub fn plain() -> Self {
        ReporterColorConfig {
            inner: ColorConfig::plain(),
        }
    }

    /// Returns true if colors are enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }
}

impl Default for ReporterColorConfig {
    fn default() -> Self {
        Self::auto()
    }
}

/// Reports a task start with color
///
/// # Arguments
///
/// * `task_id` - The task identifier
/// * `task_title` - The task title
/// * `config` - Optional color configuration
pub fn report_task_start(task_id: &str, task_title: &str, config: Option<ReporterColorConfig>) {
    let color_config = config.unwrap_or_default();

    let task_id_colored = terminal::dim(task_id, color_config.inner);
    let title_colored = terminal::bold(task_title, color_config.inner);
    let start_colored = terminal::info("Starting", color_config.inner);

    println!(
        "{} {} task: {}",
        start_colored, task_id_colored, title_colored
    );
}

/// Reports a task completion with color
///
/// # Arguments
///
/// * `task_id` - The task identifier
/// * `task_title` - The task title
/// * `success` - Whether the task completed successfully
/// * `config` - Optional color configuration
pub fn report_task_complete(
    task_id: &str,
    task_title: &str,
    success: bool,
    config: Option<ReporterColorConfig>,
) {
    let color_config = config.unwrap_or_default();

    let task_id_colored = terminal::dim(task_id, color_config.inner);
    let title_colored = terminal::bold(task_title, color_config.inner);

    if success {
        let complete_colored = terminal::success("Completed", color_config.inner);
        println!(
            "{} {} task: {}",
            complete_colored, task_id_colored, title_colored
        );
    } else {
        let failed_colored = terminal::error("Failed", color_config.inner);
        println!(
            "{} {} task: {}",
            failed_colored, task_id_colored, title_colored
        );
    }
}

/// Reports a task failure with error message
///
/// # Arguments
///
/// * `task_id` - The task identifier
/// * `error_msg` - The error message
/// * `config` - Optional color configuration
pub fn report_task_error(task_id: &str, error_msg: &str, config: Option<ReporterColorConfig>) {
    let color_config = config.unwrap_or_default();

    let task_id_colored = terminal::dim(task_id, color_config.inner);
    let error_colored = terminal::error("Error", color_config.inner);
    let msg_colored = terminal::dim(error_msg, color_config.inner);

    println!(
        "{} in task {}: {}",
        error_colored, task_id_colored, msg_colored
    );
}

/// Reports a task retry
///
/// # Arguments
///
/// * `task_id` - The task identifier
/// * `retry_count` - Current retry number
/// * `max_retries` - Maximum number of retries
/// * `config` - Optional color configuration
pub fn report_task_retry(
    task_id: &str,
    retry_count: u32,
    max_retries: u32,
    config: Option<ReporterColorConfig>,
) {
    let color_config = config.unwrap_or_default();

    let task_id_colored = terminal::dim(task_id, color_config.inner);
    let retry_colored = terminal::warning("Retrying", color_config.inner);
    let retry_msg = format!("(attempt {}/{})", retry_count, max_retries);
    let retry_msg_colored = terminal::dim(&retry_msg, color_config.inner);

    println!(
        "{} {} task {}",
        retry_colored, task_id_colored, retry_msg_colored
    );
}

/// Reports a task being blocked
///
/// # Arguments
///
/// * `task_id` - The task identifier
/// * `reason` - Optional reason for being blocked
/// * `config` - Optional color configuration
pub fn report_task_blocked(
    task_id: &str,
    reason: Option<&str>,
    config: Option<ReporterColorConfig>,
) {
    let color_config = config.unwrap_or_default();

    let task_id_colored = terminal::dim(task_id, color_config.inner);
    let blocked_colored = terminal::style_text(
        "Blocked",
        terminal::Color::BrightMagenta,
        color_config.inner,
    );

    if let Some(reason_msg) = reason {
        let reason_colored = terminal::dim(reason_msg, color_config.inner);
        println!(
            "{} {} task: {}",
            blocked_colored, task_id_colored, reason_colored
        );
    } else {
        println!("{} {} task", blocked_colored, task_id_colored);
    }
}

/// Reports overall progress summary
///
/// # Arguments
///
/// * `completed` - Number of completed tasks
/// * `total` - Total number of tasks
/// * `failed` - Number of failed tasks
/// * `config` - Optional color configuration
pub fn report_progress_summary(
    completed: usize,
    total: usize,
    failed: usize,
    config: Option<ReporterColorConfig>,
) {
    let color_config = config.unwrap_or_default();

    let completed_colored =
        terminal::success(&format!("{} completed", completed), color_config.inner);
    let total_colored = terminal::dim(&format!("/ {} total", total), color_config.inner);

    if failed > 0 {
        let failed_colored = terminal::error(&format!(", {} failed", failed), color_config.inner);
        println!(
            "Progress: {}{}{}",
            completed_colored, total_colored, failed_colored
        );
    } else {
        println!("Progress: {}{}", completed_colored, total_colored);
    }
}

/// Reports a status update with color
///
/// # Arguments
///
/// * `status` - The status text
/// * `message` - Optional message
/// * `config` - Optional color configuration
pub fn report_status(status: &str, message: Option<&str>, config: Option<ReporterColorConfig>) {
    let color_config = config.unwrap_or_default();

    let status_colored = terminal::colorize_status(status, color_config.inner);

    if let Some(msg) = message {
        let msg_colored = terminal::dim(msg, color_config.inner);
        println!("{}: {}", status_colored, msg_colored);
    } else {
        println!("{}", status_colored);
    }
}

/// Flushes stdout to ensure progress is displayed
pub fn flush() {
    let _ = io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_task_start() {
        report_task_start("task-1", "Test task", None);
        flush();
    }

    #[test]
    fn test_report_task_complete_success() {
        report_task_complete("task-1", "Test task", true, None);
        flush();
    }

    #[test]
    fn test_report_task_complete_failure() {
        report_task_complete("task-1", "Test task", false, None);
        flush();
    }

    #[test]
    fn test_report_task_error() {
        report_task_error("task-1", "Test error", None);
        flush();
    }

    #[test]
    fn test_report_task_retry() {
        report_task_retry("task-1", 1, 3, None);
        flush();
    }

    #[test]
    fn test_report_task_blocked() {
        report_task_blocked("task-1", Some("Dependency failed"), None);
        flush();
    }

    #[test]
    fn test_report_progress_summary() {
        report_progress_summary(5, 10, 1, None);
        flush();
    }

    #[test]
    fn test_report_status() {
        report_status("pending", Some("Waiting for dependencies"), None);
        flush();
    }

    #[test]
    fn test_reporter_color_config_auto() {
        let config = ReporterColorConfig::auto();
        // Just verify it doesn't panic
        let _ = config.is_enabled();
    }

    #[test]
    fn test_reporter_color_config_plain() {
        let config = ReporterColorConfig::plain();
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_reporter_color_config_default() {
        let config = ReporterColorConfig::default();
        // Just verify it doesn't panic
        let _ = config.is_enabled();
    }

    #[test]
    fn test_report_with_plain_colors() {
        let config = ReporterColorConfig::plain();
        report_task_start("task-1", "Test task", Some(config));
        flush();
    }
}
