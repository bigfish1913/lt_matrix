// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Progress tracking with color support
//!
//! This module provides progress tracking functionality with colorized output.

use crate::terminal::{self, ColorConfig};
use ltmatrix_core::TaskStatus;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Color configuration for progress tracking
#[derive(Debug, Clone, Copy)]
pub struct TrackerColorConfig {
    pub inner: ColorConfig,
}

impl TrackerColorConfig {
    /// Creates a new TrackerColorConfig that auto-detects terminal capabilities
    #[must_use]
    pub fn auto() -> Self {
        TrackerColorConfig {
            inner: ColorConfig::auto(),
        }
    }

    /// Creates a TrackerColorConfig with colors disabled
    #[must_use]
    pub fn plain() -> Self {
        TrackerColorConfig {
            inner: ColorConfig::plain(),
        }
    }

    /// Returns true if colors are enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }
}

impl Default for TrackerColorConfig {
    fn default() -> Self {
        Self::auto()
    }
}

/// Tracks the progress of tasks
pub struct ProgressTracker {
    /// Task statuses by ID
    tasks: Arc<Mutex<HashMap<String, TaskStatus>>>,
    /// Color configuration
    color_config: TrackerColorConfig,
}

impl ProgressTracker {
    /// Creates a new ProgressTracker with the given color configuration
    ///
    /// # Arguments
    ///
    /// * `color_config` - Optional color configuration
    #[must_use]
    pub fn new(color_config: Option<TrackerColorConfig>) -> Self {
        ProgressTracker {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            color_config: color_config.unwrap_or_default(),
        }
    }

    /// Adds a task to track
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task identifier
    /// * `status` - The initial status
    pub fn add_task(&self, task_id: String, status: TaskStatus) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.insert(task_id, status);
    }

    /// Updates the status of a task
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task identifier
    /// * `status` - The new status
    pub fn update_task(&self, task_id: &str, status: TaskStatus) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.insert(task_id.to_string(), status);
    }

    /// Gets the current status of a task
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task identifier
    ///
    /// # Returns
    ///
    /// The task status, or None if not found
    #[must_use]
    pub fn get_status(&self, task_id: &str) -> Option<TaskStatus> {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(task_id).cloned()
    }

    /// Returns a formatted summary of all tasks with colors
    ///
    /// # Returns
    ///
    /// A string containing the formatted task summary
    #[must_use]
    pub fn format_summary(&self) -> String {
        let tasks = self.tasks.lock().unwrap();
        let config = self.color_config.inner;

        if tasks.is_empty() {
            return terminal::dim("No tasks tracked", config);
        }

        let mut summary = String::new();

        // Group tasks by status
        let mut pending = Vec::new();
        let mut in_progress = Vec::new();
        let mut completed = Vec::new();
        let mut failed = Vec::new();
        let mut blocked = Vec::new();
        let mut skipped = Vec::new();

        for (task_id, status) in tasks.iter() {
            match status {
                TaskStatus::Pending => pending.push(task_id),
                TaskStatus::InProgress => in_progress.push(task_id),
                TaskStatus::Completed => completed.push(task_id),
                TaskStatus::Failed => failed.push(task_id),
                TaskStatus::Blocked => blocked.push(task_id),
                TaskStatus::SkippedModeDisabled => skipped.push(task_id),
            }
        }

        // Build summary string
        if !completed.is_empty() {
            let status_colored = terminal::colorize_status("completed", config);
            let count_colored = terminal::success(&format!("({})", completed.len()), config);
            summary.push_str(&format!("{} {} ", status_colored, count_colored));
        }

        if !in_progress.is_empty() {
            let status_colored = terminal::colorize_status("in_progress", config);
            let count_colored = terminal::info(&format!("({})", in_progress.len()), config);
            summary.push_str(&format!("{} {} ", status_colored, count_colored));
        }
        if !pending.is_empty() {
            let status_colored = terminal::colorize_status("pending", config);
            let count_colored = terminal::warning(&format!("({})", pending.len()), config);
            summary.push_str(&format!("{} {} ", status_colored, count_colored));
        }
        if !failed.is_empty() {
            let status_colored = terminal::colorize_status("failed", config);
            let count_colored = terminal::error(&format!("({})", failed.len()), config);
            summary.push_str(&format!("{} {} ", status_colored, count_colored));
        }
        if !blocked.is_empty() {
            let status_colored = terminal::colorize_status("blocked", config);
            let count_colored = terminal::style_text(
                &format!("({})", blocked.len()),
                terminal::Color::BrightMagenta,
                config,
            );
            summary.push_str(&format!("{} {} ", status_colored, count_colored));
        }

        if !skipped.is_empty() {
            let status_colored = terminal::colorize_status("skipped", config);
            let count_colored = terminal::dim(&format!("({})", skipped.len()), config);
            summary.push_str(&format!("{} {} ", status_colored, count_colored));
        }

        summary
    }

    /// Prints the current progress summary to stdout
    pub fn print_summary(&self) {
        let summary = self.format_summary();
        if !summary.is_empty() {
            println!("{}", summary);
        }
    }

    /// Returns task statistics
    #[must_use]
    pub fn get_stats(&self) -> TaskStats {
        let tasks = self.tasks.lock().unwrap();

        let mut stats = TaskStats::default();

        for status in tasks.values() {
            match status {
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::InProgress => stats.in_progress += 1,
                TaskStatus::Completed => stats.completed += 1,
                TaskStatus::Failed => stats.failed += 1,
                TaskStatus::Blocked => stats.blocked += 1,
                TaskStatus::SkippedModeDisabled => stats.skipped += 1,
            }
        }

        stats.total = tasks.len();

        stats
    }
}

/// Task statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct TaskStats {
    /// Total number of tasks
    pub total: usize,
    /// Number of pending tasks
    pub pending: usize,
    /// Number of in-progress tasks
    pub in_progress: usize,
    /// Number of completed tasks
    pub completed: usize,
    /// Number of failed tasks
    pub failed: usize,
    /// Number of blocked tasks
    pub blocked: usize,
    /// Number of skipped tasks (mode disabled)
    pub skipped: usize,
}

impl TaskStats {
    /// Formats the stats as a colorized string
    ///
    /// # Arguments
    ///
    /// * `config` - The color configuration
    ///
    /// # Returns
    ///
    /// A formatted string with color
    #[must_use]
    pub fn format_colored(&self, config: ColorConfig) -> String {
        let mut parts = Vec::new();

        if self.completed > 0 {
            parts.push(terminal::success(
                &format!("{} completed", self.completed),
                config,
            ));
        }

        if self.in_progress > 0 {
            parts.push(terminal::info(
                &format!("{} in progress", self.in_progress),
                config,
            ));
        }

        if self.pending > 0 {
            parts.push(terminal::warning(
                &format!("{} pending", self.pending),
                config,
            ));
        }

        if self.failed > 0 {
            parts.push(terminal::error(&format!("{} failed", self.failed), config));
        }
        if self.blocked > 0 {
            parts.push(terminal::style_text(
                &format!("{} blocked", self.blocked),
                terminal::Color::BrightMagenta,
                config,
            ));
        }

        if self.skipped > 0 {
            parts.push(terminal::dim(&format!("{} skipped", self.skipped), config));
        }

        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_new() {
        let tracker = ProgressTracker::new(None);
        let stats = tracker.get_stats();
        assert_eq!(stats.total, 0);
    }

    #[test]
    fn test_tracker_add_task() {
        let tracker = ProgressTracker::new(None);
        tracker.add_task("task-1".to_string(), TaskStatus::Pending);

        let status = tracker.get_status("task-1");
        assert_eq!(status, Some(TaskStatus::Pending));
    }

    #[test]
    fn test_tracker_update_task() {
        let tracker = ProgressTracker::new(None);
        tracker.add_task("task-1".to_string(), TaskStatus::Pending);
        tracker.update_task("task-1", TaskStatus::Completed);
        let status = tracker.get_status("task-1");
        assert_eq!(status, Some(TaskStatus::Completed));
    }

    #[test]
    fn test_tracker_get_stats() {
        let tracker = ProgressTracker::new(None);
        tracker.add_task("task-1".to_string(), TaskStatus::Pending);
        tracker.add_task("task-2".to_string(), TaskStatus::InProgress);
        tracker.add_task("task-3".to_string(), TaskStatus::Completed);

        let stats = tracker.get_stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.completed, 1);
    }

    #[test]
    fn test_tracker_format_summary() {
        let tracker = ProgressTracker::new(None);
        tracker.add_task("task-1".to_string(), TaskStatus::Completed);
        tracker.add_task("task-2".to_string(), TaskStatus::Pending);
        let summary = tracker.format_summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("completed") || summary.contains("pending"));
    }

    #[test]
    fn test_task_stats_format_colored() {
        let config = ColorConfig::plain();
        let stats = TaskStats {
            total: 4,
            completed: 1,
            in_progress: 1,
            pending: 1,
            failed: 0,
            blocked: 0,
            skipped: 1,
        };

        let formatted = stats.format_colored(config);
        assert!(formatted.contains("completed"));
        assert!(formatted.contains("in progress"));
        assert!(formatted.contains("pending"));
        assert!(formatted.contains("skipped"));
    }

    #[test]
    fn test_tracker_color_config_auto() {
        let config = TrackerColorConfig::auto();
        // Just verify it doesn't panic
        let _ = config.is_enabled();
    }

    #[test]
    fn test_tracker_color_config_plain() {
        let config = TrackerColorConfig::plain();
        assert!(!config.is_enabled());
    }
    #[test]
    fn test_tracker_color_config_default() {
        let config = TrackerColorConfig::default();
        // Just verify it doesn't panic
        let _ = config.is_enabled();
    }
}
